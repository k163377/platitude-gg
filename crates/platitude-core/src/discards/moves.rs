//! The reflog lines that may have taken commits away, read off their
//! messages; whether they did is the walk's question (`lost`).

use super::DiscardKind;
use crate::oid::Oid;

/// One line of `git log -g`, newest first within its ref.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Line {
    pub(super) new: Oid,
    pub(super) at: i64,
    pub(super) message: String,
}

/// A line worth asking about: what it did, to what, and the tip it left.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Move {
    pub(super) kind: DiscardKind,
    /// The branch it moved; empty for a move made on a detached HEAD.
    pub(super) name: String,
    pub(super) worktree: String,
    pub(super) at: i64,
    pub(super) old: Oid,
    pub(super) new: Oid,
    /// Made on a detached HEAD — its tip comes back under
    /// `detached-pgg-restored`, no branch's name (§4).
    pub(super) detached: bool,
}

/// Parses `%H<US>%gD<US>%gs` lines into their refs, each ref's lines in the
/// order git gave them (newest first): `%gD` with `--date=unix` reads
/// `main@{1790974132}` for a branch `--branches` named (`refs/heads/main@{…}`
/// for one named in full — both read as `main`), and `HEAD@{…}`,
/// `worktrees/c/HEAD@{…}` for the HEADs named as given.
pub(super) fn parse_lines(text: &str) -> Vec<(String, Line)> {
    text.lines()
        .filter_map(|line| {
            let mut fields = line.splitn(3, '\u{1f}');
            let new = Oid::from_hex_str(fields.next()?).ok()?;
            let (reference, at) = selector(fields.next()?)?;
            let reference = reference.strip_prefix("refs/heads/").unwrap_or(reference);
            let message = fields.next().unwrap_or_default().to_string();
            Some((reference.to_string(), Line { new, at, message }))
        })
        .collect()
}

/// A `%gD` selector read with `--date=unix`: the ref, and the time.
pub(super) fn selector(text: &str) -> Option<(&str, i64)> {
    let (reference, rest) = text.rsplit_once("@{")?;
    Some((reference, rest.strip_suffix('}')?.parse().ok()?))
}

/// The lines of a branch's reflog that may have taken commits off its tip.
/// Each line's old value is the value the line before it (older) left.
pub(super) fn branch_moves(branch: &str, lines: &[Line]) -> Vec<Move> {
    lines
        .windows(2)
        .filter_map(|pair| {
            let (line, before) = (&pair[0], &pair[1]);
            let kind = kind_of(&line.message)?;
            (line.new != before.new).then(|| Move {
                kind,
                name: branch.to_string(),
                worktree: String::new(),
                at: line.at,
                old: before.new,
                new: line.new,
                detached: false,
            })
        })
        .collect()
}

/// The lines of a worktree's HEAD reflog that may have taken commits away: the
/// branch moves its branch's own reflog holds (and loses with the branch),
/// a detached HEAD left behind, and the deletes another tool wrote here.
///
/// Read newest first from where HEAD stands now (`on`: its branch, `None`
/// while detached), so each line knows the branch it moved: a line
/// `checkout: moving from <a> to <b>` says where the lines before it stood
/// — `<a>` is the branch's name, or the commit's id while detached — and a
/// rebase's `returning to refs/heads/<b>` that its own lines were `<b>`'s.
/// The lines between a rebase's return and its start are its own work —
/// its picks, and what an `exec` it ran wrote (a reword's
/// `commit (amend)`): only the start is asked about (§1: what an operation
/// did on its way is not listed). `folder` names the worktree for a detached
/// HEAD left behind; `worktree` is how an entry names the worktree (empty for
/// the one the session stands in).
pub(super) fn head_moves(
    folder: &str,
    worktree: &str,
    on: Option<&str>,
    lines: &[Line],
) -> Vec<Move> {
    let mut out = Vec::new();
    let mut standing = on.map(str::to_string);
    let mut in_rebase = false;
    for (line, before) in lines.iter().zip(lines.iter().skip(1)) {
        let made = |kind, name: &str, old: Oid, detached: bool| Move {
            kind,
            name: name.to_string(),
            worktree: worktree.to_string(),
            at: line.at,
            old,
            new: line.new,
            detached,
        };
        let old = before.new;
        if let Some((from, _)) = line
            .message
            .strip_prefix("checkout: moving from ")
            .and_then(|rest| rest.split_once(" to "))
        {
            let left = Oid::from_hex_str(from).ok();
            if left == Some(old) && old != line.new {
                out.push(made(DiscardKind::LeftDetached, folder, old, true));
            }
            standing = left.is_none().then(|| from.to_string());
        } else if let Some(branch) = returning_to(&line.message) {
            standing = Some(branch.to_string());
            in_rebase = true;
        } else if let Some(kind) = rebase_started(&line.message) {
            in_rebase = false;
            if old != line.new {
                let name = standing.as_deref().unwrap_or_default();
                out.push(made(kind, name, old, standing.is_none()));
            }
        } else if in_rebase {
            continue;
        } else if let Some((name, tip)) = deleted_branch(&line.message) {
            out.push(made(DiscardKind::Deleted, &name, tip, false));
        } else if let Some(kind) = head_kind_of(&line.message)
            && old != line.new
        {
            let name = standing.as_deref().unwrap_or_default();
            out.push(made(kind, name, old, standing.is_none()));
        }
    }
    out
}

