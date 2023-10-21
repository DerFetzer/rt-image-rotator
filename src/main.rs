mod app;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        initial_window_size: Some([400.0, 300.0].into()),
        min_window_size: Some([300.0, 220.0].into()),
        ..Default::default()
    };
    eframe::run_native(
        "Rawtherapee Image Rotator",
        native_options,
        Box::new(|cc| Box::new(app::RtImageRotator::new(cc))),
    )
}
