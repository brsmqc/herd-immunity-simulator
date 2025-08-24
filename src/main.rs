// Destop
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Herd Immunity Simulator")
            .with_inner_size([1035.0, 600.0])
            .with_min_inner_size([800.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Herd Immunity Simulator",
        native_options,
        Box::new(|cc| Ok(Box::new(herd_immunity_simulator::App::new(cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {}
