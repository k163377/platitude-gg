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

pub(super) fn drift(name: &str, remote: &str, digit: &str) -> platitude_core::session::TagDrift {
    platitude_core::session::TagDrift {
        name: name.into(),
        remote: remote.into(),
        commit: oid(digit),
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
        upstream_gone: "".into(),
        upstream_oid: None,
        upstream_drifted: false,
        held_elsewhere: false,
        tracked_by: "".into(),
        ahead: 0,
        behind: 0,
    }
}

pub(super) fn local(short: &str, is_head: bool) -> platitude_core::session::BranchItem {
    platitude_core::session::BranchItem {
        short: short.into(),
        full: format!("refs/heads/{short}").into(),
        oid: oid("c"),
        has_remote: false,
        is_head,
        upstream: "".into(),
        upstream_gone: "".into(),
        upstream_oid: None,
        upstream_drifted: false,
        held_elsewhere: false,
        tracked_by: "".into(),
        ahead: 0,
        behind: 0,
    }
}

pub(super) fn locals(
    list: Vec<platitude_core::session::BranchItem>,
) -> Arc<platitude_core::session::RefsSnapshot> {
    Arc::new(platitude_core::session::RefsSnapshot {
        locals: list,
        ..Default::default()
    })
}

pub(super) fn snapshot(
    remotes: Vec<platitude_core::session::BranchItem>,
    tags: Vec<platitude_core::session::TagItem>,
) -> Arc<platitude_core::session::RefsSnapshot> {
    drifted(remotes, tags, Vec::new())
}

/// A remotes snapshot with configured remote names (what
/// `build_remote_groups` cuts by).
pub(super) fn named(
    remotes: Vec<platitude_core::session::BranchItem>,
    names: &[&str],
) -> Arc<platitude_core::session::RefsSnapshot> {
    Arc::new(platitude_core::session::RefsSnapshot {
        remotes,
        remote_names: names.iter().map(|name| (*name).into()).collect(),
        ..Default::default()
    })
}

/// The same, with the run the tag menu's push row reads (`remote_tag_drift`).
pub(super) fn drifted(
    remotes: Vec<platitude_core::session::BranchItem>,
    tags: Vec<platitude_core::session::TagItem>,
    tag_drifts: Vec<platitude_core::session::TagDrift>,
) -> Arc<platitude_core::session::RefsSnapshot> {
    carried(remotes, tags, tag_drifts, Vec::new())
}

/// The same, with the remotes' `refs/tags/` readings (`tag_remotes`) as
/// `(name, remote, digit)`: two readings on one digit agree on the commit.
pub(super) fn carried(
    remotes: Vec<platitude_core::session::BranchItem>,
    tags: Vec<platitude_core::session::TagItem>,
    tag_drifts: Vec<platitude_core::session::TagDrift>,
    readings: Vec<(&str, &str, &str)>,
) -> Arc<platitude_core::session::RefsSnapshot> {
    let mut snapshot = platitude_core::session::RefsSnapshot {
        locals: Vec::new(),
        remotes,
        tags,
        tags_by_name: Vec::new(),
        tag_drifts,
        remote_tags: Arc::new(platitude_core::session::RemoteTagIndex::build(
            readings
                .into_iter()
                .map(|(name, remote, at)| (name.into(), oid(at), false, remote.into())),
        )),
        head: None,
        remote_names: Vec::new(),
        remote_urls: Vec::new(),
        push_default: None,
        checkout_default: None,
    };
    // The index the name lookups go through, the way the joins build it.
    snapshot.index_tags();
    Arc::new(snapshot)
}

pub(super) fn section(kind: &str, all: Source) -> NavSectionModel {
    let mut model = NavSectionModel::default();
    model.section = kind.to_string();
    model.all = all;
    model
}

/// One of the working tree's bucket lists: the whole status, showing one
/// run (`NavSectionModel::run`).
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

/// A number role's answer; `-1` for a missing row, so a miss cannot read
/// as zero.
pub(super) fn numbers(model: &NavSectionModel, row: usize, role: Role) -> i32 {
    model
        .row_at(row)
        .map_or(-1, |row| model.field(row, role).number())
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

/// One of each kind of pending change, in git's order: tracked entries
/// sorted, then the untracked ones after them.
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
