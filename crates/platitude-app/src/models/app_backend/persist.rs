//! Writing the window and its layout back to the settings store.

use super::*;

impl AppBackend {
    pub(super) fn write_window(&self, x: i32, y: i32, width: i32, height: i32, maximized: bool) {
        Hub::with(|hub| {
            let previous = hub.state().window;
            hub.set_window_state(platitude_core::settings::WindowState {
                // A maximized window reports the size of the screen. Keeping
                // the last unmaximized one is what lets restoring down go
                // back to a window rather than to a full screen.
                x: if maximized { previous.x } else { Some(x) },
                y: if maximized { previous.y } else { Some(y) },
                width: if maximized { previous.width } else { width },
                height: if maximized { previous.height } else { height },
                maximized,
            });
        });
    }

    pub(super) fn write_layout_sizes(
        &self,
        sidebar_width: i32,
        details_width: i32,
        commands_height: i32,
        graph_labels_width: i32,
        graph_lanes_width: i32,
    ) {
        Hub::with(|hub| {
            let mut layout = hub.state().layout;
            layout.sidebar_width = sidebar_width;
            layout.details_width = details_width;
            layout.commands_height = commands_height;
            layout.graph_labels_width = graph_labels_width;
            layout.graph_lanes_width = graph_lanes_width;
            hub.set_layout_state(layout);
        });
    }

    pub(super) fn write_layout_flags(
        &self,
        sidebar_collapsed: bool,
        commands_shown: bool,
        tags_shown: bool,
        wip_tree: bool,
        details_tree: bool,
    ) {
        Hub::with(|hub| {
            let mut layout = hub.state().layout;
            layout.sidebar_collapsed = sidebar_collapsed;
            layout.commands_shown = commands_shown;
            layout.tags_shown = tags_shown;
            layout.wip_tree = wip_tree;
            layout.details_tree = details_tree;
            hub.set_layout_state(layout);
        });
    }

    pub(super) fn write_sections(
        &self,
        branches: bool,
        remotes: bool,
        worktree: bool,
        stashes: bool,
        tags: bool,
    ) {
        Hub::with(|hub| {
            let mut layout = hub.state().layout;
            layout.sections = platitude_core::settings::Sections {
                branches,
                remotes,
                worktree,
                stashes,
                tags,
            };
            hub.set_layout_state(layout);
        });
    }
}
