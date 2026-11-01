//! Kurodaki core: a small layered design document, a software renderer and exporters.

mod document;
mod effects;
mod import;
mod render;
#[cfg(feature = "onnx")]
pub mod segment;

pub use document::{Color, Document, Layer, LayerKind, Page};
pub use render::{render_page, render_png, RenderError};
pub use effects::{apply_effect, apply_mask, Effect};
pub use import::{decode_image, ImportError};
