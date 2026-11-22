//! PDF report export (phase 97, item M128; roadmap §11).
//!
//! A second render target of the print-preview page model: the core
//! paginates (`report_preview`), this writer draws exactly those pages —
//! it never re-paginates or re-measures. The output is deterministic
//! (fixed metadata, stable object order, no timestamps or random IDs) and
//! keeps text as text: the embedded Inter face is written as a Type0
//! CID font with identity encoding and a ToUnicode map, so extractors
//! recover the strings.
//!
//! Without the `pdf-export` feature the call answers a precise failure
//! (the sql-engine precedent): a default-feature core carries neither the
//! writer nor the font parser.

use std::path::Path;

use crate::report::ReportDef;
#[cfg(feature = "pdf-export")]
use crate::report_preview::PreviewItem;
use crate::report_preview::PreviewPage;

/// The embedded preview face (the shipped UI body font; SIL OFL, see
/// `THIRD-PARTY-NOTICES.md`).
#[cfg(feature = "pdf-export")]
const FONT: &[u8] = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");

/// Preview body size in points (the UI's body rung at 100 % scale).
#[cfg(feature = "pdf-export")]
pub const TEXT_SIZE: f32 = 12.5;

/// PDF object ids (stable allocation makes the output deterministic).
#[cfg(feature = "pdf-export")]
const CATALOG: i32 = 1;
#[cfg(feature = "pdf-export")]
const PAGES: i32 = 2;
#[cfg(feature = "pdf-export")]
const TYPE0: i32 = 3;
#[cfg(feature = "pdf-export")]
const CID: i32 = 4;
#[cfg(feature = "pdf-export")]
const DESCRIPTOR: i32 = 5;
#[cfg(feature = "pdf-export")]
const FONT_FILE: i32 = 6;
#[cfg(feature = "pdf-export")]
const TO_UNICODE: i32 = 7;
#[cfg(feature = "pdf-export")]
const FIRST_PAGE: i32 = 10;

/// The feature-gated writer entry: paginated pages → a new PDF file.
#[cfg(feature = "pdf-export")]
pub fn export_pdf(
    report: &ReportDef,
    pages: &[PreviewPage],
    destination: &Path,
) -> Result<(), String> {
    let bytes = build(report, pages)?;
    datacraft_core_write(destination, &bytes)
}

/// The marker every feature-less build returns (the dispatcher maps it
/// onto the Unsupported failure class).
pub const FEATURE_MISSING: &str =
    "the PDF writer is not part of this build (enable the datacraft-core `pdf-export` feature)";

/// Without the feature the command still answers precisely.
#[cfg(not(feature = "pdf-export"))]
pub fn export_pdf(
    _report: &ReportDef,
    _pages: &[PreviewPage],
    _destination: &Path,
) -> Result<(), String> {
    Err(FEATURE_MISSING.to_owned())
}

/// Writes a new file; an existing target is refused (an export never
/// overwrites a foreign file).
#[cfg(feature = "pdf-export")]
fn datacraft_core_write(destination: &Path, bytes: &[u8]) -> Result<(), String> {
    crate::atomic_write::write_new(destination, bytes).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            format!("{}: the target file already exists", destination.display())
        } else {
            format!("{}: {error}", destination.display())
        }
    })
}

