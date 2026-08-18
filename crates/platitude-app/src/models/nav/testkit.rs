use super::*;

pub(super) fn oid(digit: &str) -> platitude_core::Oid {
    platitude_core::Oid::from_hex_str(&digit.repeat(40)).unwrap()
}

pub(super) fn tag(short: &str, has_remote: bool, here: bool) -> platitude_core::session::TagItem {
    platitude_core::session::TagItem {
        short: short.into(),
        oid: oid("a"),
        annotated: false,
        created_unix: 0,
        has_remote,
        here,
    }
}

pub(super) fn remote(short: &str) -> platitude_core::session::BranchItem {
    platitude_core::session::BranchItem {
        short: short.into(),
        full: format!("refs/remotes/{short}").into(),
        oid: oid("b"),
        has_remote: true,
        is_head: false,
        upstream: "".into(),
    }
}

pub(super) fn snapshot(
    remotes: Vec<platitude_core::session::BranchItem>,
    tags: Vec<platitude_core::session::TagItem>,
) -> Arc<platitude_core::session::RefsSnapshot> {
    Arc::new(platitude_core::session::RefsSnapshot {
        locals: Vec::new(),
        remotes,
        tags,
        head: None,
        remote_names: Vec::new(),
    })
}

pub(super) fn section(kind: &str, all: Source) -> NavSectionModel {
    let mut model = NavSectionModel::default();
    model.section = kind.to_string();
    model.all = all;
    model
}

/// One of the working tree's bucket lists: the whole status, and the one
/// run of it this list shows (`NavSectionModel::run`).
pub(super) fn worktree(run: &str, all: Source) -> NavSectionModel {
    let mut model = section("worktree", all);
    model.run = run.to_string();
    model
}

pub(super) fn says(model: &NavSectionModel, row: usize, role: Role) -> String {
    model
        .row_at(row)
        .map(|row| model.field(row, role).as_str().to_string())
        .unwrap_or_else(|| "<no row>".to_string())
}

pub(super) fn flags(model: &NavSectionModel, row: usize, role: Role) -> bool {
    model
        .row_at(row)
        .is_some_and(|row| model.field(row, role).flag())
}

pub(super) fn depth_of(model: &NavSectionModel, row: usize) -> i32 {
    match model.row_at(row).map(|row| model.field(row, Role::Depth)) {
        Some(Value::Number(depth)) => depth,
        _ => -1,
    }
}

pub(super) fn tracked(
    staged: char,
    unstaged: char,
    path: &str,
) -> platitude_core::status::StatusItem {
    platitude_core::status::StatusItem::Tracked {
        staged,
        unstaged,
        path: path.to_string(),
        orig_path: None,
    }
}

/// One of each kind of pending change, in the order git reports them
/// rather than the order the pane shows them: the entries git tracks
/// sorted by name, and then the untracked ones sorted after all of them.
pub(super) fn pending() -> platitude_core::status::WorkTreeStatus {
    use platitude_core::status::StatusItem;
    platitude_core::status::WorkTreeStatus {
        items: vec![
            StatusItem::Unmerged {
                ours: 'U',
                theirs: 'U',
                path: "a.txt".to_string(),
            },
            StatusItem::Tracked {
                staged: 'R',
                unstaged: '.',
                path: "d.txt".to_string(),
                orig_path: Some("old.txt".to_string()),
            },
            // Both halves changed: one row under each.
            tracked('M', 'M', "src/b.txt"),
            StatusItem::Untracked {
                path: "c.txt".to_string(),
            },
        ],
        ..Default::default()
    }
}
