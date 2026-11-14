# M88 import contract — native Access forms/reports into sidecar designs

> Step 77.3 (item M88). Binding mapping contract for the Route B migration
> tool (`datacraft-import`, Phase 78). Decision record: ADR-9 in
> `docs/ARCHITECTURE.md` §18. Feasibility evidence: `docs/FORMAT-JET.md`
> "M88 feasibility — jetdb against the committed exports". Route A (own
> decoder) must implement the same contract; see `docs/ROADMAP.md` §20.

## 1. Scope

- **Source:** a Jet4/ACE `.mdb`/`.accdb` file, read-only through `jetdb`
  0.4 (pinned). Access 97 (Jet3) form/report designs are out of scope
  (`jetdb` reports `Jet3FormsNotRead`).
- **Output:** the `.datacraft` sidecar next to the database (or a path
  given explicitly). The database itself is never written (ADR-1).
- **Objects:** forms and reports whose controls fall inside the supported
  subset. Every other control and every unmapped property is reported per
  object; nothing is dropped silently.
- **Events and VBA:** never bound (ADR-2). Event properties are listed as
  ignored, not imported.

## 2. Object mapping

| Access (jetdb/SaveAsText) | Sidecar field | Rule |
| --- | --- | --- |
| object name | `FormDef.name` / `ReportDef.name` | as read |
| `RecordSource` | `FormDef.source` / `ReportDef.source` | plain table or saved-query name; any other form (SQL, expression) is unsupported and leaves `source` empty |
| `Width` (twips) | `FormDef.size.width` | `Width / 20`; absent → max control right edge |
| section heights (Detail, headers, footers) | `FormDef.size.height` | sum of the section heights `/ 20`; no sections → max control bottom |
| — | `FormDef.grid` / `ReportDef.grid` | fixed `8.0` (the designer default; imported rects are not re-snapped) |
| — | `ReportDef.page` | `PageSetup::default()` (A4 portrait, 36 pt margins) |
| `Caption` (form/report level) | — | ignored: the sidecar has no form/report caption; reported once per object |
| `Filter`, fonts, colours, printer settings | — | ignored by category (appearance/print); not an error |

## 3. Control mapping

Twips → points: `value / 20.0` (1440 twips = 72 pt = 1 inch). `Rect` is
`{x, y, width, height}` in points.

| jetdb control type | `ControlKind` | Caption/source mapping |
| --- | --- | --- |
| `Label` | `Label { text }` | `Caption` → `text` (absent → empty) |
| `TextBox` | `TextBox { bound_field }` | `ControlSource` → `bound_field` when it is a bare field name; `=` expressions are unsupported |
| `CheckBox` | `CheckBox { bound_field }` | `ControlSource` → `bound_field` (bare field only) |
| `ComboBox` | `ComboBox { bound_field, row_source, display_field }` | `ControlSource` → `bound_field`; `row_source`/`display_field` stay empty in the first cut (documented limitation) |
| `CommandButton` | `Button { caption, action: None }` | `Caption` → `caption`; no event binding |
| `SubReport` | `SubForm { form }` | `SourceObject` → `form` when it is a plain form name (optional `Form.` prefix stripped); expressions/other sources are unsupported |
| `Detail`, `FormHeader`, `FormFooter`, `PageHeader`, `PageFooter`, `GroupHeader`, `GroupFooter`, `ReportHeader`, `ReportFooter` | — | section markers: not controls; their `Height` feeds the canvas height |
| anything else (`ListBox`, `TabControl`, `Image`, `OptionGroup`, `ToggleButton`, …) | — | unsupported control: reported with name and type, never guessed |

Control order is the order `jetdb` returns (sections excluded); it is the
paint order for the simple, non-overlapping designs in scope.

## 4. Defaults and numeric edge cases

The blob omits properties equal to their Access defaults; `SaveAsText`
writes them. The importer therefore fills only the defaults confirmed by
the fixtures — never guesses:

| Property | Confirmed default | Source |
| --- | --- | --- |
| `Left` | `0` | 77.1 fixture (labels) |
| `Top` | `0` | 77.1 fixture (Stock Summary Label2) |
| `Width` (TextBox) | `1440` twips | 77.1 fixture (all text boxes) |

