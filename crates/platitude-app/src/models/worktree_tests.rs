use super::*;
use platitude_core::opstate::OpState;
use platitude_core::status::{StatusItem, WorktreeStatus};

const ROOT: &str = "1111111111111111111111111111111111111111";
const NEXT: &str = "2222222222222222222222222222222222222222";

fn head_on(branch: &str, oid_hex: &str, seq: u64) -> StateMsg {
    StateMsg::Head(HeadMsg {
        oid_hex: oid_hex.to_string(),
        branch: branch.to_string(),
        detached: false,
        seq,
    })
}

fn head(oid_hex: &str, seq: u64) -> StateMsg {
    head_on("main", oid_hex, seq)
}

fn status(items: Vec<StatusItem>) -> StateMsg {
    status_of("main", None, 0, 1, items)
}

fn status_of(
    branch: &str,
    upstream: Option<&str>,
    ahead: i32,
    head_seq: u64,
    items: Vec<StatusItem>,
) -> StateMsg {
    StateMsg::Status(Box::new(StatusMsg {
        status: WorktreeStatus {
            branch_oid: None,
            branch_head: Some(branch.to_string()),
            upstream: upstream.map(str::to_string),
            upstream_tracked: upstream.is_some(),
            ahead,
            behind: 0,
            items,
            staged_print: 0,
        },
        head_seq,
        looked: 0,
        op_state: platitude_core::opstate::OpState::default(),
        progress: None,
        sides: platitude_core::conflict::Sides::default(),
        op_message: String::new(),
        merge_tool: String::new(),
        push_remote: String::new(),
        upstream_here: false,
        push_track: platitude_core::remote::PushTrack::default(),
        eol_marks: Arc::new(Vec::new()),
        lfs_needed: 0,
        not_copied: Arc::new(Vec::new()),
        stop: platitude_core::integrate::RebaseStop::default(),
    }))
}

/// HEAD comes off its own report and nothing else: a status that
/// names no commit does not make the tree unborn once a HEAD is known.
#[test]
fn head_is_the_reports_own_and_the_status_does_not_overwrite_it() {
    let mut model = WorktreeModel::default();
    assert!(!model.absorb(Vec::new()), "nothing arrived, nothing said");
    assert!(!model.head_known);
    assert!(model.absorb(vec![head(ROOT, 1), status(Vec::new())]));
    assert!(model.head_known);
    assert!(model.loaded);
    assert_eq!(model.head_oid, ROOT);
    assert_eq!(
        model.branch_oid, ROOT,
        "on a branch, its tip is HEAD's commit"
    );
    assert_eq!(model.head_seq, 1);
    assert!(!model.unborn);
    assert_eq!(model.stash_standing, "clean");
}

/// A status carrying paths a discard cannot copy.
fn with_not_copied(msg: StateMsg, paths: &[&str]) -> StateMsg {
    let StateMsg::Status(mut status) = msg else {
        panic!("a status message");
    };
    status.not_copied = Arc::new(paths.iter().map(|p| (*p).to_string()).collect());
    StateMsg::Status(status)
}

/// The paths are held for the discards to ask of, and `reset --hard`
/// counts only the ones it takes: an untracked file survives it.
#[test]
fn a_reset_hard_loses_an_uncopied_file_only_where_it_takes_it() {
    let changed = StatusItem::Tracked {
        staged: '.',
        unstaged: 'M',
        path: "kept.psd".to_string(),
        orig_path: None,
    };
    let new = StatusItem::Untracked {
        path: "new.psd".to_string(),
    };
    let mut model = WorktreeModel::default();
    model.absorb(vec![
        head(ROOT, 1),
        with_not_copied(status(vec![changed.clone(), new.clone()]), &["new.psd"]),
    ]);
    assert_eq!(model.not_copied, ["new.psd"]);
    assert!(!model.hard_reset_not_copied, "the untracked file survives");

    model.absorb(vec![with_not_copied(
        status(vec![changed, new]),
        &["kept.psd", "new.psd"],
    )]);
    assert!(
        model.hard_reset_not_copied,
        "the changed file goes uncopied"
    );

    model.absorb(vec![status(Vec::new())]);
    assert!(model.not_copied.is_empty());
    assert!(!model.hard_reset_not_copied);
}

#[test]
fn a_branch_with_no_commits_is_unborn_once_head_has_been_read() {
    let mut model = WorktreeModel::default();
    model.absorb(vec![status(Vec::new())]);
    assert!(!model.unborn, "no HEAD report yet is not an unborn branch");
    model.absorb(vec![head("", 1)]);
    assert!(model.unborn);
    assert_eq!(model.stash_standing, "unborn");
}

