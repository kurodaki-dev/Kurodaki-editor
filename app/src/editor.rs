use egui::{Color32, Pos2, Sense, Vec2};
use kurodaki_core::{
    apply_effect, apply_mask, decode_image, render_page, render_png, segment::Segmenter, Color,
    Document, Effect, Layer, LayerKind,
};
use std::path::PathBuf;

const IMAGE_EXT: [&str; 4] = ["png", "jpg", "jpeg", "webp"];
const MODEL_SIDE: usize = 320;

fn storage() -> PathBuf {
    PathBuf::from(std::env::var("EXTERNAL_STORAGE").unwrap_or_else(|_| "/sdcard".into()))
}

fn list_images() -> Vec<PathBuf> {
    let root = storage();
    let mut out = Vec::new();
    for dir in ["Pictures", "DCIM/Camera", "Download", "Kurodaki"] {
        if let Ok(rd) = std::fs::read_dir(root.join(dir)) {
            for e in rd.flatten() {
                let p = e.path();
                let ok = p
                    .extension()
                    .and_then(|x| x.to_str())
                    .map(|x| IMAGE_EXT.contains(&x.to_lowercase().as_str()))
                    .unwrap_or(false);
                if ok {
                    out.push(p);
                }
            }
        }
    }
    out.sort();
    out
}

pub struct EditorApp {
    doc: Document,
    selected: Option<usize>,
    texture: Option<egui::TextureHandle>,
    dirty: bool,
    status: String,
    picker: Option<Vec<PathBuf>>,
    segmenter: Option<Segmenter>,
}

impl Default for EditorApp {
    fn default() -> Self {
        let mut doc = Document::new("Sans titre", 1080, 1080);
        doc.pages[0].background = Color::rgb(245, 245, 250);
        EditorApp { doc, selected: None, texture: None, dirty: true, status: String::new(), picker: None, segmenter: None }
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
        let path = storage().join("Kurodaki/export.png");
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        self.status = match render_png(&self.doc.pages[0]).map(|b| std::fs::write(&path, b)) {
            Ok(Ok(())) => format!("Exporté : {}", path.display()),
            Ok(Err(e)) => format!("Écriture impossible : {e}"),
            Err(e) => format!("Export impossible : {e}"),
        };
    }

    fn import(&mut self, path: &std::path::Path) {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("Image").to_string();
        let page_w = self.doc.pages[0].width as f32;
        self.status = match std::fs::read(path) {
            Err(e) => format!("Lecture impossible : {e} (autorise l'accès aux photos dans les réglages Android)"),
            Ok(bytes) => match decode_image(&name, &bytes, 2048) {
                Err(e) => e.to_string(),
                Ok(mut layer) => {
                    let k = (page_w * 0.6 / layer.width).min(1.0);
                    layer.width *= k;
                    layer.height *= k;
                    layer.x = 100.0;
                    layer.y = 100.0;
                    self.add(layer);
                    format!("Importé : {name}")
                }
            },
        };
    }

    fn effect_on_selected(&mut self, effect: Effect) {
        let Some(i) = self.selected else { return };
        if let Some(Layer { kind: LayerKind::Image { rgba, .. }, .. }) = self.doc.pages[0].layers.get_mut(i) {
            apply_effect(rgba, effect);
            self.dirty = true;
        } else {
            self.status = "Sélectionne d'abord un calque image.".into();
        }
    }

    fn remove_background(&mut self) {
        let Some(i) = self.selected else { return };
        if self.segmenter.is_none() {
            let model = storage().join("Kurodaki/models/u2netp.onnx");
            match Segmenter::load(&model, MODEL_SIDE) {
                Ok(s) => self.segmenter = Some(s),
                Err(e) => {
                    self.status = format!("Modèle absent ou invalide ({}): {e}", model.display());
                    return;
                }
            }
        }
        let seg = self.segmenter.as_ref().unwrap();
        let Some(layer) = self.doc.pages[0].layers.get_mut(i) else { return };
        let LayerKind::Image { width, height, rgba } = &layer.kind else {
            self.status = "Sélectionne d'abord un calque image.".into();
            return;
        };
        match seg.mask(rgba, *width, *height) {
            Ok(mask) => {
                apply_mask(layer, &mask);
                self.dirty = true;
                self.status = "Fond retiré (IA locale).".into();
            }
            Err(e) => self.status = format!("IA locale : {e}"),
        }
    }