Any other missing `Left`/`Top`/`Width`/`Height` (or a missing `Name`) is
reported as unsupported for that control and the control is skipped — the
generated six-kind fixture (Phase 78) must confirm the remaining defaults
before they are added to this table.

Two-byte numeric properties are read as **unsigned** (`u16`): `jetdb`'s
`BlobValue::Short` renders values above 32767 signed (Right/Bottom 50000 →
−15536). Negative values outside the 0..=65535 range indicate an
unsupported property. Our subset uses `Left`/`Top`/`Width`/`Height`, which
stay below the limit in the fixtures, but the conversion is `as u16` with a
range check, not a trust in the sign.

## 5. Unsupported report

The tool emits one report entry per object:

```json
{
  "object": "Items Browser",
  "kind": "form",
  "imported_controls": 5,
  "unsupported_controls": [{"name": "List1", "type": "ListBox"}],
  "unsupported_properties": [
    {"control": "txtItemName", "property": "Width", "reason": "no confirmed default"}
  ],
  "ignored_properties": ["Caption", "FontName", "Filter"]
}
```

`ignored_properties` is a bounded category list (appearance, events, print),
not an error; `unsupported_*` entries mean the sidecar is incomplete for
that object and the user must review it. The CLI prints a summary and
exits 0 when at least one object was imported; `--strict` turns any
unsupported entry into exit 1.

## 6. Fixture pair (frozen 2026-11-14)

The contract test is the committed fixture plus an expected sidecar:

- **Source:** `fixtures/programmability.accdb` (manifest-pinned, read-only;
  never copied into the test tree).
- **Expected:** a sidecar document with `database_name: "programmability"`,
  empty `tables`/`queries`/`views`, and these forms/reports:

| Object | source | size (pt) | controls (x, y, w, h in pt) |
| --- | --- | --- | --- |
| Items Browser (form) | Warehouse Items | 444.2 × 75.0 | txtItemName TextBox(75, 15, 72, 15.75); Label1 Label "Item Name"(0, 15, 72, 15.75); txtStockLevel TextBox(75, 40, 72, 15.75); Label2 Label "Stock Level"(0, 40, 72, 15.75); cmdFilter Button "Filter in stock"(160, 15, 90, 20) |
| Movement Editor (form) | Stock Movements | 444.2 × 75.0 | txtQuantity TextBox(75, 15, 72, 15.75); Label1 Label "Quantity"(0, 15, 72, 15.75); txtMovedOn TextBox(75, 40, 72, 15.75); Label2 Label "Moved On"(0, 40, 72, 15.75) |
| Category Picker (form) | Categories | 444.2 × 60.0 | txtCategoryName TextBox(75, 15, 72, 15.75); Label1 Label "Category"(0, 15, 72, 15.75) |
| Stock Summary (report) | Items In Stock | — (A4 page) | txtItemName TextBox(75, 15, 72, 15.75); Label1 Label "Item Name"(0, 15, 72, 15.75); txtStockLevel TextBox(160, 15, 72, 15.75); Label2 Label "Stock Level"(160, 0, 72, 15.75) |

Text boxes bind `ItemName`, `StockLevel`, `Quantity`, `MovedOn`,
`CategoryName` respectively. The expected values are derived from the
committed SaveAsText exports (`tools/fixture-build/objects/`), which remain
the ground truth.

**Coverage gap (Phase 78 must close it):** the committed fixture contains
only TextBox, Label, CommandButton and Detail. A generated fixture with all
six control kinds and a defaults object (Access COM, `tools/fixture-build`
recipe) pins CheckBox, ComboBox, SubForm and the remaining default values
before the importer claims them.

## 7. CLI contract (Phase 78)

```text
datacraft-import <database> [--sidecar <path>] [--strict] [--json]
```

- Default sidecar path: `SchemaDocument::sidecar_path(database)`.
- Merge semantics: load an existing sidecar if present; replace imported
  forms/reports by case-insensitive name; keep all other tables, queries,
  views, forms and reports.
- Never writes the database; the sidecar is written atomically.
- Exit codes: 0 success, 1 failure or `--strict` with unsupported entries,
  2 usage error (the CLI convention).
- `--json` prints the full report (section 5) plus the imported object
  names; the default prints a human summary.
