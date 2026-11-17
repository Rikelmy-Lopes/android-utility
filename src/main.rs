mod adb;
mod commands;
mod i18n;
mod logging;
mod startup;
mod ui;

use ui::AndroidUtilityApp;

fn main() -> eframe::Result<()> {
    let log_directory = logging::init();

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        log_directory = %log_directory
            .as_deref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "unavailable".to_owned()),
        "starting Android Utility"
    );

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([640.0, 480.0]),
        ..Default::default()
    };

    let result = eframe::run_native(
        "Android Utility",
        options,
        Box::new(|creation_context| Ok(Box::new(AndroidUtilityApp::new(creation_context)))),
    );

    match &result {
        Ok(()) => tracing::info!("application stopped"),
        Err(error) => tracing::error!(%error, "application stopped with an error"),
    }

    result
}
