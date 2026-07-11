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

/// Which piece of the invitation a positioned text field renders.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum FieldKind {
    Couple,
    GuestName,
    Date,
    Time,
    Hall,
    Venue,
    RsvpBy,
    Admits,
}

/// A single positioned text field (mm from the bottom-left, per PDF origin).
#[derive(Debug, Clone, Deserialize)]
struct FieldSpec {
    field: FieldKind,
    x_mm: f32,
    y_mm: f32,
    #[serde(default = "default_size")]
    size: f32,
    /// RGB in 0.0..=1.0; defaults to a dark grey.
    #[serde(default = "default_color")]
    color: [f32; 3],
}

fn default_size() -> f32 {
    18.0
}
fn default_color() -> [f32; 3] {
    [0.23, 0.23, 0.23]
}

#[derive(Debug, Deserialize)]
struct Layout {
    fields: Vec<FieldSpec>,
}

/// A5 portrait, the default when no template image sizes the page.
const DEFAULT_W_MM: f32 = 148.0;
const DEFAULT_H_MM: f32 = 210.0;

/// `InvitePdfRenderer` that overlays positioned text onto an optional base
/// template image using a configurable font. Everything is env-driven:
///
/// * `PDF_TEMPLATE_IMAGE` — path to a PNG/JPEG card background (page is sized to
///   it at `PDF_TEMPLATE_DPI`, default 300). Omitted → a plain A5 page.
/// * `PDF_FONT` — path to a TTF. Omitted → the builtin Times-Roman.
/// * `PDF_LAYOUT` — path to a JSON `{ "fields": [...] }` positioning file.
///   Omitted → a sensible default layout.
///
/// With none of them set it still produces a valid text-only invitation, so the
/// endpoint works out of the box.
pub struct TemplatePdfRenderer {
    template_png: Option<Vec<u8>>,
    font_bytes: Option<Vec<u8>>,
    dpi: f32,
    layout: Layout,
}

impl TemplatePdfRenderer {
    /// Build the renderer from environment configuration, loading the template
    /// image and font into memory once.
    pub fn from_env() -> Result<Self, DomainError> {
        let template_png = match std::env::var("PDF_TEMPLATE_IMAGE") {
            Ok(path) => Some(read_file(&path)?),
            Err(_) => None,
        };
        let font_bytes = match std::env::var("PDF_FONT") {
            Ok(path) => Some(read_file(&path)?),
            Err(_) => None,
        };
        let dpi = std::env::var("PDF_TEMPLATE_DPI")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(300.0);
        let layout = match std::env::var("PDF_LAYOUT") {
            Ok(path) => {
                let raw = read_file(&path)?;
                serde_json::from_slice(&raw)
                    .map_err(|e| DomainError::Pdf(format!("invalid PDF_LAYOUT: {e}")))?
            }
            Err(_) => default_layout(),
        };
        Ok(Self {
            template_png,
            font_bytes,
            dpi,
            layout,
        })
    }

    fn page_dimensions(&self) -> Result<(f32, f32), DomainError> {
        match &self.template_png {
            Some(bytes) => {
                let img = image_crate::load_from_memory(bytes)
                    .map_err(|e| DomainError::Pdf(format!("cannot decode template image: {e}")))?;
                let px_to_mm = 25.4 / self.dpi;
                Ok((
                    img.width() as f32 * px_to_mm,
                    img.height() as f32 * px_to_mm,
                ))
            }
            None => Ok((DEFAULT_W_MM, DEFAULT_H_MM)),
        }
    }
}

impl InvitePdfRenderer for TemplatePdfRenderer {
    fn render(&self, event: &Event, guest: &Guest) -> Result<Vec<u8>, DomainError> {
        let (w_mm, h_mm) = self.page_dimensions()?;
        let (doc, page, layer) =
            PdfDocument::new("Wedding Invitation", Mm(w_mm), Mm(h_mm), "Invitation");
        let layer: PdfLayerReference = doc.get_page(page).get_layer(layer);

        // Background template, scaled to fill the page.
        if let Some(bytes) = &self.template_png {
            let dynamic = image_crate::load_from_memory(bytes)
                .map_err(|e| DomainError::Pdf(format!("cannot decode template image: {e}")))?;
            let image = Image::from_dynamic_image(&dynamic);
            image.add_to_layer(
                layer.clone(),
                ImageTransform {
                    translate_x: Some(Mm(0.0)),
                    translate_y: Some(Mm(0.0)),
                    dpi: Some(self.dpi),
                    ..Default::default()
                },
            );
        }

        let font: IndirectFontRef = match &self.font_bytes {
            Some(bytes) => doc
                .add_external_font(Cursor::new(bytes.as_slice()))
                .map_err(|e| DomainError::Pdf(format!("cannot load font: {e}")))?,
            None => doc
                .add_builtin_font(BuiltinFont::TimesRoman)
                .map_err(|e| DomainError::Pdf(format!("cannot load builtin font: {e}")))?,
        };

        for spec in &self.layout.fields {
            let text = field_text(spec.field, event, guest);
            if text.is_empty() {
                continue;
            }
            layer.set_fill_color(Color::Rgb(Rgb::new(
                spec.color[0],
                spec.color[1],
                spec.color[2],
                None,
            )));
            layer.use_text(text, spec.size, Mm(spec.x_mm), Mm(spec.y_mm), &font);
        }

        doc.save_to_bytes()
            .map_err(|e| DomainError::Pdf(format!("cannot serialize PDF: {e}")))
    }
}

fn read_file(path: &str) -> Result<Vec<u8>, DomainError> {
    std::fs::read(path).map_err(|e| DomainError::Pdf(format!("cannot read {path}: {e}")))
}

/// Resolve a field to the text drawn for this event/guest.
fn field_text(kind: FieldKind, e: &Event, g: &Guest) -> String {
    match kind {
        FieldKind::Couple => format!(
            "{} {} & {} {}",
            e.bride_name, e.bride_family_name, e.groom_name, e.groom_family_name
        ),
        FieldKind::GuestName => g.name.clone(),
        FieldKind::Date => e.event_date.format("%A, %e %B %Y").to_string(),
        FieldKind::Time => format!(
            "{} - {}",
            e.start_time.format("%H:%M"),
            e.end_time.format("%H:%M")
        ),
        FieldKind::Hall => e.hall_name.clone(),
        FieldKind::Venue => e.venue_name.clone(),
        FieldKind::RsvpBy => format!("RSVP by {}", e.rsvp_by.format("%e %B %Y")),
        FieldKind::Admits => format!("Admits up to {}", g.max_party_size),
    }
}

/// Reasonable A5-portrait layout used when `PDF_LAYOUT` is not configured.
/// (y grows upward from the bottom of the page.)
fn default_layout() -> Layout {
    let dark = default_color();
    let f = |field, y, size| FieldSpec {
        field,
        x_mm: 20.0,
        y_mm: y,
        size,
        color: dark,
    };
    Layout {
        fields: vec![
            f(FieldKind::Couple, 185.0, 26.0),
            f(FieldKind::GuestName, 150.0, 20.0),
            f(FieldKind::Date, 120.0, 16.0),
            f(FieldKind::Time, 105.0, 16.0),
            f(FieldKind::Hall, 90.0, 16.0),
            f(FieldKind::Venue, 75.0, 14.0),
            f(FieldKind::Admits, 55.0, 14.0),
            f(FieldKind::RsvpBy, 40.0, 14.0),
        ],
    }
}