/// Builds the PDF bytes from the preview page model.
#[cfg(feature = "pdf-export")]
fn build(report: &ReportDef, pages: &[PreviewPage]) -> Result<Vec<u8>, String> {
    use pdf_writer::types::{CidFontType, FontFlags, SystemInfo, UnicodeCmap};
    use pdf_writer::{Finish, Name, Pdf, Rect as PdfRect, Ref, Str};

    let face = ttf_parser::Face::parse(FONT, 0)
        .map_err(|error| format!("the embedded font failed to parse: {error}"))?;
    let upem = f32::from(face.units_per_em());
    let scale = 1000.0 / upem;
    let glyphs = Glyphs::collect(pages, &face);

    let mut pdf = Pdf::new();
    pdf.catalog(Ref::new(CATALOG)).pages(Ref::new(PAGES));
    let page_ids: Vec<Ref> = (0..pages.len())
        .map(|index| Ref::new(FIRST_PAGE + index as i32 * 2))
        .collect();
    pdf.pages(Ref::new(PAGES))
        .kids(page_ids.iter().copied())
        .count(pages.len() as i32);

    // The embedded face: full TrueType program, identity CID mapping and
    // a ToUnicode map so the text extracts as text.
    let mut font_stream = pdf.stream(Ref::new(FONT_FILE), FONT);
    font_stream.pair(Name(b"Length1"), FONT.len() as i32);
    font_stream.finish();

    let bbox = face.global_bounding_box();
    let mut descriptor = pdf.font_descriptor(Ref::new(DESCRIPTOR));
    descriptor
        .name(Name(b"Inter-Regular"))
        .flags(FontFlags::SYMBOLIC)
        .bbox(PdfRect::new(
            f32::from(bbox.x_min) * scale,
            f32::from(bbox.y_min) * scale,
            f32::from(bbox.x_max) * scale,
            f32::from(bbox.y_max) * scale,
        ))
        .italic_angle(0.0)
        .ascent(f32::from(face.ascender()) * scale)
        .descent(f32::from(face.descender()) * scale)
        .cap_height(f32::from(face.capital_height().unwrap_or(face.ascender())) * scale)
        .stem_v(80.0)
        .font_file2(Ref::new(FONT_FILE));
    descriptor.finish();

    let mut cid = pdf.cid_font(Ref::new(CID));
    cid.subtype(CidFontType::Type2)
        .base_font(Name(b"Inter-Regular"))
        .system_info(SystemInfo {
            registry: Str(b"Adobe"),
            ordering: Str(b"Identity"),
            supplement: 0,
        })
        .font_descriptor(Ref::new(DESCRIPTOR))
        .default_width(0.0)
        .cid_to_gid_map_predefined(Name(b"Identity"));
    let mut widths = cid.widths();
    for (glyph, width) in &glyphs.widths {
        widths.same(*glyph, *glyph, *width);
    }
    drop(widths);
    cid.finish();

    let mut cmap = UnicodeCmap::new(
        Name(b"DataCraftInter"),
        SystemInfo {
            registry: Str(b"Adobe"),
            ordering: Str(b"Identity"),
            supplement: 0,
        },
    );
    for (glyph, character) in &glyphs.to_unicode {
        cmap.pair(*glyph, *character);
    }
    pdf.stream(Ref::new(TO_UNICODE), &cmap.finish());

    pdf.type0_font(Ref::new(TYPE0))
        .base_font(Name(b"Inter-Regular"))
        .encoding_predefined(Name(b"Identity-H"))
        .descendant_font(Ref::new(CID))
        .to_unicode(Ref::new(TO_UNICODE))
        .finish();

    for (index, page) in pages.iter().enumerate() {
        let page_id = page_ids[index];
        let content_id = Ref::new(FIRST_PAGE + index as i32 * 2 + 1);
        let mut content = pdf_writer::Content::new();
        content.begin_text();
        content.set_font(Name(b"F1"), TEXT_SIZE);
        for item in &page.items {
            let encoded = glyphs.encode(&item.text, &face);
            if encoded.is_empty() {
                continue;
            }
            let (x, y) = text_origin(report, item);
            content.set_text_matrix([1.0, 0.0, 0.0, 1.0, x, y]);
            content.show(Str(&encoded));
        }
        content.end_text();
        pdf.stream(content_id, &content.finish());

        let mut writer = pdf.page(page_id);
        writer
            .parent(Ref::new(PAGES))
            .media_box(PdfRect::new(
                0.0,
                0.0,
                report.page.width,
                report.page.height,
            ))
            .contents(content_id);
        writer
            .resources()
            .fonts()
            .pair(Name(b"F1"), Ref::new(TYPE0));
        writer.finish();
    }
    Ok(pdf.finish())
}

/// Bottom-left text origin of one preview item: the preview paints at the
/// rect's top-left; PDF text is positioned at the baseline.
#[cfg(feature = "pdf-export")]
fn text_origin(report: &ReportDef, item: &PreviewItem) -> (f32, f32) {
    let x = item.rect.x.max(0.0);
    let y = report.page.height - item.rect.y - TEXT_SIZE;
    (x, y)
}

/// The used glyphs: encoded strings, widths and the ToUnicode pairs.
#[cfg(feature = "pdf-export")]
struct Glyphs {
    /// Per character, its glyph id (the first mapping wins; a missing
    /// glyph becomes `.notdef` = 0 and never enters ToUnicode).
    by_char: std::collections::BTreeMap<char, u16>,
    /// Used glyph ids with their widths in 1000-unit space.
    widths: Vec<(u16, f32)>,
    /// Glyph id → the character it came from.
    to_unicode: Vec<(u16, char)>,
}

#[cfg(feature = "pdf-export")]
impl Glyphs {
    fn collect(pages: &[PreviewPage], face: &ttf_parser::Face<'_>) -> Self {
        let scale = 1000.0 / f32::from(face.units_per_em());
        let mut by_char = std::collections::BTreeMap::new();
        for page in pages {
            for item in &page.items {
                for character in item.text.chars() {
                    by_char.entry(character).or_insert_with(|| {
                        face.glyph_index(character)
                            .map(|glyph| glyph.0)
                            .unwrap_or(0)
                    });
                }
            }
        }
        let mut seen = std::collections::BTreeMap::new();
        for (character, glyph) in &by_char {
            if *glyph == 0 {
                continue;
            }
            seen.entry(*glyph).or_insert(*character);
        }
        let widths = seen
            .keys()
            .map(|glyph| {
                let width = face
                    .glyph_hor_advance(ttf_parser::GlyphId(*glyph))
                    .map_or(0.0, |advance| f32::from(advance) * scale);
                (*glyph, width)
            })
            .collect();
        Self {
            by_char,
            widths,
            to_unicode: seen.into_iter().collect(),
        }
    }

    /// Big-endian glyph ids of one string (Identity-H).
    fn encode(&self, text: &str, _face: &ttf_parser::Face<'_>) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(text.chars().count() * 2);
        for character in text.chars() {
            let glyph = self.by_char.get(&character).copied().unwrap_or(0);
            bytes.extend_from_slice(&glyph.to_be_bytes());
        }
        bytes
    }
}
