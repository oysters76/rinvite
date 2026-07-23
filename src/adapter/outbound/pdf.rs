use std::collections::HashMap;

use printpdf::{
    BuiltinFont, Color, FontId, Mm, Op, ParsedFont, PdfDocument, PdfFontHandle, PdfPage,
    PdfSaveOptions, Point, Pt, RawImage, RawImageData, RawImageFormat, Rgb, TextItem, TextMatrix,
    XObjectId, XObjectTransform,
};
use serde::Deserialize;

use crate::domain::error::DomainError;
use crate::domain::event::Event;
use crate::domain::guest::Guest;
use crate::domain::port::outbound::InvitePdfRenderer;

// ===== Config (deserialized from the JSON pointed at by PDF_CONFIG) =========

#[derive(Debug, Deserialize)]
struct PdfConfig {
    template_image: String,
    #[serde(default = "default_dpi")]
    dpi: f32,
    /// Fixed page size in mm `[width, height]`. When set, the template is
    /// scaled to fill this page (and element anchors scale with it). When
    /// omitted, the page is sized to the template image at `dpi`.
    #[serde(default)]
    page_mm: Option<[f32; 2]>,
    /// name -> TTF path.
    fonts: HashMap<String, String>,
    elements: Vec<Element>,
}

fn default_dpi() -> f32 {
    300.0
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Align {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Transform {
    #[default]
    None,
    Upper,
    Lower,
}

#[derive(Debug, Clone, Deserialize)]
struct Element {
    /// Text with `{placeholder}` tokens (see `resolve`).
    template: String,
    /// Key into the fonts registry.
    font: String,
    size: f32,
    x_mm: f32,
    y_mm: f32,
    #[serde(default)]
    align: Align,
    #[serde(default = "default_color")]
    color: [f32; 3],
    #[serde(default)]
    transform: Transform,
    /// Extra tracking between glyphs, in mm (renders glyph-by-glyph when > 0).
    #[serde(default)]
    letter_spacing: f32,
}

fn default_color() -> [f32; 3] {
    [0.20, 0.15, 0.10]
}

// ===== Renderer =============================================================

/// A loaded font: its raw bytes, re-added to each `PdfDocument` and re-parsed
/// into a `ttf_parser::Face` once per batch for width measurement.
struct FontAsset {
    bytes: Vec<u8>,
}

/// A5-portrait default when there's no config.
const DEFAULT_W_MM: f32 = 148.0;
const DEFAULT_H_MM: f32 = 210.0;

/// `InvitePdfRenderer` that overlays positioned, multi-font text onto a base
/// template image, matching a configurable layout.
///
/// * `PDF_CONFIG` — path to the JSON layout (template image, dpi, font
///   registry, positioned elements). When unset, falls back to a plain A5 page
///   with the builtin Times font so the endpoint still works out of the box.
pub struct TemplatePdfRenderer {
    config: Option<LoadedConfig>,
    /// Dev-only: when `PDF_CONFIG_RELOAD` is set, holds the `PDF_CONFIG` path so
    /// the layout (and its image/fonts) is re-read on every render — letting you
    /// tune `pdf-config.json` and see it with a plain repeated curl, no restart.
    reload_path: Option<String>,
}

struct LoadedConfig {
    template_png: Vec<u8>,
    dpi: f32,
    page_mm: Option<[f32; 2]>,
    fonts: HashMap<String, FontAsset>,
    elements: Vec<Element>,
}

impl TemplatePdfRenderer {
    pub fn from_env() -> Result<Self, DomainError> {
        let path = std::env::var("PDF_CONFIG").ok().filter(|p| !p.is_empty());
        // Dev hot-reload: any non-empty, non-"0" value re-reads the config each
        // render. Off by default, so production keeps the startup-cached config.
        let reload = std::env::var("PDF_CONFIG_RELOAD")
            .ok()
            .is_some_and(|v| !v.is_empty() && v != "0");
        match (reload, path) {
            (true, Some(p)) => Ok(Self {
                config: None,
                reload_path: Some(p),
            }),
            (_, Some(p)) => Ok(Self {
                config: Some(Self::load_config(&p)?),
                reload_path: None,
            }),
            (_, None) => Ok(Self {
                config: None,
                reload_path: None,
            }),
        }
    }

    fn load_config(path: &str) -> Result<LoadedConfig, DomainError> {
        let raw = read_file(path)?;
        let cfg: PdfConfig = serde_json::from_slice(&raw)
            .map_err(|e| DomainError::Pdf(format!("invalid PDF_CONFIG {path}: {e}")))?;

        let template_png = read_file(&cfg.template_image)?;

        let mut fonts = HashMap::new();
        for (name, font_path) in &cfg.fonts {
            let bytes = read_file(font_path)?;
            // Validate the font up front so a bad file fails at load, not render.
            ttf_parser::Face::parse(&bytes, 0)
                .map_err(|e| DomainError::Pdf(format!("cannot parse font {font_path}: {e}")))?;
            fonts.insert(name.clone(), FontAsset { bytes });
        }

        Ok(LoadedConfig {
            template_png,
            dpi: cfg.dpi,
            page_mm: cfg.page_mm,
            fonts,
            elements: cfg.elements,
        })
    }
}

impl InvitePdfRenderer for TemplatePdfRenderer {
    fn render(&self, event: &Event, guest: &Guest) -> Result<Vec<u8>, DomainError> {
        self.render_all(event, std::slice::from_ref(guest))
    }

    fn render_all(&self, event: &Event, guests: &[Guest]) -> Result<Vec<u8>, DomainError> {
        // Dev hot-reload path: re-read the config (and its image/fonts) so edits
        // to pdf-config.json are picked up without restarting the server.
        if let Some(path) = &self.reload_path {
            let cfg = Self::load_config(path)?;
            return render_all_from_config(&cfg, event, guests);
        }
        match &self.config {
            Some(cfg) => render_all_from_config(cfg, event, guests),
            None => render_all_plain(event, guests),
        }
    }
}

/// Page geometry derived once from the template + optional fixed page size,
/// shared across every page of a batch.
struct PageGeom {
    page_w: f32,
    page_h: f32,
    sx: f32,
    sy: f32,
    /// The template decoded once, flattened to opaque RGB, as a printpdf image.
    image: RawImage,
}

fn page_geom(cfg: &LoadedConfig) -> Result<PageGeom, DomainError> {
    let dynamic = image::load_from_memory(&cfg.template_png)
        .map_err(|e| DomainError::Pdf(format!("cannot decode template image: {e}")))?;
    let px_to_mm = 25.4 / cfg.dpi;
    let natural_w = dynamic.width() as f32 * px_to_mm;
    let natural_h = dynamic.height() as f32 * px_to_mm;
    let (page_w, page_h) = match cfg.page_mm {
        Some([w, h]) => (w, h),
        None => (natural_w, natural_h),
    };
    // Flatten to RGB — a PNG alpha channel would otherwise render as transparency
    // and hide the whole card. Build the printpdf `RawImage` directly from the
    // opaque bytes so it can be embedded once and shared across every page.
    let rgb = dynamic.to_rgb8();
    let (px_w, px_h) = (rgb.width() as usize, rgb.height() as usize);
    let image = RawImage {
        pixels: RawImageData::U8(rgb.into_raw()),
        width: px_w,
        height: px_h,
        data_format: RawImageFormat::RGB8,
        tag: Vec::new(),
    };
    Ok(PageGeom {
        page_w,
        page_h,
        sx: page_w / natural_w,
        sy: page_h / natural_h,
        image,
    })
}

fn render_all_from_config(
    cfg: &LoadedConfig,
    event: &Event,
    guests: &[Guest],
) -> Result<Vec<u8>, DomainError> {
    let geom = page_geom(cfg)?;
    let mut doc = PdfDocument::new("Wedding Invitation");

    // Embed the template image ONCE and reference it from every page. This is the
    // whole point of the printpdf 0.12 op model: a single shared XObject instead
    // of a full raster per guest, so batch memory and file size stay ~1x the
    // template regardless of guest count.
    let background = doc.add_image(&geom.image);

    // Add each font once (embedded in the document), and parse its `Face` once for
    // width measurement (kept from before — low-risk and already works).
    let mut font_ids: HashMap<&str, FontId> = HashMap::new();
    let mut faces: HashMap<&str, ttf_parser::Face> = HashMap::new();
    for (name, asset) in &cfg.fonts {
        let mut warnings = Vec::new();
        let parsed = ParsedFont::from_bytes(&asset.bytes, 0, &mut warnings)
            .ok_or_else(|| DomainError::Pdf(format!("cannot parse font '{name}'")))?;
        font_ids.insert(name.as_str(), doc.add_font(&parsed));
        let face = ttf_parser::Face::parse(&asset.bytes, 0)
            .map_err(|e| DomainError::Pdf(format!("cannot parse font '{name}': {e}")))?;
        faces.insert(name.as_str(), face);
    }

    // One page per guest: background reference + positioned text, all as `Op`s.
    let pages: Vec<PdfPage> = guests
        .iter()
        .map(|guest| {
            let ops = config_page_ops(cfg, &geom, &background, &font_ids, &faces, event, guest)?;
            Ok(PdfPage::new(Mm(geom.page_w), Mm(geom.page_h), ops))
        })
        .collect::<Result<_, DomainError>>()?;

    let mut warnings = Vec::new();
    Ok(doc
        .with_pages(pages)
        .save(&PdfSaveOptions::default(), &mut warnings))
}

/// Build the ops for one guest's card: background reference + positioned text.
#[allow(clippy::too_many_arguments)]
fn config_page_ops(
    cfg: &LoadedConfig,
    geom: &PageGeom,
    background: &XObjectId,
    font_ids: &HashMap<&str, FontId>,
    faces: &HashMap<&str, ttf_parser::Face>,
    event: &Event,
    guest: &Guest,
) -> Result<Vec<Op>, DomainError> {
    let mut ops = Vec::new();

    // Background template, scaled to fill the page.
    ops.push(Op::UseXobject {
        id: background.clone(),
        transform: XObjectTransform {
            translate_x: Some(Pt(0.0)),
            translate_y: Some(Pt(0.0)),
            scale_x: Some(geom.sx),
            scale_y: Some(geom.sy),
            dpi: Some(cfg.dpi),
            rotate: None,
            no_auto_scale: false,
        },
    });

    let tokens = resolve_tokens(event, guest);
    for el in &cfg.elements {
        let face = faces
            .get(el.font.as_str())
            .ok_or_else(|| DomainError::Pdf(format!("element uses unknown font '{}'", el.font)))?;
        let font_id = font_ids
            .get(el.font.as_str())
            .ok_or_else(|| DomainError::Pdf(format!("element uses unknown font '{}'", el.font)))?;

        let text = apply_transform(substitute(&el.template, &tokens), el.transform);
        if text.is_empty() {
            continue;
        }

        let color = Color::Rgb(Rgb::new(el.color[0], el.color[1], el.color[2], None));
        // Anchor scales with the page; font size and measured text width do not
        // (text isn't part of the stretched image).
        let x_anchor = el.x_mm * geom.sx;
        let y_anchor = el.y_mm * geom.sy;

        if el.letter_spacing > 0.0 {
            push_spaced_ops(
                &mut ops,
                &text,
                el,
                face,
                font_id,
                color,
                x_anchor,
                y_anchor,
                el.letter_spacing * geom.sx,
            );
        } else {
            let width = text_width_mm(&text, face, el.size);
            let x = aligned_x(x_anchor, width, el.align);
            ops.push(Op::StartTextSection);
            ops.push(Op::SetFont {
                font: PdfFontHandle::External(font_id.clone()),
                size: Pt(el.size),
            });
            ops.push(Op::SetFillColor { col: color });
            ops.push(Op::SetTextCursor {
                pos: Point {
                    x: Mm(x).into(),
                    y: Mm(y_anchor).into(),
                },
            });
            ops.push(Op::ShowText {
                items: vec![TextItem::Text(text)],
            });
            ops.push(Op::EndTextSection);
        }
    }
    Ok(ops)
}

/// Emit ops to draw text glyph-by-glyph, inserting `spacing` mm between
/// characters. Each glyph is positioned with its own text cursor, exactly
/// reproducing the pre-migration per-glyph placement.
#[allow(clippy::too_many_arguments)]
fn push_spaced_ops(
    ops: &mut Vec<Op>,
    text: &str,
    el: &Element,
    face: &ttf_parser::Face,
    font_id: &FontId,
    color: Color,
    x_anchor: f32,
    y_anchor: f32,
    spacing: f32,
) {
    // Total width including the added tracking, for alignment.
    let chars: Vec<char> = text.chars().collect();
    let glyphs_w = text_width_mm(text, face, el.size);
    let total = glyphs_w + spacing * (chars.len().saturating_sub(1)) as f32;
    let mut x = aligned_x(x_anchor, total, el.align);

    ops.push(Op::StartTextSection);
    ops.push(Op::SetFont {
        font: PdfFontHandle::External(font_id.clone()),
        size: Pt(el.size),
    });
    ops.push(Op::SetFillColor { col: color });
    for c in chars {
        // `SetTextMatrix::Translate` sets an ABSOLUTE position (relative to the
        // page); a per-glyph `SetTextCursor` (`Td`) would be relative and the
        // offsets would accumulate, running the text off the page.
        ops.push(Op::SetTextMatrix {
            matrix: TextMatrix::Translate(Mm(x).into(), Mm(y_anchor).into()),
        });
        ops.push(Op::ShowText {
            items: vec![TextItem::Text(c.to_string())],
        });
        x += char_width_mm(c, face, el.size) + spacing;
    }
    ops.push(Op::EndTextSection);
}

/// Zero-config fallback: plain A5 pages with the builtin Times font, one per
/// guest.
fn render_all_plain(event: &Event, guests: &[Guest]) -> Result<Vec<u8>, DomainError> {
    let mut doc = PdfDocument::new("Wedding Invitation");
    // Builtin (standard-14) fonts need no registration; reference by handle.
    let font = PdfFontHandle::Builtin(BuiltinFont::TimesRoman);
    let black = Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None));

    let lines = [
        ("{bride_name} & {groom_name}", 28.0, 185.0),
        ("{guest_name}", 18.0, 165.0),
        ("{event_date_full}", 14.0, 145.0),
        ("From {start_time} to {end_time}", 14.0, 133.0),
        ("{hall_name}", 14.0, 118.0),
        ("{venue_name}", 12.0, 108.0),
        ("Admits up to {max_party_size}", 12.0, 90.0),
        ("RSVP by {rsvp_by}", 12.0, 75.0),
    ];

    let pages: Vec<PdfPage> = guests
        .iter()
        .map(|guest| {
            let tokens = resolve_tokens(event, guest);
            let mut ops = Vec::new();
            for (tpl, size, y) in lines {
                let text = substitute(tpl, &tokens);
                ops.push(Op::StartTextSection);
                ops.push(Op::SetFont {
                    font: font.clone(),
                    size: Pt(size),
                });
                ops.push(Op::SetFillColor { col: black.clone() });
                ops.push(Op::SetTextCursor {
                    pos: Point {
                        x: Mm(20.0).into(),
                        y: Mm(y).into(),
                    },
                });
                ops.push(Op::ShowText {
                    items: vec![TextItem::Text(text)],
                });
                ops.push(Op::EndTextSection);
            }
            PdfPage::new(Mm(DEFAULT_W_MM), Mm(DEFAULT_H_MM), ops)
        })
        .collect();

    let mut warnings = Vec::new();
    Ok(doc
        .with_pages(pages)
        .save(&PdfSaveOptions::default(), &mut warnings))
}

