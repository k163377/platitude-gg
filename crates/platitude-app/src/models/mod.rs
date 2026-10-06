//! QML-facing objects: a thin bridge mapping platitude-core results onto Qt
//! models and properties (swappable, 実装計画 §2).
//!
//! - per-tab objects wire themselves with `attach(tabId)` and consume their
//!   feed in `drain` (the only slot ever called through an invoker)

mod app_backend;
mod clone;
mod commands;
mod details;
mod diff;
#[cfg(test)]
mod diff_tests;
mod discards;
mod facts;
mod graph;
mod line_endings;
mod nav;
mod notify;
mod pathtree;
mod rebase_plan;
mod repo_config;
mod repo_tab;
mod tab_name;
mod tabs;
#[cfg(test)]
mod tabs_tests;
mod working_tree;

pub use app_backend::{AppBackend, build_id, build_tree};
pub use clone::CloneModel;
pub use commands::CommandsModel;
pub use details::DetailsModel;
pub use diff::DiffModel;
pub use discards::DiscardModel;
pub use facts::GitFacts;
pub use graph::GraphModel;
pub use line_endings::LineEndingsModel;
pub use nav::NavSectionModel;
pub use rebase_plan::RebasePlanModel;
pub use repo_config::RepoConfigModel;
pub use repo_tab::RepoTab;
pub use tabs::TabsModel;
pub use working_tree::WorkingTreeModel;

pub(crate) use notify::{impl_extend_notified, impl_move_notified, impl_notify_runs, push_run};

/// Registers a type under the `platitude` QML module.
macro_rules! qml_register {
    ($ty:ty, $name:literal, singleton = $singleton:literal) => {
        impl qtbridge::QmlElement for $ty {
            const URI: &str = "platitude";
            const ELEMENT_NAME: &str = $name;
            const MAJOR_VERSION: u8 = 1;
            const MINOR_VERSION: u8 = 0;
            const IS_SINGLETON: bool = $singleton;
        }
    };
}
pub(crate) use qml_register;
