//! Kurodaki core: a small layered design document, a software renderer and exporters.

mod document;
mod render;

pub use document::{Color, Document, Layer, LayerKind, Page};
pub use render::{render_page, render_png, RenderError};
