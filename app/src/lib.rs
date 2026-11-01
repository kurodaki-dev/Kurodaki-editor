mod editor;

pub use editor::EditorApp;

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info));
    let options = eframe::NativeOptions {
        event_loop_builder: Some(Box::new(move |builder| {
            builder.with_android_app(app);
        })),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "Kurodaki Editor",
        options,
        Box::new(|_cc| Ok(Box::new(EditorApp::default()))),
    );
}
