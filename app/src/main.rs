use kurodaki_editor::EditorApp;

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Kurodaki Editor",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(EditorApp::default()))),
    )
}
