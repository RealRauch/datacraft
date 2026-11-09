//! Report band grouping and aggregation (step 15.2, item H41).
//!
//! The pagination engine of the print preview. With `ReportDef.group_by`
//! set, rows with equal group values form groups (encounter order —
//! sorting is the data's responsibility) and the page sequence becomes
//! group header band → one detail page per record → group footer band.
//!
//! Expression controls are text boxes whose bound field starts with `=`
//! (the Access `ControlSource` convention): detail pages evaluate per
//! record (aggregates see exactly that record); footer bands evaluate
//! their top-level `Sum/Count/Min/Max/Avg` over the group's rows
//! (`Count(*)` counts rows, the others skip `Null`s), anything else
//! evaluates against the group's first record. On the empty page of a
//! record-less source, expressions render empty.

use crate::expr::{self};
use crate::form::ControlKind;
use crate::report::ReportDef;
use crate::report_aggregates::{aggregate_head, run_aggregate, scope_of};
use crate::report_preview::{Band, PreviewItem, PreviewPage};
use crate::schema::{FieldValue, TableDef};

/// Paginates a report over its rows (the preview entry point).
pub(crate) fn paginate(
    report: &ReportDef,
    source: Option<&TableDef>,
    rows: &[Vec<FieldValue>],
) -> Result<Vec<PreviewPage>, String> {
    match &report.group_by {
        Some(group_field) => banded(report, source, rows, group_field),
        None => ungrouped(report, source, rows),
    }
}

fn ungrouped(
    report: &ReportDef,
    source: Option<&TableDef>,
    rows: &[Vec<FieldValue>],
) -> Result<Vec<PreviewPage>, String> {
    let page_count = rows.len().max(1);
    (0..page_count)
        .map(|index| {
            let row = rows.get(index);
            let one;
            let aggregate_rows: &[&Vec<FieldValue>] = match row {
                Some(row) => {
                    one = [row];
                    &one
                }
                None => &[],
            };
            let mut items = Vec::new();
            for control in &report.controls {
                if let Some(text) = render_control(control, source, row, aggregate_rows)? {
                    items.push(PreviewItem {
                        text,
                        rect: control.rect,
                    });
                }
            }
            Ok(PreviewPage {
                band: Band::Detail,
                items,
            })
        })
        .collect()
}

fn banded(
    report: &ReportDef,
    source: Option<&TableDef>,
    rows: &[Vec<FieldValue>],
    group_field: &str,
) -> Result<Vec<PreviewPage>, String> {
    let column = group_column(source, group_field)?;
    let mut pages = Vec::new();
    let mut group: Vec<&Vec<FieldValue>> = Vec::new();
    for row in rows {
        let key = key_of(row, column);
        let open_key = group.first().map(|first| key_of(first, column));
        if !group.is_empty() && Some(key.clone()) != open_key {
            flush(report, source, &mut group, &mut pages)?;
        }
        group.push(row);
    }
    flush(report, source, &mut group, &mut pages)?;
    Ok(pages)
}

fn flush(
    report: &ReportDef,
    source: Option<&TableDef>,
    group: &mut Vec<&Vec<FieldValue>>,
    pages: &mut Vec<PreviewPage>,
) -> Result<(), String> {
    if group.is_empty() {
        return Ok(());
    }
    let key = key_of(
        group[0],
        group_column(source, report.group_by.as_deref().expect("banded"))?,
    );
    pages.push(header_page(report, &key));
    for row in group.iter() {
        let one = [*row];
        let mut items = Vec::new();
        for control in &report.controls {
            if let Some(text) = render_control(control, source, Some(*row), &one)? {
                items.push(PreviewItem {
                    text,
                    rect: control.rect,
                });
            }
        }
        pages.push(PreviewPage {
            band: Band::Detail,
            items,
        });
    }
    // The footer band carries literals and aggregate expressions only —
    // plain bound fields are detail-band content.
    let mut items = Vec::new();
    for control in &report.controls {
        let footer_relevant = match &control.kind {
            ControlKind::Label { .. } | ControlKind::Button { .. } => true,
            ControlKind::TextBox {
                bound_field: Some(field),
            }
            | ControlKind::CheckBox {
                bound_field: Some(field),
            }
            | ControlKind::ComboBox {
                bound_field: Some(field),
                ..
            } => field.starts_with('='),
            _ => false,
        };
        if !footer_relevant {
            continue;
        }
        if let Some(text) =
            render_control(control, source, group.first().copied(), group.as_slice())?
        {
            items.push(PreviewItem {
                text,
                rect: control.rect,
            });
        }
    }
    pages.push(PreviewPage {
        band: Band::GroupFooter,
        items,
    });
    group.clear();
    Ok(())
}

fn key_of(row: &[FieldValue], column: usize) -> String {
    row.get(column)
        .map(crate::present::display_value)
        .unwrap_or_default()
}

fn group_column(source: Option<&TableDef>, field: &str) -> Result<usize, String> {
    let Some(source) = source else {
        return Err(format!(
            "grouping needs a source table — field {field:?} cannot resolve"
        ));
    };
    source
        .fields
        .iter()
        .position(|candidate| candidate.name.eq_ignore_ascii_case(field))
        .ok_or_else(|| format!("group field {field:?} not found in {:?}", source.name))
}

fn header_page(report: &ReportDef, key: &str) -> PreviewPage {
    let page = report.page;
    let rect = crate::canvas::Rect {
        x: page.margin_left,
        y: page.margin_top,
        width: page.printable_width(),
        height: 18.0,
    };
    PreviewPage {
        band: Band::GroupHeader,
        items: vec![PreviewItem {
            text: key.to_owned(),
            rect,
        }],
    }
}

