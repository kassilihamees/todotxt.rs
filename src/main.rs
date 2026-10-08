#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#[cfg(any(not(windows), feature = "portable-ui"))]
mod desktop;
#[cfg(windows)]
mod windows;
#[cfg(any(not(windows), feature = "portable-ui"))]
use eframe::egui;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|s| s == "--help" || s == "-h") {
        println!(
            "todotxt.rs [PATH | --demo] [--config-dir DIRECTORY] [--portable-ui]\n\nWith no PATH, reopen the last file. On first run, use a private copy of the sample.\n--demo opens that private sample copy. --config-dir isolates settings for testing.\nWindows defaults to native Win32 controls. --portable-ui selects the optional\nportable frontend (build with --features portable-ui on Windows)."
        );
        return Ok(());
    }
    let (mut path, mut config, mut demo, mut portable) = (None, None, false, false);
    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--demo" => demo = true,
            "--portable-ui" => portable = true,
            "--config-dir" => config = iter.next().map(std::path::PathBuf::from),
            "--file" => path = iter.next().map(std::path::PathBuf::from),
            _ => path = Some(arg.into()),
        }
    }
    #[cfg(windows)]
    if !portable {
        return windows::run(path, config, demo).map_err(Into::into);
    }
    #[cfg(all(windows, not(feature = "portable-ui")))]
    return Err("Build with --features portable-ui to use --portable-ui on Windows.".into());
    #[cfg(not(windows))]
    let _ = portable;
    #[cfg(any(not(windows), feature = "portable-ui"))]
    portable_run(path, config, demo).map_err(Into::into)
}
#[cfg(any(not(windows), feature = "portable-ui"))]
fn portable_run(
    path: Option<std::path::PathBuf>,
    config: Option<std::path::PathBuf>,
    demo: bool,
) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([506.0, 1006.0])
            .with_min_inner_size([350.0, 200.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/todotxt.png"))
                    .expect("application icon"),
            ),
        persistence_path: config.as_ref().map(|p| p.join("window.ron")),
        ..Default::default()
    };
    eframe::run_native(
        "todotxt.rs",
        options,
        Box::new(move |cc| Ok(Box::new(desktop::Desktop::new(cc, path, config, demo)))),
    )
}
