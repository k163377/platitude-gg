//! `state.toml`: what the last session left behind.

use toml::{Table, Value};

use super::SCHEMA_VERSION;
use super::toml::{coord, flag, int_in, repo_key, sub_table, width_or_auto};

/// Where the window was left. Position is optional because "never saved"
/// has to stay distinguishable from "saved at 0,0" — the first should let
/// the window manager place the window, the second should not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowState {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: i32,
    pub height: i32,
    pub maximized: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            x: None,
            y: None,
            width: 1440,
            height: 900,
            maximized: false,
        }
    }
}

/// Which sidebar sections are expanded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sections {
    pub branches: bool,
    pub remotes: bool,
    pub worktree: bool,
    pub stashes: bool,
    pub tags: bool,
}

impl Default for Sections {
    fn default() -> Self {
        Self {
            branches: true,
            remotes: true,
            worktree: true,
            stashes: true,
            tags: true,
        }
    }
}

/// One set for the whole application, not one per tab. Persisting per tab
/// would make the layout jump on every tab switch and then keep doing it
/// across restarts; what is worth remembering is the layout that was last
/// settled on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutState {
    pub sidebar_width: i32,
    pub sidebar_collapsed: bool,
    pub details_width: i32,
    /// Chip and lane columns inside the graph, or [`AUTO_WIDTH`] while
    /// nobody has moved that divider — which is worth keeping apart from
    /// a number, because a session that never touched it goes on
    /// following the pane's default even when that default changes.
    pub graph_labels_width: i32,
    pub graph_lanes_width: i32,
    /// How tall the log stands, never whether it was standing: the log is
    /// where a command's answer is read, so a session that left it up has
    /// no claim on the next one's screen.
    pub commands_height: i32,
    pub tags_shown: bool,
    pub wip_tree: bool,
    pub details_tree: bool,
    pub sections: Sections,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            sidebar_width: 260,
            sidebar_collapsed: false,
            details_width: 400,
            graph_labels_width: AUTO_WIDTH,
            graph_lanes_width: AUTO_WIDTH,
            commands_height: 280,
            tags_shown: true,
            wip_tree: true,
            details_tree: true,
            sections: Sections::default(),
        }
    }
}

/// "No width was chosen here" — see [`LayoutState::graph_labels_width`].
pub const AUTO_WIDTH: i32 = -1;

/// A cap on the tab list, so a file that has been hand-edited into
/// something enormous cannot make startup crawl.
const MAX_TABS: usize = 64;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TabsState {
    /// Work tree paths, in the order the tabs sat in.
    pub paths: Vec<String>,
    /// Index into `paths`. Always in range once loaded.
    pub active: usize,
}

/// `state.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    pub window: WindowState,
    pub layout: LayoutState,
    pub tabs: TabsState,
}

