use egui::{Color32, Pos2, Sense, Vec2};
use kurodaki_core::{render_page, render_png, Color, Document, Layer, LayerKind};

pub struct EditorApp {
    doc: Document,
    selected: Option<usize>,
    texture: Option<egui::TextureHandle>,
    dirty: bool,
    status: String,
}

impl Default for EditorApp {
    fn default() -> Self {
        let mut doc = Document::new("Sans titre", 1080, 1080);
        doc.pages[0].background = Color::rgb(245, 245, 250);
        EditorApp { doc, selected: None, texture: None, dirty: true, status: String::new() }
    }
}

fn to_c32(c: Color) -> Color32 {
    Color32::from_rgb(c.r, c.g, c.b)
}

impl EditorApp {
    fn refresh_texture(&mut self, ctx: &egui::Context) {
        if !self.dirty {
            return;
        }
        self.dirty = false;
        match render_page(&self.doc.pages[0]) {
            Ok(pm) => {
                let size = [pm.width() as usize, pm.height() as usize];
                let rgba: Vec<u8> = pm
                    .pixels()
                    .iter()
                    .flat_map(|p| {
                        let d = p.demultiply();
                        [d.red(), d.green(), d.blue(), d.alpha()]
                    })
                    .collect();
                let img = egui::ColorImage::from_rgba_unmultiplied(size, &rgba);
                self.texture = Some(ctx.load_texture("page", img, egui::TextureOptions::LINEAR));
            }
            Err(e) => self.status = format!("Rendu impossible : {e}"),
        }
    }

    fn export(&mut self) {
        let dir = std::env::var("EXTERNAL_STORAGE").unwrap_or_else(|_| ".".into());
        let path = std::path::Path::new(&dir).join("kurodaki-export.png");
        self.status = match render_png(&self.doc.pages[0]).map(|b| std::fs::write(&path, b)) {
            Ok(Ok(())) => format!("Exporté : {}", path.display()),
            Ok(Err(e)) => format!("Écriture impossible : {e}"),
            Err(e) => format!("Export impossible : {e}"),
        };
    }

    fn add(&mut self, layer: Layer) {
        let page = &mut self.doc.pages[0];
        page.layers.push(layer);
        self.selected = Some(page.layers.len() - 1);
        self.dirty = true;
    }
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("tools").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("▭ Rectangle").clicked() {
                    self.add(Layer::rect("Rectangle", 200.0, 200.0, 360.0, 240.0, Color::rgb(90, 120, 240)));
                }
                if ui.button("◯ Ellipse").clicked() {
                    self.add(Layer::ellipse("Ellipse", 300.0, 300.0, 300.0, 300.0, Color::rgb(240, 110, 90)));
                }
                if ui.button("⬇ Exporter PNG").clicked() {
                    self.export();
                }
                ui.label(&self.status);
            });
        });

        egui::SidePanel::right("props").min_width(200.0).show(ctx, |ui| {
            ui.heading("Calques");
            let page = &mut self.doc.pages[0];
            let mut action: Option<(usize, usize)> = None;
            let mut delete: Option<usize> = None;
            for i in (0..page.layers.len()).rev() {
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut page.layers[i].visible, "").changed() {
                        self.dirty = true;
                    }
                    let name = page.layers[i].name.clone();
                    if ui.selectable_label(self.selected == Some(i), name).clicked() {
                        self.selected = Some(i);
                    }
                    if ui.small_button("▲").clicked() && i + 1 < page.layers.len() {
                        action = Some((i, i + 1));
                    }
                    if ui.small_button("▼").clicked() && i > 0 {
                        action = Some((i, i - 1));
                    }
                    if ui.small_button("🗑").clicked() {
                        delete = Some(i);
                    }
                });
            }
            if let Some((from, to)) = action {
                page.move_layer(from, to);
                self.selected = Some(to);
                self.dirty = true;
            }
            if let Some(i) = delete {
                page.layers.remove(i);
                self.selected = None;
                self.dirty = true;
            }
            ui.separator();
            if let Some(i) = self.selected.filter(|&i| i < page.layers.len()) {
                let l = &mut page.layers[i];
                let mut changed = false;
                ui.text_edit_singleline(&mut l.name);
                changed |= ui.add(egui::Slider::new(&mut l.x, -500.0..=1500.0).text("x")).changed();
                changed |= ui.add(egui::Slider::new(&mut l.y, -500.0..=1500.0).text("y")).changed();
                changed |= ui.add(egui::Slider::new(&mut l.width, 1.0..=1500.0).text("largeur")).changed();
                changed |= ui.add(egui::Slider::new(&mut l.height, 1.0..=1500.0).text("hauteur")).changed();
                changed |= ui.add(egui::Slider::new(&mut l.rotation, -180.0..=180.0).text("rotation")).changed();
                changed |= ui.add(egui::Slider::new(&mut l.opacity, 0.0..=1.0).text("opacité")).changed();
                if let LayerKind::Rect { corner_radius } = &mut l.kind {
                    changed |= ui.add(egui::Slider::new(corner_radius, 0.0..=300.0).text("arrondi")).changed();
                }
                let mut c = to_c32(l.fill);
                if ui.color_edit_button_srgba(&mut c).changed() {
                    l.fill = Color::rgb(c.r(), c.g(), c.b());
                    changed = true;
                }
                self.dirty |= changed;
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.refresh_texture(ctx);
            let (pw, ph) = (self.doc.pages[0].width as f32, self.doc.pages[0].height as f32);
            let avail = ui.available_size();
            let scale = (avail.x / pw).min(avail.y / ph).max(0.05);
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(pw * scale, ph * scale), Sense::click_and_drag());
            if let Some(tex) = &self.texture {
                ui.painter().image(
                    tex.id(),
                    rect,
                    egui::Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            let to_page = |p: Pos2| ((p.x - rect.min.x) / scale, (p.y - rect.min.y) / scale);
            if resp.drag_started() || resp.clicked() {
                if let Some(p) = resp.interact_pointer_pos() {
                    let (x, y) = to_page(p);
                    self.selected = self.doc.pages[0].hit_test(x, y);
                }
            }
            if resp.dragged() {
                if let Some(i) = self.selected.filter(|&i| i < self.doc.pages[0].layers.len()) {
                    let d = resp.drag_delta() / scale;
                    let l = &mut self.doc.pages[0].layers[i];
                    l.x += d.x;
                    l.y += d.y;
                    self.dirty = true;
                }
            }
            if let Some(i) = self.selected.filter(|&i| i < self.doc.pages[0].layers.len()) {
                let l = &self.doc.pages[0].layers[i];
                let r = egui::Rect::from_min_size(
                    rect.min + Vec2::new(l.x, l.y) * scale,
                    Vec2::new(l.width, l.height) * scale,
                );
                ui.painter().rect_stroke(r, 0.0, egui::Stroke::new(2.0_f32, Color32::from_rgb(0, 150, 255)));
            }
        });
    }
}
