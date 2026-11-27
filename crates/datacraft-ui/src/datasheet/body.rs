//! The datasheet's rendered row body (phase 101, steps 101.4/101.5; item
//! M132) — split from `datasheet.rs` for the file-size gate.
//!
//! Every cell is a string in display-column order: the column permutation
//! applies here once, and a `Null` cell carries the localized marker so an
//! empty value is distinguishable from an empty string.

use datacraft_core::edit_buffer::EditBuffer;
use datacraft_core::schema::{FieldKind, FieldValue};

use crate::i18n::t;
use crate::labels::value_label_for;

/// One row's cells in display order. The column kinds (storage order)
/// feed the rendering: a decimal keeps its declared scale (M135).
fn cells(order: &[usize], values: &[FieldValue], kinds: &[FieldKind]) -> Vec<String> {
    order
        .iter()
        .map(|&storage| match values.get(storage) {
            Some(FieldValue::Null) | None => t("(NULL)").to_owned(),
            Some(value) => value_label_for(value, kinds.get(storage)),
        })
        .collect()
}

/// The rendered body: staged rows render their pending image, inserts
/// append after them, and a windowed tab simply passes its rows.
pub(crate) fn build_body(
    order: &[usize],
    rows: &[Vec<FieldValue>],
    display: &[usize],
    slots: &[usize],
    staged: Option<&EditBuffer>,
    kinds: &[FieldKind],
) -> Vec<Vec<String>> {
    match staged {
        Some(buffer) => slots
            .iter()
            .map(|&slot| {
                let values = buffer
                    .row_image(slot)
                    .unwrap_or_else(|| rows.get(slot).map(Vec::as_slice).unwrap_or(&[]));
                cells(order, values, kinds)
            })
            .collect(),
        None => display
            .iter()
            .map(|&cache_row| cells(order, &rows[cache_row], kinds))
            .collect(),
    }
}
