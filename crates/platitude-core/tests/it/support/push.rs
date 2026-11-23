//! A branch's push through both doors, read as the app reads them: the
//! toolbar off the status tick — the branch HEAD is on, its marks and the
//! counts read under them (`StatusLoaded::push`) — against the refs
//! snapshot (`RefsSnapshot::head_push`), and a branch's row off the
//! snapshot alone (`RefsSnapshot::branch_push`). A test compares what the
//! doors answer, never a reading put together by hand.

use platitude_core::offers::TreeStanding;
use platitude_core::remote::{HeadBranch, HeadPush, PushFacts, StatusPush};
use platitude_core::session::{RefsSnapshot, SessionEvent};
use platitude_core::status::WorktreeStatus;

use super::session::CaptureSink;

/// Where a branch that tracks nothing, marked nowhere, goes.
pub const FALLBACK: &str = "origin";

/// The push of the branch HEAD is on as the app holds it after `status`
/// (`WorktreeModel.headPush`): settled on the branch that status names.
pub fn head_of(status: &WorktreeStatus, push: &StatusPush) -> HeadPush {
    match &status.branch_head {
        None => HeadPush::Detached,
        Some(name) => HeadPush::Read(HeadBranch {
            name: name.clone(),
            unborn: false,
            upstream: status.upstream.clone(),
            upstream_counts: status.upstream_counts,
            status: push.clone(),
        }),
    }
}

/// The toolbar's answer, on an open tab, off one status and one refs
/// snapshot held side by side.
pub fn toolbar(status: &WorktreeStatus, push: &StatusPush, snapshot: &RefsSnapshot) -> PushFacts {
    snapshot.head_push(&head_of(status, push), FALLBACK, true)
}

/// The toolbar's answer off the events from `from` on: the last status
/// about `branch` there, beside the last refs snapshot.
pub async fn toolbar_from(sink: &CaptureSink, from: usize, branch: &str) -> PushFacts {
    sink.wait_for("a status of the branch, and a refs snapshot", |evs| {
        let (status, push) = evs[from..].iter().rev().find_map(|e| match e {
            SessionEvent::StatusLoaded { status, push, .. }
                if status.branch_head.as_deref() == Some(branch) =>
            {
                Some((status, push))
            }
            _ => None,
        })?;
        let snapshot = evs.iter().rev().find_map(|e| match e {
            SessionEvent::RefsLoaded { snapshot, .. } => Some(snapshot),
            _ => None,
        })?;
        Some(toolbar(status, push, snapshot))
    })
    .await
}

/// A row's answer for `branch` in `snapshot`, on an open tab: the branch
/// HEAD is on answers nothing, as the listing has it.
pub fn row_in(snapshot: &RefsSnapshot, branch: &str) -> PushFacts {
    snapshot.branch_push(branch, FALLBACK, true, &TreeStanding::default())
}

/// [`row_in`] off the last refs snapshot that lists `branch`.
pub async fn row(sink: &CaptureSink, branch: &str) -> PushFacts {
    sink.wait_for("a snapshot listing the branch", |evs| {
        evs.iter().rev().find_map(|e| match e {
            SessionEvent::RefsLoaded { snapshot, .. } if snapshot.local_named(branch).is_some() => {
                Some(row_in(snapshot, branch))
            }
            _ => None,
        })
    })
    .await
}