/// `rebase (finish): returning to refs/heads/<b>` — and the same for an
/// abort, an interactive rebase and the rebase a pull runs: the branch the
/// rebase's own lines were for.
fn returning_to(message: &str) -> Option<&str> {
    if !rebase_line(message) {
        return None;
    }
    message
        .split_once("returning to refs/heads/")
        .map(|(_, branch)| branch)
}

/// A line a rebase wrote: `rebase …`, and a pull's — git puts the pull's
/// whole command line in front (`pull --no-edit (finish): …`, `pull -r …`),
/// whatever made it rebase.
fn rebase_line(message: &str) -> bool {
    message.starts_with("rebase") || message.starts_with("pull")
}

/// HEAD's line where a rebase starts: the tip before it is this line's
/// old value.
fn rebase_started(message: &str) -> Option<DiscardKind> {
    (rebase_line(message) && message.contains("(start)")).then_some(DiscardKind::Rebase)
}

/// A branch's reflog line that can take commits off its tip, by what it
/// did; `None` for the lines that only ever add on top (a commit, a merge,
/// a fast-forward, the branch's creation).
pub(super) fn kind_of(message: &str) -> Option<DiscardKind> {
    if message.starts_with("reset:") {
        Some(DiscardKind::Reset)
    } else if message.starts_with("commit (amend):") {
        Some(DiscardKind::Amend)
    } else if rebase_line(message) {
        // `rebase (finish)` on the branch; a pull that rebases writes the
        // same after its own command line. A pull that merged only adds on
        // top, which the walk answers.
        (!message.ends_with(": Fast-forward")).then_some(DiscardKind::Rebase)
    } else if message == "rewritten during rebase" {
        // The other branches a rebase moves with `--update-refs`.
        Some(DiscardKind::Rebase)
    } else if message.starts_with("branch: Reset to") {
        Some(DiscardKind::Moved)
    } else {
        None
    }
}

/// The same for HEAD's reflog outside a rebase (`rebase_started` asks of
/// its start), where a rebase's other lines only add on top or return to
/// the branch.
fn head_kind_of(message: &str) -> Option<DiscardKind> {
    if rebase_line(message) {
        return None;
    }
    kind_of(message)
}

