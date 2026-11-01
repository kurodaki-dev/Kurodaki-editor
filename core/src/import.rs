use crate::document::Layer;

#[derive(Debug)]
pub enum ImportError {
    Io(String),
    Decode(String),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::Io(e) => write!(f, "lecture impossible : {e}"),
            ImportError::Decode(e) => write!(f, "image invalide : {e}"),
        }
    }
}

impl std::error::Error for ImportError {}

/// Decodes PNG/JPEG/WebP bytes into an image layer, scaled down to fit `max_side` if larger.
pub fn decode_image(name: &str, bytes: &[u8], max_side: u32) -> Result<Layer, ImportError> {
    let mut img = image::load_from_memory(bytes).map_err(|e| ImportError::Decode(e.to_string()))?;
    if img.width().max(img.height()) > max_side {
        img = img.resize(max_side, max_side, image::imageops::FilterType::Triangle);
    }
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Ok(Layer::image(name, 0.0, 0.0, w, h, rgba.into_raw()))
}
