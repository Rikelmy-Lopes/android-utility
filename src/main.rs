mod adb;
mod commands;
mod i18n;
mod startup;
mod ui;

use ui::AndroidUtilityApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([640.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Android Utility",
        options,
        Box::new(|creation_context| Ok(Box::new(AndroidUtilityApp::new(creation_context)))),
    )
}
