use crate::document::{Layer, LayerKind};

/// Local, deterministic image adjustments. Nothing here touches the network.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {
    /// -1.0..=1.0
    Brightness(f32),
    /// -1.0..=1.0
    Contrast(f32),
    /// 0.0 = grey, 1.0 = unchanged, 2.0 = doubled
    Saturation(f32),
    Grayscale,
    Sepia,
    Invert,
    /// "Magic touch": stretches levels per channel and lifts mid-tones.
    AutoEnhance,
}

fn luma(r: f32, g: f32, b: f32) -> f32 {
    0.299 * r + 0.587 * g + 0.114 * b
}

fn clamp8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

fn auto_levels(rgba: &mut [u8]) {
    // Per-channel 1%/99% percentiles over non-transparent pixels.
    let mut hist = [[0u32; 256]; 3];
    let mut total = 0u32;
    for p in rgba.chunks_exact(4).filter(|p| p[3] > 0) {
        for c in 0..3 {
            hist[c][p[c] as usize] += 1;
        }
        total += 1;
    }
    if total == 0 {
        return;
    }
    let cut = total / 100;
    let mut lo = [0u8; 3];
    let mut hi = [255u8; 3];
    for c in 0..3 {
        let (mut acc, mut found) = (0u32, false);
        for v in 0..256 {
            acc += hist[c][v];
            if !found && acc > cut {
                lo[c] = v as u8;
                found = true;
            }
        }
        acc = 0;
        for v in (0..256).rev() {
            acc += hist[c][v];
            if acc > cut {
                hi[c] = v as u8;
                break;
            }
        }
    }
    for p in rgba.chunks_exact_mut(4) {
        for c in 0..3 {
            let (l, h) = (lo[c] as f32, (hi[c].max(lo[c] + 1)) as f32);
            let n = ((p[c] as f32 - l) / (h - l)).clamp(0.0, 1.0);
            p[c] = clamp8(n.powf(0.9) * 255.0);
        }
    }
}

/// Applies an effect to raw RGBA8 pixels in place.
pub fn apply_effect(rgba: &mut [u8], effect: Effect) {
    match effect {
        Effect::AutoEnhance => {
            auto_levels(rgba);
            apply_effect(rgba, Effect::Saturation(1.12));
        }
        Effect::Brightness(b) => {
            for p in rgba.chunks_exact_mut(4) {
                for c in 0..3 {
                    p[c] = clamp8(p[c] as f32 + b * 255.0);
                }
            }
        }
        Effect::Contrast(k) => {
            let f = 1.0 + k.clamp(-1.0, 1.0);
            for p in rgba.chunks_exact_mut(4) {
                for c in 0..3 {
                    p[c] = clamp8((p[c] as f32 - 128.0) * f + 128.0);
                }
            }
        }
        Effect::Saturation(s) => {
            for p in rgba.chunks_exact_mut(4) {
                let (r, g, b) = (p[0] as f32, p[1] as f32, p[2] as f32);
                let y = luma(r, g, b);
                p[0] = clamp8(y + (r - y) * s);
                p[1] = clamp8(y + (g - y) * s);
                p[2] = clamp8(y + (b - y) * s);
            }
        }
        Effect::Grayscale => apply_effect(rgba, Effect::Saturation(0.0)),
        Effect::Sepia => {
            for p in rgba.chunks_exact_mut(4) {
                let (r, g, b) = (p[0] as f32, p[1] as f32, p[2] as f32);
                p[0] = clamp8(0.393 * r + 0.769 * g + 0.189 * b);
                p[1] = clamp8(0.349 * r + 0.686 * g + 0.168 * b);
                p[2] = clamp8(0.272 * r + 0.534 * g + 0.131 * b);
            }
        }
        Effect::Invert => {
            for p in rgba.chunks_exact_mut(4) {
                for c in 0..3 {
                    p[c] = 255 - p[c];
                }
            }
        }
    }
}

/// Multiplies the alpha channel by an 8-bit mask of the same pixel count.
/// Returns false (and leaves the layer untouched) if sizes do not match or it is not an image.
pub fn apply_mask(layer: &mut Layer, mask: &[u8]) -> bool {
    match &mut layer.kind {
        LayerKind::Image { width, height, rgba } if mask.len() == (*width * *height) as usize => {
            for (p, m) in rgba.chunks_exact_mut(4).zip(mask) {
                p[3] = ((p[3] as u16 * *m as u16) / 255) as u8;
            }
            true
        }
        _ => false,
    }
}
