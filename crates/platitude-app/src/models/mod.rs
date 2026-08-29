//! QML-facing objects. This layer only maps platitude-core results onto Qt
//! models/properties (thin bridge, swappable per 実装計画 §2).
//!
//! Conventions:
//! - every object is registered under the `platitude` QML module via the
//!   `qml_register!` macro (NoQmlElement + manual QmlRegister)
//! - per-tab objects wire themselves with `attach(tabId)` and consume their
//!   feed in `drain` (the only slot ever called through an invoker)
//! - `#[derive(QModelItem)]` requires `HashMap` in scope (macro hygiene).

mod app_backend;
mod clone;
mod commands;
mod details;
mod diff;
#[cfg(test)]
mod diff_tests;
mod facts;
mod graph;
mod nav;
mod notify;
mod pathtree;
mod repo_tab;
mod tab_name;
mod tabs;
#[cfg(test)]
mod tabs_tests;
mod worktree;

pub use app_backend::{AppBackend, build_tree};
pub use clone::CloneModel;
pub use commands::CommandsModel;
pub use details::DetailsModel;
pub use diff::DiffModel;
pub use facts::GitFacts;
pub use graph::GraphModel;
pub use nav::NavSectionModel;
pub use repo_tab::RepoTab;
pub use tabs::TabsModel;
pub use worktree::WorkTreeModel;

pub(crate) use notify::{impl_extend_notified, impl_move_notified, impl_notify_runs, push_run};

/// Registers a type under the `platitude` QML module.
macro_rules! qml_register {
    ($ty:ty, $name:literal, singleton = $singleton:literal) => {
        impl qtbridge::QmlRegister for $ty {
            const URI: &str = "platitude";
            const ELEMENT_NAME: &str = $name;
            const MAJOR_VERSION: u8 = 1;
            const MINOR_VERSION: u8 = 0;
            const IS_SINGLETON: bool = $singleton;
        }
    };
}
pub(crate) use qml_register;