#[test]
fn detached_names_no_branch_tip() {
    let mut model = WorktreeModel::default();
    model.absorb(vec![StateMsg::Head(HeadMsg {
        oid_hex: ROOT.to_string(),
        branch: String::new(),
        detached: true,
        seq: 1,
    })]);
    assert_eq!(model.head_oid, ROOT);
    assert_eq!(model.branch_oid, "");
}

#[test]
fn the_published_answer_follows_the_commit_it_is_about() {
    let mut model = WorktreeModel::default();
    model.absorb(vec![
        head(ROOT, 1),
        StateMsg::HeadPublished {
            oid_hex: ROOT.to_string(),
            published: true,
        },
    ]);
    assert!(model.head_published);
    model.absorb(vec![head(NEXT, 2)]);
    assert!(!model.head_published, "an answer about the last commit");
    model.absorb(vec![StateMsg::HeadPublished {
        oid_hex: NEXT.to_string(),
        published: true,
    }]);
    assert!(model.head_published);
}

/// A `false` answer is an answer: known from the first one, whatever
/// it says, until the tab stands in another worktree.
#[test]
fn the_reach_is_known_once_answered_and_until_the_worktree_is_left() {
    let mut model = WorktreeModel::default();
    assert!(!model.head_reach_known);
    model.absorb(vec![StateMsg::HeadReach {
        reached_elsewhere: false,
    }]);
    assert!(model.head_reach_known);
    assert!(!model.head_reached_elsewhere);
    model.forget_the_worktree();
    assert!(!model.head_reach_known);
}

/// The first read after a write reports an unmoved HEAD, and it still
/// counts: `headSeq` is what a landing arms against.
#[test]
fn a_report_that_moved_nothing_still_counts() {
    let mut model = WorktreeModel::default();
    model.absorb(vec![head(ROOT, 1)]);
    model.absorb(vec![head(ROOT, 2)]);
    assert_eq!(model.head_oid, ROOT);
    assert_eq!(model.head_seq, 2);
}

/// `statusSeq` stays with the status, not the HEAD report in hand
/// (`status_seq`).
#[test]
fn the_counts_stand_beside_the_report_they_were_read_under() {
    let mut model = WorktreeModel::default();
    model.absorb(vec![
        head(ROOT, 1),
        status_of("main", None, 0, 1, Vec::new()),
    ]);
    assert_eq!(model.status_seq, 1);
    model.absorb(vec![head(NEXT, 2)]);
    assert_eq!(model.head_seq, 2);
    assert_eq!(model.status_seq, 1, "the counts are still the old tree's");
    model.absorb(vec![status_of("main", None, 0, 2, Vec::new())]);
    assert_eq!(model.status_seq, 2);
}

#[test]
fn the_counts_are_the_branch_they_were_read_with() {
    let mut model = WorktreeModel::default();
    model.absorb(vec![
        head_on("feature", ROOT, 1),
        status_of("feature", Some("origin/feature"), 2, 1, Vec::new()),
    ]);
    assert!(model.counts_settled);
    assert_eq!(model.upstream, "origin/feature");
    assert_eq!(model.ahead, 2);

    model.absorb(vec![head_on("main", NEXT, 2)]);
    assert!(!model.counts_settled, "feature's counts are not main's");
    assert_eq!(model.upstream, "");
    assert!(!model.upstream_tracked);
    assert_eq!(model.ahead, 0);

    model.absorb(vec![status_of(
        "main",
        Some("origin/main"),
        0,
        2,
        Vec::new(),
    )]);
    assert!(model.counts_settled);
    assert_eq!(model.upstream, "origin/main");
    assert_eq!(model.ahead, 0);
}

/// The push destination's own counts go blank with the upstream's.
#[test]
fn the_push_destinations_counts_are_the_branch_they_were_read_with() {
    let mut model = WorktreeModel::default();
    let mut read = status_of("feature", Some("origin/feature"), 0, 1, Vec::new());
    if let StateMsg::Status(status) = &mut read {
        status.push_track = platitude_core::remote::PushTrack {
            tracking: "fork/feature".to_string(),
            ahead: 1,
            behind: 2,
        };
    }
    model.absorb(vec![head_on("feature", ROOT, 1), read]);
    let pushes = |model: &WorktreeModel| {
        (
            model.push_tracking.clone(),
            model.push_ahead,
            model.push_behind,
        )
    };
    assert_eq!(pushes(&model), ("fork/feature".to_string(), 1, 2));

    model.absorb(vec![head_on("main", NEXT, 2)]);
    assert_eq!(
        pushes(&model),
        (String::new(), 0, 0),
        "feature's destination is not main's"
    );
}