    fn save_project(&mut self) {
        let path = storage().join("Kurodaki/projet.kuro.json");
        let res = std::fs::create_dir_all(path.parent().unwrap())
            .map_err(|e| e.to_string())
            .and_then(|_| self.doc.to_json().map_err(|e| e.to_string()))
            .and_then(|j| std::fs::write(&path, j).map_err(|e| e.to_string()));
        self.status = match res {
            Ok(()) => format!("Projet enregistré : {}", path.display()),
            Err(e) => format!("Sauvegarde impossible : {e}"),
        };
    }

    fn open_project(&mut self) {
        let path = storage().join("Kurodaki/projet.kuro.json");
        match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|s| Document::from_json(&s).map_err(|e| e.to_string())) {
            Ok(doc) if !doc.pages.is_empty() => {
                self.doc = doc;
                self.selected = None;
                self.dirty = true;
                self.status = "Projet ouvert.".into();
            }
            Ok(_) => self.status = "Projet vide.".into(),
            Err(e) => self.status = format!("Ouverture impossible : {e}"),
        }
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
                if ui.button("🖼 Importer").clicked() {
                    self.picker = Some(list_images());
                }
                if ui.button("✨ Magic Touch").clicked() {
                    self.effect_on_selected(Effect::AutoEnhance);
                }
                if ui.button("✂ Retirer le fond").clicked() {
                    self.remove_background();
                }
                if ui.button("💾 Enregistrer").clicked() {
                    self.save_project();
                }
                if ui.button("📂 Ouvrir").clicked() {
                    self.open_project();
                }
                if ui.button("⬇ Exporter PNG").clicked() {
                    self.export();
                }
                ui.label(&self.status);
            });
        });

        let mut chosen: Option<PathBuf> = None;
        let mut close = false;
        if let Some(files) = &self.picker {
            egui::Window::new("Choisir une image").collapsible(false).show(ctx, |ui| {
                if files.is_empty() {
                    ui.label("Aucune image trouvée dans Pictures, DCIM/Camera, Download ou Kurodaki. Vérifie l'autorisation Photos.");
                }
                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    for f in files {
                        let label = f.strip_prefix(storage()).unwrap_or(f).display().to_string();
                        if ui.button(label).clicked() {
                            chosen = Some(f.clone());
                        }
                    }
                });
                if ui.button("Fermer").clicked() {
                    close = true;
                }
            });
        }
        if let Some(f) = chosen {
            self.picker = None;
            self.import(&f);
        } else if close {
            self.picker = None;
        }

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
                let mut effect: Option<Effect> = None;
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
                if matches!(l.kind, LayerKind::Image { .. }) {
                    ui.separator();
                    ui.label("Retouches locales");
                    ui.horizontal_wrapped(|ui| {
                        for (name, e) in [
                            ("N&B", Effect::Grayscale),
                            ("Sépia", Effect::Sepia),
                            ("Inverser", Effect::Invert),
                            ("+ Contraste", Effect::Contrast(0.15)),
                            ("+ Lumière", Effect::Brightness(0.06)),
                            ("+ Couleurs", Effect::Saturation(1.2)),
                        ] {
                            if ui.small_button(name).clicked() {
                                effect = Some(e);
                            }
                        }
                    });
                }
                let mut c = to_c32(l.fill);
                if ui.color_edit_button_srgba(&mut c).changed() {
                    l.fill = Color::rgb(c.r(), c.g(), c.b());
                    changed = true;
                }
                if let (Some(e), LayerKind::Image { rgba, .. }) = (effect, &mut l.kind) {
                    apply_effect(rgba, e);
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
