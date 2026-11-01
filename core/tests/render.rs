use kurodaki_core::*;

fn px(pm: &tiny_skia::Pixmap, x: u32, y: u32) -> [u8; 4] {
    let p = pm.pixel(x, y).unwrap().demultiply();
    [p.red(), p.green(), p.blue(), p.alpha()]
}

#[test]
fn background_and_rect() {
    let mut page = Page::new(100, 100);
    page.layers.push(Layer::rect("r", 10.0, 10.0, 50.0, 50.0, Color::rgb(255, 0, 0)));
    let pm = render_page(&page).unwrap();
    assert_eq!(px(&pm, 0, 0), [255, 255, 255, 255]);
    assert_eq!(px(&pm, 30, 30), [255, 0, 0, 255]);
}

#[test]
fn ellipse_corners_stay_empty() {
    let mut page = Page::new(100, 100);
    page.layers.push(Layer::ellipse("e", 0.0, 0.0, 100.0, 100.0, Color::BLACK));
    let pm = render_page(&page).unwrap();
    assert_eq!(px(&pm, 50, 50), [0, 0, 0, 255]);
    assert_eq!(px(&pm, 1, 1), [255, 255, 255, 255]);
}

#[test]
fn hidden_layers_and_opacity() {
    let mut page = Page::new(20, 20);
    let mut l = Layer::rect("r", 0.0, 0.0, 20.0, 20.0, Color::BLACK);
    l.visible = false;
    page.layers.push(l);
    assert_eq!(px(&render_page(&page).unwrap(), 5, 5), [255, 255, 255, 255]);
    page.layers[0].visible = true;
    page.layers[0].opacity = 0.5;
    let p = px(&render_page(&page).unwrap(), 5, 5);
    assert!((p[0] as i32 - 127).abs() <= 2);
}

#[test]
fn image_layer_and_bad_data() {
    let mut page = Page::new(10, 10);
    page.layers.push(Layer::image("i", 0.0, 0.0, 1, 1, vec![0, 255, 0, 255]));
    page.layers[0].width = 10.0;
    page.layers[0].height = 10.0;
    assert_eq!(px(&render_page(&page).unwrap(), 5, 5), [0, 255, 0, 255]);
    page.layers[0].kind = LayerKind::Image { width: 2, height: 2, rgba: vec![0; 3] };
    assert!(matches!(render_page(&page), Err(RenderError::InvalidImage)));
}

#[test]
fn hit_test_respects_order_and_rotation() {
    let mut page = Page::new(200, 200);
    page.layers.push(Layer::rect("a", 0.0, 0.0, 100.0, 100.0, Color::BLACK));
    page.layers.push(Layer::rect("b", 50.0, 50.0, 100.0, 100.0, Color::BLACK));
    assert_eq!(page.hit_test(75.0, 75.0), Some(1));
    assert_eq!(page.hit_test(10.0, 10.0), Some(0));
    assert_eq!(page.hit_test(190.0, 190.0), None);
    let mut thin = Layer::rect("t", 0.0, 90.0, 200.0, 20.0, Color::BLACK);
    thin.rotation = 90.0;
    assert!(thin.contains(100.0, 5.0));
    assert!(!thin.contains(5.0, 100.0));
}

#[test]
fn json_roundtrip_and_png() {
    let mut doc = Document::new("demo", 64, 64);
    doc.pages[0].layers.push(Layer::rect("r", 1.0, 2.0, 3.0, 4.0, Color::rgb(1, 2, 3)));
    let back = Document::from_json(&doc.to_json().unwrap()).unwrap();
    assert_eq!(doc, back);
    let png = render_png(&doc.pages[0]).unwrap();
    assert_eq!(&png[1..4], b"PNG");
    assert!(matches!(render_page(&Page::new(0, 0)), Err(RenderError::InvalidSize)));
}

#[test]
fn effects_and_mask() {
    let mut px = vec![200u8, 100, 50, 255];
    apply_effect(&mut px, Effect::Invert);
    assert_eq!(px, vec![55, 155, 205, 255]);
    let mut g = vec![255u8, 0, 0, 255];
    apply_effect(&mut g, Effect::Grayscale);
    assert!(g[0] == g[1] && g[1] == g[2] && g[3] == 255);
    let mut dull: Vec<u8> = (0..16).flat_map(|i| [80 + i, 80 + i, 80 + i, 255]).collect();
    apply_effect(&mut dull, Effect::AutoEnhance);
    let (mn, mx) = (dull[0], dull[60]);
    assert!(mx as i32 - mn as i32 > 150);

    let mut layer = Layer::image("i", 0.0, 0.0, 2, 1, vec![9, 9, 9, 255, 9, 9, 9, 255]);
    assert!(apply_mask(&mut layer, &[255, 0]));
    assert!(!apply_mask(&mut layer, &[1, 2, 3]));
    if let LayerKind::Image { rgba, .. } = &layer.kind {
        assert_eq!((rgba[3], rgba[7]), (255, 0));
    }
}

#[test]
fn decode_png_and_reject_garbage() {
    let mut buf = Vec::new();
    let img = image::RgbaImage::from_pixel(8, 4, image::Rgba([1, 2, 3, 255]));
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png).unwrap();
    let l = decode_image("x", &buf, 4).unwrap();
    assert!(l.width <= 4.0 && l.height <= 4.0);
    assert!(decode_image("x", b"not an image", 100).is_err());
}