#[test]
fn the_stash_standing_is_settled_off_head_and_the_counts_together() {
    let mut model = WorktreeModel::default();
    model.absorb(vec![head("", 1)]);
    model.absorb(vec![status(vec![StatusItem::Untracked {
        path: "f.txt".to_string(),
    }])]);
    assert_eq!(model.stash_standing, "unborn", "no commit to stash on");
    model.absorb(vec![head(ROOT, 2)]);
    assert_eq!(
        model.stash_standing, "ready",
        "the same tree, once it has a commit"
    );
}

/// The badge's word is the only thing on screen naming the stopped
/// operation (`BandStateGroup` / `BandStateCard`). A `match` over
/// `InProgress`, so a fifth operation stops the build here rather than
/// reaching the badge with no word.
#[test]
fn every_operation_that_can_stand_here_reaches_the_badge_under_its_own_word() {
    use platitude_core::integrate::InProgress;

    for op in [
        InProgress::Rebase,
        InProgress::Merge,
        InProgress::CherryPick,
        InProgress::Revert,
    ] {
        let (state, word) = match op {
            InProgress::Rebase => (
                OpState {
                    rebasing: true,
                    ..OpState::default()
                },
                "REBASING",
            ),
            InProgress::Merge => (
                OpState {
                    merging: true,
                    ..OpState::default()
                },
                "MERGING",
            ),
            InProgress::CherryPick => (
                OpState {
                    cherry_picking: true,
                    ..OpState::default()
                },
                "CHERRY-PICKING",
            ),
            InProgress::Revert => (
                OpState {
                    reverting: true,
                    ..OpState::default()
                },
                "REVERTING",
            ),
        };
        let mut model = WorktreeModel::default();
        model.settle_op(&state, "");
        assert_eq!(model.op_text, word);
        assert!(model.op_also.is_empty(), "{word}: one operation, one word");
    }
}

/// The badge is measured from which slot each word lands in
/// (`BandStateMetrics.opW`).
#[test]
fn a_bisect_takes_the_second_word_and_the_first_when_it_is_alone() {
    let mut model = WorktreeModel::default();
    model.settle_op(
        &OpState {
            bisecting: true,
            ..OpState::default()
        },
        "",
    );
    assert_eq!(
        (model.op_text.as_str(), model.op_also.as_str()),
        ("BISECTING", "")
    );

    model.settle_op(
        &OpState {
            rebasing: true,
            bisecting: true,
            ..OpState::default()
        },
        "",
    );
    assert_eq!(
        (model.op_text.as_str(), model.op_also.as_str()),
        ("REBASING", "BISECTING")
    );
}

#[test]
fn a_rebase_that_stopped_on_a_pick_is_one_operation_not_two() {
    let mut model = WorktreeModel::default();
    model.settle_op(
        &OpState {
            rebasing: true,
            cherry_picking: true,
            ..OpState::default()
        },
        "",
    );
    assert_eq!(
        (model.op_text.as_str(), model.op_also.as_str()),
        ("REBASING", "")
    );

    // Nothing standing: no word, which keeps the badge off the band.
    model.settle_op(&OpState::default(), "");
    assert_eq!((model.op_text.as_str(), model.op_also.as_str()), ("", ""));
}

const REBASING: OpState = OpState {
    rebasing: true,
    merging: false,
    cherry_picking: false,
    reverting: false,
    bisecting: false,
};

/// A model whose badge a tick stamped `looked` put at `step` of `steps`.
fn ticked(model: &mut WorktreeModel, looked: u64, step: u32, steps: u32) {
    let feed = Arc::new(Feed::default());
    feed.push_replace(OpProgressMsg {
        op_state: REBASING,
        progress: Some(platitude_core::conflict::Progress {
            current: step,
            total: steps,
        }),
        looked,
    });
    model.progress_feed = Some(feed);
    model.drain_progress();
}

fn status_looked(looked: u64, op_state: OpState, count: Option<(u32, u32)>) -> StateMsg {
    let StateMsg::Status(mut msg) = status(Vec::new()) else {
        unreachable!("status() builds a status")
    };
    msg.looked = looked;
    msg.op_state = op_state;
    msg.progress =
        count.map(|(current, total)| platitude_core::conflict::Progress { current, total });
    StateMsg::Status(msg)
}