// ===== Text helpers =========================================================

fn aligned_x(x: f32, width_mm: f32, align: Align) -> f32 {
    match align {
        Align::Left => x,
        Align::Center => x - width_mm / 2.0,
        Align::Right => x - width_mm,
    }
}

fn char_width_mm(c: char, face: &ttf_parser::Face, size_pt: f32) -> f32 {
    let upm = face.units_per_em() as f32;
    let advance = face
        .glyph_index(c)
        .and_then(|g| face.glyph_hor_advance(g))
        .unwrap_or((upm * 0.5) as u16) as f32;
    // advance(units) / upm * size(pt) -> pt, then pt -> mm
    advance / upm * size_pt * 25.4 / 72.0
}

fn text_width_mm(text: &str, face: &ttf_parser::Face, size_pt: f32) -> f32 {
    let upm = face.units_per_em() as f32;
    let units: f32 = text
        .chars()
        .map(|c| {
            face.glyph_index(c)
                .and_then(|g| face.glyph_hor_advance(g))
                .unwrap_or((upm * 0.5) as u16) as f32
        })
        .sum();
    units / upm * size_pt * 25.4 / 72.0
}

fn apply_transform(s: String, t: Transform) -> String {
    match t {
        Transform::None => s,
        Transform::Upper => s.to_uppercase(),
        Transform::Lower => s.to_lowercase(),
    }
}

