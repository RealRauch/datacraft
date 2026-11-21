//! Localized presentation of core domain values (step 18.2, item I2).
//!
//! Core stays locale-neutral: snapshots carry enum kinds and raw values, and
//! the `inspect()` JSON is byte-identical in every language. The UI renders
//! the display text here, in the current language (ADR-4, ARCHITECTURE §15).

use datacraft_core::schema::{AutoKind, BinaryKind, FieldKind, FieldValue, NumericKind};

use datacraft_core::ObjectKind;
use datacraft_core::port::NativeQueryKind;

use crate::i18n::{t, tr};

use eframe::egui;

/// The path-field identity row (step 91.3, item L53): once the field
/// carries a path, the chosen file name renders prominently below it and
/// the full path rides its hover — a realistic long path is identifiable
/// at the default field size on every surface (import window and the
/// backstage path fields).
pub fn path_field_identity(ui: &mut egui::Ui, path: &str) {
    let trimmed = path.trim();
    let name = file_name_of(trimmed);
    if name.is_empty() {
        return;
    }
    ui.label(egui::RichText::new(name).strong())
        .on_hover_text(trimmed);
}

/// The file name of a path, splitting over **both** separators (step
/// 91.13): `std::path::Path` splits on the host's separator only, so a
/// Windows path shown on a Linux client keeps its full string as one
/// component — the identity would name the whole path instead of the
/// file. Empty when the path ends on a separator.
fn file_name_of(path: &str) -> &str {
    path.rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("")
}

/// Localized name of a native query flavor (the QBE viewer's subtitle).
/// The wire enum stays locale-neutral (ADR-4); the UI supplies the words.
pub fn native_query_kind_label(kind: NativeQueryKind) -> &'static str {
    match kind {
        NativeQueryKind::Select => t("Select"),
        NativeQueryKind::Crosstab => t("Crosstab"),
        NativeQueryKind::Delete => t("Delete query"),
        NativeQueryKind::Update => t("Update query"),
        NativeQueryKind::Append => t("Append query"),
        NativeQueryKind::MakeTable => t("Make-table query"),
        NativeQueryKind::DataDefinition => t("Data-definition query"),
        NativeQueryKind::Passthrough => t("Pass-through query"),
        NativeQueryKind::Union => t("Union query"),
    }
}

/// Localized singular name of an object kind (step 93.3, item L64): the
/// Object Details list's trailing Type column — the same display
/// vocabulary the catalog carries for the sections' singular labels.
pub fn object_kind_label(kind: ObjectKind) -> &'static str {
    match kind {
        ObjectKind::Table => t("Table"),
        ObjectKind::Query => t("Query"),
        ObjectKind::View => t("View"),
        ObjectKind::Form => t("Form"),
        ObjectKind::Report => t("Report"),
        ObjectKind::Macro => t("Macro"),
        ObjectKind::Module => t("Module"),
    }
}

/// Access UI type name of a field kind (design grid and field list).
pub fn field_kind_label(kind: &FieldKind) -> String {
    match kind {
        FieldKind::Text { .. } => t("Short Text").to_owned(),
        FieldKind::Memo => t("Long Text").to_owned(),
        FieldKind::Number(numeric) => match numeric {
            NumericKind::Byte => t("Byte").to_owned(),
            NumericKind::Integer => t("Integer").to_owned(),
            NumericKind::LongInteger => t("Long Integer").to_owned(),
            NumericKind::Single => t("Single").to_owned(),
            NumericKind::Double => t("Double").to_owned(),
            NumericKind::Decimal { precision, scale } => tr(
                "Decimal ({precision}, {scale})",
                &[
                    ("precision", &precision.to_string()),
                    ("scale", &scale.to_string()),
                ],
            ),
        },
        FieldKind::DateTime => t("Date/Time").to_owned(),
        FieldKind::Currency => t("Currency").to_owned(),
        FieldKind::AutoNumber(AutoKind::LongInteger) => t("AutoNumber").to_owned(),
        FieldKind::AutoNumber(AutoKind::ReplicationId) => {
            t("AutoNumber (Replication ID)").to_owned()
        }
        FieldKind::Boolean { .. } => t("Yes/No").to_owned(),
        FieldKind::Binary(BinaryKind::OleObject) => t("OLE Object").to_owned(),
        FieldKind::Binary(BinaryKind::Attachment) => t("Attachment").to_owned(),
        FieldKind::MultiValue(_) => t("Multi-Value").to_owned(),
        FieldKind::Complex => t("Complex").to_owned(),
        FieldKind::Guid => t("GUID").to_owned(),
    }
}

/// User-renderable text for one cell. Core's locale-neutral formatting
/// carries numbers and dates; the UI adds the language for booleans, binary
/// markers and complex joins.
pub fn value_label(value: &FieldValue) -> String {
    match value {
        FieldValue::Boolean(true) => t("Yes").to_owned(),
        FieldValue::Boolean(false) => t("No").to_owned(),
        FieldValue::Binary(bytes) => tr(
            "<binary: {bytes} bytes>",
            &[("bytes", &bytes.len().to_string())],
        ),
        FieldValue::Complex(records) => records
            .iter()
            .filter_map(|record| record.scalar())
            .map(value_label)
            .collect::<Vec<_>>()
            .join(", "),
        other => datacraft_core::present::display_value(other),
    }
}

#[cfg(test)]
mod tests {
    use datacraft_core::port::NativeQueryKind;

    use super::*;
    use crate::i18n::{Lang, lang_lock, set_lang};

    #[test]
    fn native_query_kinds_are_localized() {
        let _guard = lang_lock();
        set_lang(Lang::English);
        assert_eq!(native_query_kind_label(NativeQueryKind::Select), "Select");
        set_lang(Lang::German);
        assert_eq!(native_query_kind_label(NativeQueryKind::Select), "Auswahl");
        assert_eq!(
            native_query_kind_label(NativeQueryKind::MakeTable),
            "Tabellenerstellungsabfrage"
        );
        set_lang(Lang::English);
    }

    /// The identity splits over both separators on every host (step
    /// 91.13): a Windows path shown on a Linux client names the file,
    /// not the whole string.
    #[test]
    fn the_identity_splits_over_both_separators() {
        assert_eq!(
            file_name_of("C:\\Users\\someuser\\Downloads\\shop.mdb"),
            "shop.mdb"
        );
        assert_eq!(file_name_of("/home/someuser/shop.mdb"), "shop.mdb");
        assert_eq!(file_name_of("shop.mdb"), "shop.mdb");
        assert_eq!(file_name_of("C:\\data\\dir\\"), "");
    }
}
