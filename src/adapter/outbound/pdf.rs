use std::collections::HashMap;
use std::io::Cursor;

use printpdf::{
    BuiltinFont, Color, Image, ImageTransform, IndirectFontRef, Mm, PdfDocument, PdfLayerReference,
    Rgb, image_crate,
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

/// A loaded font: its raw bytes (re-added to each `PdfDocument`) kept alongside
/// its parsed metrics for width measurement.
struct FontAsset {
    bytes: Vec<u8>,
    units_per_em: f32,
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
        let config = match std::env::var("PDF_CONFIG") {
            Ok(path) => Some(Self::load_config(&path)?),
            Err(_) => None,
        };
        Ok(Self { config })
    }

    fn load_config(path: &str) -> Result<LoadedConfig, DomainError> {
        let raw = read_file(path)?;
        let cfg: PdfConfig = serde_json::from_slice(&raw)
            .map_err(|e| DomainError::Pdf(format!("invalid PDF_CONFIG {path}: {e}")))?;

        let template_png = read_file(&cfg.template_image)?;

        let mut fonts = HashMap::new();
        for (name, font_path) in &cfg.fonts {
            let bytes = read_file(font_path)?;
            let face = ttf_parser::Face::parse(&bytes, 0)
                .map_err(|e| DomainError::Pdf(format!("cannot parse font {font_path}: {e}")))?;
            let units_per_em = face.units_per_em() as f32;
            fonts.insert(
                name.clone(),
                FontAsset {
                    bytes,
                    units_per_em,
                },
            );
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
    rgb: image_crate::DynamicImage,
}

fn page_geom(cfg: &LoadedConfig) -> Result<PageGeom, DomainError> {
    let dynamic = image_crate::load_from_memory(&cfg.template_png)
        .map_err(|e| DomainError::Pdf(format!("cannot decode template image: {e}")))?;
    let px_to_mm = 25.4 / cfg.dpi;
    let natural_w = dynamic.width() as f32 * px_to_mm;
    let natural_h = dynamic.height() as f32 * px_to_mm;
    let (page_w, page_h) = match cfg.page_mm {
        Some([w, h]) => (w, h),
        None => (natural_w, natural_h),
    };
    // Flatten to RGB — printpdf 0.7 renders a PNG alpha channel as transparency,
    // which would hide the whole card.
    let rgb = image_crate::DynamicImage::ImageRgb8(dynamic.to_rgb8());
    Ok(PageGeom {
        page_w,
        page_h,
        sx: page_w / natural_w,
        sy: page_h / natural_h,
        rgb,
    })
}

fn render_all_from_config(
    cfg: &LoadedConfig,
    event: &Event,
    guests: &[Guest],
) -> Result<Vec<u8>, DomainError> {
    let geom = page_geom(cfg)?;
    let (doc, page1, layer1) = PdfDocument::new(
        "Wedding Invitation",
        Mm(geom.page_w),
        Mm(geom.page_h),
        "Invitation",
    );

    // Fonts are document-level: add them once and reuse across every page.
    let mut refs: HashMap<&str, IndirectFontRef> = HashMap::new();
    for (name, asset) in &cfg.fonts {
        let font = doc
            .add_external_font(Cursor::new(asset.bytes.as_slice()))
            .map_err(|e| DomainError::Pdf(format!("cannot load font '{name}': {e}")))?;
        refs.insert(name.as_str(), font);
    }

    for (i, guest) in guests.iter().enumerate() {
        let layer = if i == 0 {
            doc.get_page(page1).get_layer(layer1)
        } else {
            let (p, l) = doc.add_page(Mm(geom.page_w), Mm(geom.page_h), "Invitation");
            doc.get_page(p).get_layer(l)
        };
        draw_config_page(&layer, cfg, &geom, &refs, event, guest)?;
    }

    doc.save_to_bytes()
        .map_err(|e| DomainError::Pdf(format!("cannot serialize PDF: {e}")))
}

/// Draw one guest's card (background + positioned text) onto a page's layer.
fn draw_config_page(
    layer: &PdfLayerReference,
    cfg: &LoadedConfig,
    geom: &PageGeom,
    refs: &HashMap<&str, IndirectFontRef>,
    event: &Event,
    guest: &Guest,
) -> Result<(), DomainError> {
    // Background template, scaled to fill the page.
    Image::from_dynamic_image(&geom.rgb).add_to_layer(
        layer.clone(),
        ImageTransform {
            translate_x: Some(Mm(0.0)),
            translate_y: Some(Mm(0.0)),
            scale_x: Some(geom.sx),
            scale_y: Some(geom.sy),
            dpi: Some(cfg.dpi),
            ..Default::default()
        },
    );

    let tokens = resolve_tokens(event, guest);
    for el in &cfg.elements {
        let asset = cfg
            .fonts
            .get(&el.font)
            .ok_or_else(|| DomainError::Pdf(format!("element uses unknown font '{}'", el.font)))?;
        let font = &refs[el.font.as_str()];

        let text = apply_transform(substitute(&el.template, &tokens), el.transform);
        if text.is_empty() {
            continue;
        }

        layer.set_fill_color(Color::Rgb(Rgb::new(
            el.color[0],
            el.color[1],
            el.color[2],
            None,
        )));

        // Anchor scales with the page; font size and measured text width do not
        // (text isn't part of the stretched image).
        let x_anchor = el.x_mm * geom.sx;
        let y_anchor = el.y_mm * geom.sy;

        if el.letter_spacing > 0.0 {
            draw_spaced(
                layer,
                &text,
                el,
                asset,
                font,
                x_anchor,
                y_anchor,
                el.letter_spacing * geom.sx,
            );
        } else {
            let width = text_width_mm(&text, asset, el.size);
            let x = aligned_x(x_anchor, width, el.align);
            layer.use_text(&text, el.size, Mm(x), Mm(y_anchor), font);
        }
    }
    Ok(())
}

/// Draw text glyph-by-glyph, inserting `spacing` mm between characters.
#[allow(clippy::too_many_arguments)]
fn draw_spaced(
    layer: &PdfLayerReference,
    text: &str,
    el: &Element,
    asset: &FontAsset,
    font: &IndirectFontRef,
    x_anchor: f32,
    y_anchor: f32,
    spacing: f32,
) {
    // Total width including the added tracking, for alignment.
    let chars: Vec<char> = text.chars().collect();
    let glyphs_w = text_width_mm(text, asset, el.size);
    let total = glyphs_w + spacing * (chars.len().saturating_sub(1)) as f32;
    let mut x = aligned_x(x_anchor, total, el.align);
    for c in chars {
        let s = c.to_string();
        layer.use_text(&s, el.size, Mm(x), Mm(y_anchor), font);
        x += char_width_mm(c, asset, el.size) + spacing;
    }
}

/// Zero-config fallback: plain A5 pages with the builtin Times font, one per
/// guest.
fn render_all_plain(event: &Event, guests: &[Guest]) -> Result<Vec<u8>, DomainError> {
    let (doc, page1, layer1) = PdfDocument::new(
        "Wedding Invitation",
        Mm(DEFAULT_W_MM),
        Mm(DEFAULT_H_MM),
        "Invitation",
    );
    let font = doc
        .add_builtin_font(BuiltinFont::TimesRoman)
        .map_err(|e| DomainError::Pdf(format!("cannot load builtin font: {e}")))?;

    for (i, guest) in guests.iter().enumerate() {
        let layer = if i == 0 {
            doc.get_page(page1).get_layer(layer1)
        } else {
            let (p, l) = doc.add_page(Mm(DEFAULT_W_MM), Mm(DEFAULT_H_MM), "Invitation");
            doc.get_page(p).get_layer(l)
        };
        let tokens = resolve_tokens(event, guest);
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
        for (tpl, size, y) in lines {
            let text = substitute(tpl, &tokens);
            layer.use_text(&text, size, Mm(20.0), Mm(y), &font);
        }
    }
    doc.save_to_bytes()
        .map_err(|e| DomainError::Pdf(format!("cannot serialize PDF: {e}")))
}

// ===== Text helpers =========================================================

fn aligned_x(x: f32, width_mm: f32, align: Align) -> f32 {
    match align {
        Align::Left => x,
        Align::Center => x - width_mm / 2.0,
        Align::Right => x - width_mm,
    }
}

fn char_width_mm(c: char, asset: &FontAsset, size_pt: f32) -> f32 {
    let face = match ttf_parser::Face::parse(&asset.bytes, 0) {
        Ok(f) => f,
        Err(_) => return size_pt * 0.5 * 25.4 / 72.0, // rough fallback
    };
    let advance = face
        .glyph_index(c)
        .and_then(|g| face.glyph_hor_advance(g))
        .unwrap_or((asset.units_per_em * 0.5) as u16) as f32;
    // advance(units) / upm * size(pt) -> pt, then pt -> mm
    advance / asset.units_per_em * size_pt * 25.4 / 72.0
}

fn text_width_mm(text: &str, asset: &FontAsset, size_pt: f32) -> f32 {
    let face = match ttf_parser::Face::parse(&asset.bytes, 0) {
        Ok(f) => f,
        Err(_) => return text.chars().count() as f32 * size_pt * 0.5 * 25.4 / 72.0,
    };
    let units: f32 = text
        .chars()
        .map(|c| {
            face.glyph_index(c)
                .and_then(|g| face.glyph_hor_advance(g))
                .unwrap_or((asset.units_per_em * 0.5) as u16) as f32
        })
        .sum();
    units / asset.units_per_em * size_pt * 25.4 / 72.0
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