/// Renders one control for the current page: literals, bound fields and
/// `=expressions`. `aggregate_rows` is the record slice the expression's
/// aggregates see (the record itself on detail pages, the whole group in
/// the footer); a `None` row (empty source) renders expressions as empty.
fn render_control(
    control: &crate::form::ControlDef,
    source: Option<&TableDef>,
    row: Option<&Vec<FieldValue>>,
    aggregate_rows: &[&Vec<FieldValue>],
) -> Result<Option<String>, String> {
    let text = match &control.kind {
        ControlKind::Label { text } => Some(text.clone()),
        ControlKind::Button { caption } => Some(caption.clone()),
        ControlKind::TextBox {
            bound_field: Some(field),
        }
        | ControlKind::CheckBox {
            bound_field: Some(field),
        }
        | ControlKind::ComboBox {
            bound_field: Some(field),
            ..
        } if field.starts_with('=') => {
            return expression_text(control, &field[1..], source, row, aggregate_rows).map(Some);
        }
        ControlKind::TextBox {
            bound_field: Some(field),
        }
        | ControlKind::CheckBox {
            bound_field: Some(field),
        }
        | ControlKind::ComboBox {
            bound_field: Some(field),
            ..
        } => Some(crate::report_preview::bound_text(source, row, field)),
        _ => None,
    };
    Ok(text)
}

fn expression_text(
    control: &crate::form::ControlDef,
    expression: &str,
    source: Option<&TableDef>,
    row: Option<&Vec<FieldValue>>,
    aggregate_rows: &[&Vec<FieldValue>],
) -> Result<String, String> {
    let Some(row) = row else {
        return Ok(String::new());
    };
    if let Some((aggregate, argument)) = aggregate_head(expression) {
        let value = run_aggregate(aggregate, &argument, source, aggregate_rows)?;
        return Ok(crate::present::display_value(&value));
    }
    let scope = scope_of(source, row);
    let value = expr::parse(expression)
        .and_then(|parsed| parsed.evaluate(&scope))
        .map_err(|e| format!("control {:?}: {e}", control.name))?;
    Ok(crate::present::display_value(&value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::form::ControlDef;
    use crate::schema::{FieldDef, FieldKind, NumericKind};

    fn table() -> TableDef {
        TableDef::new(
            "T",
            vec![
                FieldDef::new("region", FieldKind::Text { max_len: 10 }),
                FieldDef::new("qty", FieldKind::Number(NumericKind::LongInteger)),
                FieldDef::new("price", FieldKind::Currency),
            ],
        )
    }

    fn rows() -> Vec<Vec<FieldValue>> {
        let row = |region: &str, qty: i64, price: i64| {
            vec![
                FieldValue::Text(region.into()),
                FieldValue::Integer(qty),
                FieldValue::Currency(price),
            ]
        };
        vec![
            row("West", 2, 10_000),
            row("West", 1, 20_000),
            row("East", 4, 5_000),
        ]
    }

    fn report(group_by: Option<&str>, controls: Vec<ControlDef>) -> ReportDef {
        let mut def = ReportDef::new("r");
        def.source = "T".into();
        def.group_by = group_by.map(str::to_owned);
        def.controls = controls;
        def
    }

    fn textbox(name: &str, bound: &str) -> ControlDef {
        ControlDef {
            name: name.into(),
            kind: ControlKind::TextBox {
                bound_field: Some(bound.into()),
            },
            rect: crate::canvas::Rect {
                x: 36.0,
                y: 700.0,
                width: 100.0,
                height: 20.0,
            },
        }
    }

    fn describe(pages: &[PreviewPage]) -> Vec<String> {
        pages
            .iter()
            .map(|page| {
                format!(
                    "{:?}:{}",
                    page.band,
                    page.items
                        .iter()
                        .map(|item| item.text.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                )
            })
            .collect()
    }

    #[test]
    fn bands_render_header_details_and_footer() {
        let def = report(
            Some("region"),
            vec![textbox("q", "qty"), textbox("s", "=Sum([price])")],
        );
        let pages = paginate(&def, Some(&table()), &rows()).expect("renders");
        assert_eq!(
            describe(&pages),
            vec![
                "GroupHeader:West",
                // Detail aggregates see the record only.
                "Detail:2,1",
                "Detail:1,2",
                "GroupFooter:3",
                "GroupHeader:East",
                "Detail:4,0.5",
                "GroupFooter:0.5",
            ]
        );
    }

    #[test]
    fn footer_text_evaluates_non_aggregates_against_the_first_record() {
        let data = rows();
        let def = report(Some("region"), vec![textbox("t", "=[region] & \" total\"")]);
        let pages = paginate(&def, Some(&table()), &data).expect("renders");
        // The West group closes before East arrives; its footer carries
        // the first record's region.
        let footer = &pages[3];
        assert_eq!(footer.band, Band::GroupFooter);
        assert_eq!(footer.items[0].text, "West total");
        assert_eq!(
            &pages.last().expect("east footer").items[0].text,
            "East total"
        );
    }

    #[test]
    fn ungrouped_previews_stay_detail_pages_with_row_scoped_aggregates() {
        let def = report(None, vec![textbox("s", "=Sum([price])")]);
        let pages = paginate(&def, Some(&table()), &rows()).expect("renders");
        assert_eq!(describe(&pages), vec!["Detail:1", "Detail:2", "Detail:0.5"]);
    }

    #[test]
    fn empty_sources_render_one_empty_detail_page() {
        let def = report(None, vec![textbox("s", "=Sum([price])")]);
        let pages = paginate(&def, Some(&table()), &[]).expect("renders");
        assert_eq!(describe(&pages), vec!["Detail:"]);
    }
}
