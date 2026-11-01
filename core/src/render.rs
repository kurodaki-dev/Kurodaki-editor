use crate::document::{Color, Layer, LayerKind, Page};
use tiny_skia::{
    FillRule, FilterQuality, Paint, PathBuilder, Pixmap, PixmapPaint, Rect, Transform,
};

#[derive(Debug)]
pub enum RenderError {
    InvalidSize,
    InvalidImage,
    Encode(String),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::InvalidSize => write!(f, "invalid page size"),
            RenderError::InvalidImage => write!(f, "invalid image layer data"),
            RenderError::Encode(e) => write!(f, "png encoding failed: {e}"),
        }
    }
}

impl std::error::Error for RenderError {}

fn paint_for(c: Color, opacity: f32) -> Paint<'static> {
    let mut p = Paint::default();
    let a = (c.a as f32 * opacity.clamp(0.0, 1.0)).round() as u8;
    p.set_color_rgba8(c.r, c.g, c.b, a);
    p.anti_alias = true;
    p
}

fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<tiny_skia::Path> {
    let r = r.clamp(0.0, w.min(h) / 2.0);
    let mut pb = PathBuilder::new();
    if r <= 0.0 {
        pb.push_rect(Rect::from_xywh(x, y, w, h)?);
    } else {
        let k = 0.552_284_75 * r;
        pb.move_to(x + r, y);
        pb.line_to(x + w - r, y);
        pb.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
        pb.line_to(x + w, y + h - r);
        pb.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);
        pb.line_to(x + r, y + h);
        pb.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);
        pb.line_to(x, y + r);
        pb.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
        pb.close();
    }
    pb.finish()
}

fn draw_layer(pm: &mut Pixmap, l: &Layer) -> Result<(), RenderError> {
    if !l.visible || l.width <= 0.0 || l.height <= 0.0 {
        return Ok(());
    }
    let (cx, cy) = (l.x + l.width / 2.0, l.y + l.height / 2.0);
    let ts = Transform::from_rotate_at(l.rotation, cx, cy);

    match &l.kind {
        LayerKind::Rect { corner_radius } => {
            if let Some(path) = rounded_rect(l.x, l.y, l.width, l.height, *corner_radius) {
                pm.fill_path(&path, &paint_for(l.fill, l.opacity), FillRule::Winding, ts, None);
            }
        }
        LayerKind::Ellipse => {
            if let Some(rect) = Rect::from_xywh(l.x, l.y, l.width, l.height) {
                if let Some(path) = PathBuilder::from_oval(rect) {
                    pm.fill_path(&path, &paint_for(l.fill, l.opacity), FillRule::Winding, ts, None);
                }
            }
        }
        LayerKind::Image { width, height, rgba } => {
            if rgba.len() != (*width as usize) * (*height as usize) * 4 || *width == 0 || *height == 0 {
                return Err(RenderError::InvalidImage);
            }
            let mut src = Pixmap::new(*width, *height).ok_or(RenderError::InvalidImage)?;
            for (dst, px) in src.pixels_mut().iter_mut().zip(rgba.chunks_exact(4)) {
                *dst = tiny_skia::ColorU8::from_rgba(px[0], px[1], px[2], px[3]).premultiply();
            }
            let sx = l.width / *width as f32;
            let sy = l.height / *height as f32;
            let place = Transform::from_translate(l.x, l.y).pre_scale(sx, sy);
            let full = ts.pre_concat(place);
            let paint = PixmapPaint {
                opacity: l.opacity.clamp(0.0, 1.0),
                quality: FilterQuality::Bilinear,
                ..PixmapPaint::default()
            };
            pm.draw_pixmap(0, 0, src.as_ref(), &paint, full, None);
        }
    }
    Ok(())
}

/// Renders a page to a premultiplied RGBA pixmap.
pub fn render_page(page: &Page) -> Result<Pixmap, RenderError> {
    let mut pm = Pixmap::new(page.width, page.height).ok_or(RenderError::InvalidSize)?;
    let bg = page.background;
    pm.fill(tiny_skia::Color::from_rgba8(bg.r, bg.g, bg.b, bg.a));
    for layer in &page.layers {
        draw_layer(&mut pm, layer)?;
    }
    Ok(pm)
}

/// Renders a page and encodes it as PNG.
pub fn render_png(page: &Page) -> Result<Vec<u8>, RenderError> {
    render_page(page)?.encode_png().map_err(|e| RenderError::Encode(e.to_string()))
}
