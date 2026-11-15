//! Local branches renamed outside between two reads of the refs
//! ([`RefsSnapshot::renamed`]): which names a rename could have left, and how
//! the renames found fold into the ones a snapshot already holds.

use std::collections::HashSet;

use crate::refs::{RefEntry, RefKind};
use crate::session::{BranchItem, RefsSnapshot};

/// How many renames a snapshot holds, the newest kept, and how many names
/// one read asks the reflog about: a menu standing on an old name asks for
/// the last few, and both the run and the processes stay bounded whatever a
/// script does to the branches — a name past it is gone, as before.
const HELD: usize = 16;

/// The local branches this listing `now` holds that the held snapshot's
/// `before` did not, each with the ones `before` held on the same commit
/// that `now` does not: the only listings `git branch -m` leaves — the
/// first [`HELD`] of them. A branch renamed and moved between two reads
/// leaves no such pair, and counts as gone.
pub(super) fn candidates<'a>(
    before: &'a [BranchItem],
    now: &'a [RefEntry],
) -> Vec<(&'a str, Vec<&'a str>)> {
    let listed_now: HashSet<&str> = now
        .iter()
        .filter(|r| r.kind == RefKind::LocalBranch)
        .map(|r| r.short.as_str())
        .collect();
    let gone: Vec<&BranchItem> = before
        .iter()
        .filter(|b| !listed_now.contains(b.short.as_str()))
        .collect();
    if gone.is_empty() {
        return Vec::new();
    }
    let listed_before: HashSet<&str> = before.iter().map(|b| b.short.as_str()).collect();
    now.iter()
        .filter(|r| r.kind == RefKind::LocalBranch && !listed_before.contains(r.short.as_str()))
        .filter_map(|r| {
            let olds: Vec<&str> = gone
                .iter()
                .filter(|b| b.oid == r.commit_oid())
                .map(|b| b.short.as_str())
                .collect();
            (!olds.is_empty()).then(|| (r.short.as_str(), olds))
        })
        .take(HELD)
        .collect()
}

/// The renames `found` in this read folded into the ones `held` holds: an
/// older entry whose new name was renamed again follows it to the last
/// name, an old name `snapshot` lists again is a branch of its own, and a
/// new name it no longer lists takes its entry with it.
pub(super) fn folded(
    held: &[(crate::Name, crate::Name)],
    found: Vec<(crate::Name, crate::Name)>,
    snapshot: &RefsSnapshot,
) -> Vec<(crate::Name, crate::Name)> {
    let mut out: Vec<(crate::Name, crate::Name)> = held
        .iter()
        .map(|(old, new)| {
            let last = found
                .iter()
                .find(|(from, _)| from == new)
                .map_or_else(|| new.clone(), |(_, to)| to.clone());
            (old.clone(), last)
        })
        .collect();
    out.extend(found);
    out.retain(|(old, new)| {
        snapshot.local_named(old).is_none() && snapshot.local_named(new).is_some()
    });
    if out.len() > HELD {
        out.drain(..out.len() - HELD);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oid::Oid;

    fn oid(byte: u8) -> Oid {
        Oid::from_hex(&[byte; 40]).expect("an oid")
    }

    fn local(short: &str, at: u8) -> BranchItem {
        BranchItem {
            short: short.into(),
            full: format!("refs/heads/{short}").into(),
            oid: oid(at),
            has_remote: false,
            is_head: false,
            upstream: "".into(),
            upstream_gone: "".into(),
            upstream_local: "".into(),
            upstream_here: false,
            upstream_oid: None,
            upstream_drifted: false,
            held_elsewhere: false,
            tracked_by: "".into(),
            ahead: 0,
            behind: 0,
        }
    }

    fn listing(locals: &[(&str, u8)]) -> RefsSnapshot {
        let mut locals: Vec<BranchItem> = locals.iter().map(|(s, at)| local(s, *at)).collect();
        locals.sort_by(|a, b| a.short.cmp(&b.short));
        RefsSnapshot {
            locals,
            ..RefsSnapshot::default()
        }
    }

    fn pair(old: &str, new: &str) -> (crate::Name, crate::Name) {
        (old.into(), new.into())
    }

    fn listed(short: &str, at: u8, kind: RefKind) -> RefEntry {
        RefEntry {
            name: format!("refs/heads/{short}").into(),
            short: short.into(),
            kind,
            target: oid(at),
            peeled: None,
            upstream: None,
            is_head: false,
            created_unix: 0,
            ahead: 0,
            behind: 0,
        }
    }

    #[test]
    fn only_a_name_gained_on_a_commit_a_lost_name_stood_on_is_asked() {
        let before = [
            local("main", b'1'),
            local("topic", b'2'),
            local("left", b'3'),
        ];
        let now = [
            listed("main", b'1', RefKind::LocalBranch),
            listed("topic-two", b'2', RefKind::LocalBranch),
            listed("moved", b'4', RefKind::LocalBranch),
            listed("origin/topic-two", b'2', RefKind::RemoteBranch),
        ];
        assert_eq!(
            candidates(&before, &now),
            vec![("topic-two", vec!["topic"])],
            "a name on another commit is a branch made, not a rename; a remote one is no branch here"
        );
        let unchanged = [
            listed("main", b'1', RefKind::LocalBranch),
            listed("topic", b'2', RefKind::LocalBranch),
        ];
        assert!(
            candidates(&before[..2], &unchanged).is_empty(),
            "nothing went, nothing is asked"
        );
    }

    #[test]
    fn a_rename_of_a_rename_follows_to_the_last_name() {
        let now = listing(&[("main", b'1'), ("three", b'2')]);
        let held = [pair("one", "two")];
        assert_eq!(
            folded(&held, vec![pair("two", "three")], &now),
            vec![pair("one", "three"), pair("two", "three")]
        );
    }

    #[test]
    fn an_old_name_listed_again_or_a_new_name_gone_drops_its_entry() {
        let now = listing(&[("one", b'1'), ("main", b'1')]);
        let held = [pair("one", "two"), pair("left", "gone")];
        assert_eq!(folded(&held, Vec::new(), &now), Vec::new());
    }

    #[test]
    fn the_newest_renames_are_kept_when_there_are_too_many() {
        let names: Vec<String> = (0..HELD + 4).map(|i| format!("b{i:02}")).collect();
        let now = listing(
            &names
                .iter()
                .map(|name| (format!("{name}-new"), b'1'))
                .collect::<Vec<_>>()
                .iter()
                .map(|(name, at)| (name.as_str(), *at))
                .collect::<Vec<_>>(),
        );
        let found: Vec<_> = names
            .iter()
            .map(|name| pair(name, &format!("{name}-new")))
            .collect();
        let kept = folded(&[], found, &now);
        assert_eq!(kept.len(), HELD);
        assert_eq!(kept.first(), Some(&pair("b04", "b04-new")));
    }
}
