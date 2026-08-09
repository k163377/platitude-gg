//! QML-facing objects. This layer only maps platitude-core results onto Qt
//! models/properties (thin bridge, swappable per 実装計画 §2.1).
//!
//! Conventions:
//! - every object is registered under the `platitude` QML module via the
//!   `qml_register!` macro (NoQmlElement + manual QmlRegister)
//! - per-tab objects wire themselves with `attach(tabId)` and consume their
//!   feed in `drain` (the only slot ever called through an invoker)
//! - `#[derive(QModelItem)]` requires `HashMap` in scope (macro hygiene).

mod app_backend;
mod commands;
mod details;
mod diff;
mod graph;
mod nav;
mod repo_tab;
mod tabs;
mod worktree;

pub use app_backend::{AppBackend, build_tree};
pub use commands::CommandsModel;
pub use details::DetailsModel;
pub use diff::DiffModel;
pub use graph::GraphModel;
pub use nav::NavSectionModel;
pub use repo_tab::RepoTab;
pub use tabs::TabsModel;
pub use worktree::WorkTreeModel;

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

/// Batch append with one begin/endInsertRows pair (qtbridge has no batch
/// insert; this mirrors QListModelBase::push via the public proxy API —
/// see .claude/rules/app-ui.md "Qt Bridges の要点(罠)").
macro_rules! impl_extend_notified {
    ($ty:ty, $field:ident, $item:ty) => {
        impl $ty {
            #[expect(unsafe_code)]
            fn extend_notified(&mut self, batch: Vec<$item>) {
                if batch.is_empty() {
                    return;
                }
                let Some(proxy) = self.try_get_rust_proxy_ptr() else {
                    self.$field.extend(batch);
                    return;
                };
                let first = self.$field.len() as i32;
                let last = first + batch.len() as i32 - 1;
                // SAFETY: same pattern as QListModelBase::push — the proxy
                // pointer stays valid while the QObject side is attached,
                // and we are on the Qt main thread inside a slot.
                unsafe { &mut *proxy }.base_begin_insert_rows(
                    &mut *self,
                    &qtbridge::qtbridge_type_lib::QModelIndex::default(),
                    first,
                    last,
                );
                self.$field.extend(batch);
                // SAFETY: see above.
                unsafe { &mut *proxy }.base_end_insert_rows(&mut *self);
            }
        }
    };
}
pub(crate) use impl_extend_notified;