impl State {
    pub(super) fn from_table(table: &Table) -> Self {
        let mut state = Self::default();

        if let Some(w) = sub_table(table, "window") {
            let d = WindowState::default();
            state.window = WindowState {
                x: coord(w, "x"),
                y: coord(w, "y"),
                width: int_in(w, "width", 320..=32_000, d.width),
                height: int_in(w, "height", 240..=32_000, d.height),
                maximized: flag(w, "maximized", d.maximized),
            };
        }

        if let Some(l) = sub_table(table, "layout") {
            let d = LayoutState::default();
            state.layout = LayoutState {
                sidebar_width: int_in(l, "sidebar_width", 180..=4_000, d.sidebar_width),
                sidebar_collapsed: flag(l, "sidebar_collapsed", d.sidebar_collapsed),
                details_width: int_in(l, "details_width", 300..=4_000, d.details_width),
                graph_labels_width: width_or_auto(l, "graph_labels_width"),
                graph_lanes_width: width_or_auto(l, "graph_lanes_width"),
                commands_height: int_in(l, "commands_height", 120..=4_000, d.commands_height),
                tags_shown: flag(l, "tags_shown", d.tags_shown),
                wip_tree: flag(l, "wip_tree", d.wip_tree),
                details_tree: flag(l, "details_tree", d.details_tree),
                sections: match sub_table(l, "sections") {
                    Some(s) => Sections {
                        branches: flag(s, "branches", d.sections.branches),
                        remotes: flag(s, "remotes", d.sections.remotes),
                        worktree: flag(s, "worktree", d.sections.worktree),
                        stashes: flag(s, "stashes", d.sections.stashes),
                        tags: flag(s, "tags", d.sections.tags),
                    },
                    None => d.sections,
                },
            };
        }

        if let Some(t) = sub_table(table, "tabs") {
            let paths: Vec<String> = t
                .get("paths")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .filter(|p| !p.is_empty())
                        .take(MAX_TABS)
                        .map(repo_key)
                        .collect()
                })
                .unwrap_or_default();
            let active = t
                .get("active")
                .and_then(Value::as_integer)
                .and_then(|v| usize::try_from(v).ok())
                .filter(|v| *v < paths.len())
                .unwrap_or(0);
            state.tabs = TabsState { paths, active };
        }

        state
    }

    pub(super) fn to_table(&self) -> Table {
        let mut root = Table::new();
        root.insert("version".into(), Value::Integer(SCHEMA_VERSION));

        let mut window = Table::new();
        if let Some(x) = self.window.x {
            window.insert("x".into(), Value::Integer(x.into()));
        }
        if let Some(y) = self.window.y {
            window.insert("y".into(), Value::Integer(y.into()));
        }
        window.insert("width".into(), Value::Integer(self.window.width.into()));
        window.insert("height".into(), Value::Integer(self.window.height.into()));
        window.insert("maximized".into(), Value::Boolean(self.window.maximized));
        root.insert("window".into(), Value::Table(window));

        let l = &self.layout;
        let mut layout = Table::new();
        layout.insert(
            "sidebar_width".into(),
            Value::Integer(l.sidebar_width.into()),
        );
        layout.insert(
            "sidebar_collapsed".into(),
            Value::Boolean(l.sidebar_collapsed),
        );
        layout.insert(
            "details_width".into(),
            Value::Integer(l.details_width.into()),
        );
        layout.insert(
            "graph_labels_width".into(),
            Value::Integer(l.graph_labels_width.into()),
        );
        layout.insert(
            "graph_lanes_width".into(),
            Value::Integer(l.graph_lanes_width.into()),
        );
        layout.insert(
            "commands_height".into(),
            Value::Integer(l.commands_height.into()),
        );
        layout.insert("tags_shown".into(), Value::Boolean(l.tags_shown));
        layout.insert("wip_tree".into(), Value::Boolean(l.wip_tree));
        layout.insert("details_tree".into(), Value::Boolean(l.details_tree));
        let mut sections = Table::new();
        sections.insert("branches".into(), Value::Boolean(l.sections.branches));
        sections.insert("remotes".into(), Value::Boolean(l.sections.remotes));
        sections.insert("worktree".into(), Value::Boolean(l.sections.worktree));
        sections.insert("stashes".into(), Value::Boolean(l.sections.stashes));
        sections.insert("tags".into(), Value::Boolean(l.sections.tags));
        layout.insert("sections".into(), Value::Table(sections));
        root.insert("layout".into(), Value::Table(layout));

        let mut tabs = Table::new();
        let paths: Vec<Value> = self
            .tabs
            .paths
            .iter()
            .filter(|p| !p.is_empty())
            .take(MAX_TABS)
            .map(|p| Value::String(repo_key(p)))
            .collect();
        let active =
            i64::try_from(self.tabs.active.min(paths.len().saturating_sub(1))).unwrap_or(0);
        tabs.insert("active".into(), Value::Integer(active));
        tabs.insert("paths".into(), Value::Array(paths));
        root.insert("tabs".into(), Value::Table(tabs));

        root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::STATE_FILE;
    use crate::settings::testkit::dir_store;
    #[test]
    fn one_bad_value_costs_only_that_key() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join(STATE_FILE),
            r#"
version = 1
[window]
width = "wide"
height = 1000
[layout]
sidebar_width = 0
details_width = 640
colour = "midnight"
"#,
        )
        .expect("write");

        let state = dir_store(dir.path()).load_state();
        let d = State::default();
        assert_eq!(state.window.width, d.window.width, "wrong type falls back");
        assert_eq!(state.window.height, 1000, "the good key survives");
        assert_eq!(
            state.layout.sidebar_width, d.layout.sidebar_width,
            "an unusable width falls back like a wrong type"
        );
        assert_eq!(state.layout.details_width, 640);
    }

    /// The graph's two columns carry a value below every real width that
    /// still means something: nobody has moved this divider. It has to
    /// survive the same range check that throws out a stored 3.
    #[test]
    fn an_untouched_graph_divider_is_not_a_width() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join(STATE_FILE),
            r#"
version = 1
[layout]
graph_labels_width = -1
graph_lanes_width = 3
"#,
        )
        .expect("write");

        let state = dir_store(dir.path()).load_state();
        assert_eq!(
            state.layout.graph_labels_width, AUTO_WIDTH,
            "-1 is a value, not a bad one"
        );
        assert_eq!(
            state.layout.graph_lanes_width, AUTO_WIDTH,
            "a column too narrow to hold a lane falls back like a wrong type"
        );

        let mut moved = state;
        moved.layout.graph_labels_width = 190;
        moved.layout.graph_lanes_width = 300;
        let store = dir_store(dir.path());
        store.save_state(&moved).expect("save");
        let back = store.load_state().layout;
        assert_eq!(
            (back.graph_labels_width, back.graph_lanes_width),
            (190, 300)
        );
    }

    #[test]
    fn active_tab_outside_the_list_falls_back() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join(STATE_FILE),
            "[tabs]\nactive = 9\npaths = ['C:/a', '', 'C:/b']\n",
        )
        .expect("write");
        let tabs = dir_store(dir.path()).load_state().tabs;
        assert_eq!(tabs.paths, vec!["C:/a".to_string(), "C:/b".to_string()]);
        assert_eq!(tabs.active, 0);
    }
}
