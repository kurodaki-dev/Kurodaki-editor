//! Optional on-device background removal using a user-provided ONNX model (U²-Net style).
//! The model file is never downloaded by the app: the user places it on the device.

use std::path::Path;
use tract_onnx::prelude::*;

pub struct Segmenter {
    model: SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>,
    side: usize,
}

impl Segmenter {
    /// `side` is the square input size the model expects (320 for u2netp, 1024 for u2net).
    pub fn load(path: &Path, side: usize) -> TractResult<Self> {
        let model = tract_onnx::onnx()
            .model_for_path(path)?
            .with_input_fact(0, f32::fact([1, 3, side, side]).into())?
            .into_optimized()?
            .into_runnable()?;
        Ok(Segmenter { model, side })
    }

    /// Returns an 8-bit mask with `width * height` entries (255 = keep).
    pub fn mask(&self, rgba: &[u8], width: u32, height: u32) -> TractResult<Vec<u8>> {
        let s = self.side;
        let src = image::RgbaImage::from_raw(width, height, rgba.to_vec())
            .ok_or_else(|| anyhow::anyhow!("bad image buffer"))?;
        let small = image::imageops::resize(&src, s as u32, s as u32, image::imageops::FilterType::Triangle);
        let (mean, std) = ([0.485f32, 0.456, 0.406], [0.229f32, 0.224, 0.225]);
        let input = tract_ndarray::Array4::from_shape_fn((1, 3, s, s), |(_, c, y, x)| {
            (small.get_pixel(x as u32, y as u32)[c] as f32 / 255.0 - mean[c]) / std[c]
        });
        let out = self.model.run(tvec!(Tensor::from(input).into()))?;
        let view = out[0].to_array_view::<f32>()?;
        let vals: Vec<f32> = view.iter().copied().collect();
        if vals.len() < s * s {
            anyhow::bail!("unexpected model output size {}", vals.len());
        }
        let vals = &vals[vals.len() - s * s..];
        let (mn, mx) = vals.iter().fold((f32::MAX, f32::MIN), |(a, b), &v| (a.min(v), b.max(v)));
        let span = (mx - mn).max(1e-6);
        let small_mask = image::GrayImage::from_fn(s as u32, s as u32, |x, y| {
            image::Luma([(((vals[y as usize * s + x as usize] - mn) / span) * 255.0) as u8])
        });
        let full = image::imageops::resize(&small_mask, width, height, image::imageops::FilterType::Triangle);
        Ok(full.into_raw())
    }
}
