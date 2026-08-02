//! platitude-gg entry point: tokio runtime + hub + QML application.

// Hide the console window of the GUI subsystem build on Windows (debug runs
// from a terminal still show logs on stderr).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod encode;
mod hub;
mod models;
mod urlpath;

use hub::Hub;
use models::{
    AppBackend, DetailsModel, DiffModel, GraphModel, RepoTab, SidebarModel, StashModel, TabsModel,
    WorkTreeModel,
};
use qtbridge::QApp;

fn main() {
    init_tracing();

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!(error = %e, "failed to start the async runtime");
            std::process::exit(1);
        }
    };
    Hub::install(runtime);

    let code = QApp::new()
        .application_name("platitude-gg")
        .register::<AppBackend>()
        .register::<TabsModel>()
        .register::<RepoTab>()
        .register::<GraphModel>()
        .register::<SidebarModel>()
        .register::<WorkTreeModel>()
        .register::<StashModel>()
        .register::<DetailsModel>()
        .register::<DiffModel>()
        .load_qml(include_bytes!("Main.qml"))
        .run();

    Hub::shutdown();
    std::process::exit(code);
}

/// stderr logging; level via `PG_LOG` (error/warn/info/debug/trace).
fn init_tracing() {
    let level = match std::env::var("PG_LOG").as_deref() {
        Ok("error") => tracing::Level::ERROR,
        Ok("info") => tracing::Level::INFO,
        Ok("debug") => tracing::Level::DEBUG,
        Ok("trace") => tracing::Level::TRACE,
        _ => tracing::Level::WARN,
    };
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_writer(std::io::stderr)
        .init();
}