/// A status reads the count after its whole `git status`; a tick that
/// looked later lands first, and the status must not take it back.
#[test]
fn a_status_that_looked_before_the_tick_does_not_take_the_count_back() {
    let mut model = WorktreeModel::default();
    ticked(&mut model, 5, 7, 10);
    model.absorb(vec![status_looked(3, REBASING, Some((6, 10)))]);
    assert_eq!((model.op_step, model.op_steps), (7, 10));
    assert_eq!(model.op_text, "REBASING");
}

#[test]
fn a_status_read_before_the_first_marker_does_not_take_the_badge_down() {
    let mut model = WorktreeModel::default();
    ticked(&mut model, 5, 1, 100);
    model.absorb(vec![status_looked(3, OpState::default(), None)]);
    assert_eq!(model.op_text, "REBASING");
    assert_eq!((model.op_step, model.op_steps), (1, 100));
}

#[test]
fn the_status_after_the_write_ends_the_badge() {
    let mut model = WorktreeModel::default();
    ticked(&mut model, 5, 9, 10);
    model.absorb(vec![status_looked(8, OpState::default(), None)]);
    assert_eq!(model.op_text, "");
    assert_eq!((model.op_step, model.op_steps), (0, 0));
}

#[test]
fn a_new_worktree_counts_its_stamps_from_the_start() {
    let mut model = WorktreeModel::default();
    ticked(&mut model, 50, 3, 10);
    model.forget_the_worktree();
    ticked(&mut model, 1, 2, 4);
    assert_eq!((model.op_step, model.op_steps), (2, 4));
}

/// A status naming the stopped operation's two sides.
fn with_sides(msg: StateMsg, ours: &str, theirs: &str) -> StateMsg {
    let StateMsg::Status(mut status) = msg else {
        panic!("a status message");
    };
    status.sides = platitude_core::conflict::Sides {
        ours: ours.to_string(),
        theirs: theirs.to_string(),
    };
    StateMsg::Status(status)
}

/// The side named for the branch HEAD is on is current, whichever side
/// it is; detached, neither is — a rebase names the branch it is onto.
#[test]
fn a_side_is_current_where_head_is_on_its_branch() {
    let currents = |model: &WorktreeModel| (model.side_ours_current, model.side_theirs_current);
    let mut model = WorktreeModel::default();
    model.absorb(vec![
        head(ROOT, 1),
        with_sides(status(Vec::new()), "main", "feat"),
    ]);
    assert_eq!(currents(&model), (true, false));
    model.absorb(vec![with_sides(status(Vec::new()), "feat", "main")]);
    assert_eq!(currents(&model), (false, true));
    model.absorb(vec![StateMsg::Head(HeadMsg {
        oid_hex: NEXT.to_string(),
        branch: String::new(),
        detached: true,
        seq: 2,
    })]);
    assert_eq!(currents(&model), (false, false), "detached, no branch is");
    model.absorb(vec![with_sides(status(Vec::new()), "", "")]);
    assert_eq!(currents(&model), (false, false), "no name is no branch");
    model.absorb(vec![
        head(ROOT, 3),
        with_sides(status(Vec::new()), "main", "feat"),
    ]);
    model.forget_the_worktree();
    assert_eq!(currents(&model), (false, false));
}

/// Each field lands in its own seat of the records: a value apart per
/// field, and every flag up, so two swapped seats cannot pass.
#[test]
fn the_records_carry_each_field_in_its_own_seat() {
    let model = WorktreeModel {
        branch: "feature/日本語".into(),
        detached: true,
        head_oid: ROOT.into(),
        op_text: "REBASING".into(),
        op_editing: true,
        moves_blocked: true,
        conflict_count: 2,
        upstream: "origin/feature/日本語".into(),
        pull_blocked: true,
        unborn: true,
        hard_reset_takes: 3,
        hard_reset_not_copied: true,
        head_reached_elsewhere: true,
        stash_standing: "conflicts".into(),
        not_copied: vec!["big.psd".into()],
        ..WorktreeModel::default()
    };
    let standing = platitude_core::offers::TreeStanding {
        branch: "feature/日本語".into(),
        detached: true,
        head_oid: ROOT.into(),
        op_text: "REBASING".into(),
        op_editing: true,
        moves_blocked: true,
        upstream: "origin/feature/日本語".into(),
        diverged: true,
    };
    assert_eq!(*model.standing(), standing);
    assert_eq!(
        *model.reading(),
        crate::encode::TreeReading {
            standing,
            conflict_count: 2,
            unborn: true,
            hard_reset_takes: 3,
            hard_reset_not_copied: true,
            head_reached_elsewhere: true,
            stash_standing: "conflicts".into(),
            not_copied: vec!["big.psd".into()],
        }
    );
}