/// `delete_branch: <name> <upstream> [<tip>]` — how JetBrains IDEs write a
/// delete into HEAD's reflog: the name, and the tip in brackets at the end.
fn deleted_branch(message: &str) -> Option<(String, Oid)> {
    let rest = message.strip_prefix("delete_branch: ")?;
    let name = rest.split(' ').next()?.to_string();
    let tip = rest.rsplit_once('[')?.1.strip_suffix(']')?;
    Some((name, Oid::from_hex_str(tip).ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "1111111111111111111111111111111111111111";
    const B: &str = "2222222222222222222222222222222222222222";
    const C: &str = "3333333333333333333333333333333333333333";

    fn oid(hex: &str) -> Oid {
        Oid::from_hex_str(hex).expect("test oid")
    }

    fn line(new: &str, at: i64, message: &str) -> Line {
        Line {
            new: oid(new),
            at,
            message: message.to_string(),
        }
    }

    #[test]
    fn a_line_reads_its_ref_value_time_and_message() {
        let text = format!(
            "{A}\u{1f}main@{{1790974132}}\u{1f}reset: moving to HEAD~1\n\
             {B}\u{1f}worktrees/c/HEAD@{{1790974133}}\u{1f}commit: x\n\
             {C}\u{1f}refs/heads/topic@{{1790974134}}\u{1f}commit: y\n"
        );
        let parsed = parse_lines(&text);
        assert_eq!(
            parsed[0],
            (
                "main".to_string(),
                line(A, 1790974132, "reset: moving to HEAD~1")
            )
        );
        assert_eq!(parsed[1].0, "worktrees/c/HEAD");
        assert_eq!(
            parsed[2].0, "topic",
            "a branch named in full reads as --branches names it"
        );
    }

    #[test]
    fn a_moved_tip_leaves_the_value_the_older_line_left() {
        let lines = [
            line(B, 20, "reset: moving to HEAD~1"),
            line(A, 10, "commit: c3"),
        ];
        let moves = branch_moves("main", &lines);
        assert_eq!(moves.len(), 1);
        assert_eq!(
            (moves[0].old, moves[0].new, moves[0].kind),
            (oid(A), oid(B), DiscardKind::Reset)
        );
    }

    #[test]
    fn lines_that_only_add_are_not_asked_about() {
        for message in [
            "commit: c2",
            "commit (merge): Merge x",
            "merge topic: Fast-forward",
            "pull: Fast-forward",
            "branch: Created from HEAD",
            "cherry-pick: x",
            "update by push",
        ] {
            let lines = [line(B, 20, message), line(A, 10, "commit: c1")];
            assert!(branch_moves("main", &lines).is_empty(), "{message}");
        }
    }

    #[test]
    fn the_kind_comes_off_the_message() {
        assert_eq!(kind_of("commit (amend): c2"), Some(DiscardKind::Amend));
        assert_eq!(
            kind_of("rebase (finish): refs/heads/topic onto 1234"),
            Some(DiscardKind::Rebase)
        );
        assert_eq!(
            kind_of("rebase -i (finish): refs/heads/topic onto 1234"),
            Some(DiscardKind::Rebase)
        );
        assert_eq!(kind_of("branch: Reset to HEAD~1"), Some(DiscardKind::Moved));
        assert_eq!(
            kind_of("pull --no-edit (finish): refs/heads/main onto 1234"),
            Some(DiscardKind::Rebase),
            "the rebase a pull runs, behind the pull's own command line"
        );
        assert_eq!(
            kind_of("rewritten during rebase"),
            Some(DiscardKind::Rebase),
            "a branch `--update-refs` moved"
        );
    }

    /// Newest first, standing on topic now: a rebase of topic, an amend on
    /// topic, a reset on `gone` — a branch since deleted, named by the
    /// checkout that left it — and an amend made detached, whose commit the
    /// checkout onto `gone` left behind.
    #[test]
    fn head_names_the_branch_each_line_moved() {
        let lines = [
            line(C, 90, "rebase (finish): returning to refs/heads/topic"),
            line(C, 80, "rebase (pick): x"),
            line(A, 70, "rebase (start): checkout main"),
            line(B, 60, "commit (amend): y"),
            line(C, 50, "checkout: moving from gone to topic"),
            line(A, 40, "reset: moving to HEAD~1"),
            line(B, 30, &format!("checkout: moving from {A} to gone")),
            line(A, 20, "commit (amend): z"),
            line(C, 10, "checkout: moving from main to HEAD"),
            line(C, 5, "commit: w"),
        ];
        let moves = head_moves("repo", "", Some("topic"), &lines);
        let seen: Vec<(DiscardKind, &str, i64)> = moves
            .iter()
            .map(|m| (m.kind, m.name.as_str(), m.at))
            .collect();
        assert_eq!(
            seen,
            vec![
                (DiscardKind::Rebase, "topic", 70),
                (DiscardKind::Amend, "topic", 60),
                (DiscardKind::Reset, "gone", 40),
                (DiscardKind::LeftDetached, "repo", 30),
                (DiscardKind::Amend, "", 20),
            ]
        );
        let detached: Vec<bool> = moves.iter().map(|m| m.detached).collect();
        assert_eq!(detached, [false, false, false, true, true]);
    }

    /// A rebase is asked about once, where it started: the amend a reword's
    /// `exec` wrote on its way is the rebase's own work — and a pull's
    /// rebase reads the same under the pull's command line.
    #[test]
    fn a_rebase_is_its_start_and_nothing_it_did_on_its_way() {
        for action in ["rebase -i", "pull --no-edit"] {
            let lines = [
                line(
                    C,
                    90,
                    &format!("{action} (finish): returning to refs/heads/topic"),
                ),
                line(C, 80, "commit (amend): reworded"),
                line(B, 70, &format!("{action} (pick): x")),
                line(A, 60, &format!("{action} (start): checkout main")),
                line(B, 50, "commit: y"),
            ];
            let moves = head_moves("repo", "", Some("topic"), &lines);
            let seen: Vec<(DiscardKind, &str, i64)> = moves
                .iter()
                .map(|m| (m.kind, m.name.as_str(), m.at))
                .collect();
            assert_eq!(seen, vec![(DiscardKind::Rebase, "topic", 60)], "{action}");
        }
    }

    #[test]
    fn a_detached_head_left_for_a_branch_is_a_leave() {
        let lines = [
            line(B, 20, &format!("checkout: moving from {A} to main")),
            line(A, 10, "commit: d2"),
        ];
        let leaves = head_moves("c", "c", Some("main"), &lines);
        assert_eq!(leaves.len(), 1);
        assert_eq!(
            (leaves[0].kind, leaves[0].old, leaves[0].worktree.as_str()),
            (DiscardKind::LeftDetached, oid(A), "c")
        );
    }

    #[test]
    fn leaving_a_branch_is_not_a_leave() {
        let lines = [
            line(B, 20, "checkout: moving from topic to main"),
            line(A, 10, "commit: x"),
        ];
        assert!(head_moves("c", "", Some("main"), &lines).is_empty());
    }

    #[test]
    fn a_delete_another_tool_wrote_names_the_branch_and_its_tip() {
        let lines = [
            line(B, 20, &format!("delete_branch: 3.x FasterXML/3.x [{A}]")),
            line(B, 10, "commit: x"),
        ];
        let deletes = head_moves("repo", "", Some("main"), &lines);
        assert_eq!(deletes.len(), 1);
        assert_eq!(
            (deletes[0].kind, deletes[0].name.as_str(), deletes[0].old),
            (DiscardKind::Deleted, "3.x", oid(A))
        );
    }
}
