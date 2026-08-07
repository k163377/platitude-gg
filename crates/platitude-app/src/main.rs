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
    AppBackend, CommandsModel, DetailsModel, DiffModel, GraphModel, NavSectionModel, RepoTab,
    TabsModel, WorkTreeModel,
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

    let mut app = QApp::new();
    app.application_name("platitude-gg");
    // Embed the QML module (singletons + components + Main) as qrc
    // resources. Every file listed in ui/qmldir must be embedded here.
    qtbridge::include_bytes_qml!("ui/qmldir", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/Theme.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/Metrics.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/ActionButton.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/AppDialog.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/AppMenu.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/AppMenuItem.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/AppMenuSeparator.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/AskBar.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/AutoScrollBar.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/ChangeIcon.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/ClipboardHelper.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/CommandRowDelegate.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/CommandsPane.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/DetailsPane.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/DiffPane.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/DirtySwitchDialog.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/FileRowDelegate.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/FormField.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/GraphPane.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/GraphRowDelegate.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/HoldIcon.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/HoverButton.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/HoverToolButton.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/IdentIcon.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/IdentityDialog.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/ImagePreviewCell.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/NavHeader.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/NavIcon.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/NavItemDelegate.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/NavList.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/PaneHeader.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/RefChip.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/RefListPopup.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/RepoPage.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SettingsDialog.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SidebarPane.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SlimField.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/SummaryArea.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/TopBar.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/WipPane.qml", "qt/qml/platitude");
    qtbridge::include_bytes_qml!("ui/Main.qml", "qt/qml/platitude");
    let code = app
        .register::<AppBackend>()
        .register::<TabsModel>()
        .register::<RepoTab>()
        .register::<GraphModel>()
        .register::<NavSectionModel>()
        .register::<WorkTreeModel>()
        .register::<DetailsModel>()
        .register::<DiffModel>()
        .register::<CommandsModel>()
        .add_import_path("qrc:/qt/qml")
        .load_qml_from_file("qrc:/qt/qml/platitude/ui/Main.qml")
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