fn substitute(template: &str, tokens: &HashMap<&str, String>) -> String {
    let mut out = template.to_owned();
    for (k, v) in tokens {
        if out.contains('{') {
            out = out.replace(&format!("{{{k}}}"), v);
        }
    }
    out
}

fn read_file(path: &str) -> Result<Vec<u8>, DomainError> {
    std::fs::read(path).map_err(|e| DomainError::Pdf(format!("cannot read {path}: {e}")))
}

/// Ordinal suffix for a day of month (1 -> "1st", 22 -> "22nd").
fn ordinal(day: u32) -> String {
    let suffix = match (day % 10, day % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{day}{suffix}")
}

/// Format a time as "10.00 A.M." / "3.00 P.M.".
fn clock_time(t: chrono::NaiveTime) -> String {
    use chrono::Timelike;
    let h24 = t.hour();
    let (h12, ap) = match h24 {
        0 => (12, "A.M."),
        1..=11 => (h24, "A.M."),
        12 => (12, "P.M."),
        _ => (h24 - 12, "P.M."),
    };
    format!("{}.{:02} {}", h12, t.minute(), ap)
}

/// Build the placeholder -> value map for one event/guest.
fn resolve_tokens(e: &Event, g: &Guest) -> HashMap<&'static str, String> {
    use chrono::Datelike;
    let mut m = HashMap::new();
    m.insert("bride_name", e.bride_name.clone());
    m.insert("groom_name", e.groom_name.clone());
    m.insert("bride_family_name", e.bride_family_name.clone());
    m.insert("groom_family_name", e.groom_family_name.clone());
    m.insert("guest_name", g.name.clone());
    m.insert("day_name", e.event_date.format("%A").to_string());
    m.insert("day", e.event_date.day().to_string());
    m.insert("day_ordinal", ordinal(e.event_date.day()));
    m.insert("month", e.event_date.format("%B").to_string());
    m.insert("month_short", e.event_date.format("%b").to_string());
    m.insert("year", e.event_date.format("%Y").to_string());
    m.insert(
        "event_date_full",
        e.event_date.format("%A, %e %B %Y").to_string(),
    );
    m.insert("start_time", clock_time(e.start_time));
    m.insert("end_time", clock_time(e.end_time));
    m.insert("hall_name", e.hall_name.clone());
    m.insert("venue_name", e.venue_name.clone());
    // Optional: full labelled line, or empty so the config element is skipped.
    m.insert(
        "poruwa_ceremony",
        match e.poruwa_ceremony_time {
            Some(t) => format!("Poruwa Ceremony at {}", clock_time(t)),
            None => String::new(),
        },
    );
    m.insert(
        "rsvp_by",
        format!("{} {}", ordinal(e.rsvp_by.day()), e.rsvp_by.format("%B")),
    );
    m.insert("rsvp_by_full", e.rsvp_by.format("%A, %e %B %Y").to_string());
    m.insert("max_party_size", g.max_party_size.to_string());
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinals() {
        assert_eq!(ordinal(1), "1st");
        assert_eq!(ordinal(2), "2nd");
        assert_eq!(ordinal(3), "3rd");
        assert_eq!(ordinal(4), "4th");
        assert_eq!(ordinal(11), "11th");
        assert_eq!(ordinal(21), "21st");
        assert_eq!(ordinal(25), "25th");
    }

    #[test]
    fn times() {
        use chrono::NaiveTime;
        assert_eq!(
            clock_time(NaiveTime::from_hms_opt(10, 0, 0).unwrap()),
            "10.00 A.M."
        );
        assert_eq!(
            clock_time(NaiveTime::from_hms_opt(15, 30, 0).unwrap()),
            "3.30 P.M."
        );
        assert_eq!(
            clock_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
            "12.00 A.M."
        );
        assert_eq!(
            clock_time(NaiveTime::from_hms_opt(12, 0, 0).unwrap()),
            "12.00 P.M."
        );
    }

    #[test]
    fn substitution_and_transform() {
        let mut t = HashMap::new();
        t.insert("bride_name", "Hansika".to_string());
        assert_eq!(substitute("Miss {bride_name}", &t), "Miss Hansika");
        assert_eq!(
            apply_transform("Friday".to_string(), Transform::Upper),
            "FRIDAY"
        );
    }
}
