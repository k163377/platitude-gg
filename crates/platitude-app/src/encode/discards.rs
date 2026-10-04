//! What the discard log reads off git's reflogs and Platitude GG's own
//! record, as QML words it (`RecoverEntries`).

use platitude_core::discards::{Discard, DiscardKind, Look, Part, Restore};
use qtbridge::qtbridge_type_lib::QVariantMap;

use super::wire::{Fields, Listed, Record, field};

/// One operation that took something away.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscardRow {
    /// What it was, by [`kind_word`].
    pub kind: String,
    /// What it happened to: the branch, the tag, the working copy's folder,
    /// the stash's message; for thrown-away work the branch it was on.
    pub name: String,
    /// The remote it reached, for what went from a remote; empty otherwise.
    pub remote: String,
    /// The working copy it happened in when that is not this one.
    pub copy: String,
    /// When git or the record wrote the line, epoch seconds.
    pub at: i64,
    /// How many commits only it reaches — what the graph draws dashed.
    pub lost: i32,
    /// What each restore brings back, in the order a whole restore goes.
    pub parts: Listed<DiscardPart>,
    /// Whether a part can be brought back alone (`Discard::restores_by_part`).
    pub by_part: bool,
}

/// One part of an entry: what one restore brings back, and as what.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscardPart {
    /// `branch` | `tag` | `worktree` | `stash` | `changes`.
    pub restore: String,
    /// The name it comes back under — the branch, the tag; a working
    /// copy's path; a stash's message; for thrown-away work the folder of
    /// the working copy it goes back into.
    pub name: String,
    /// What it comes back with: a branch's upstream (`origin/main`), the
    /// branch a working copy had out; empty otherwise.
    pub with: String,
    /// How its tip draws on the graph: `commit` | `uncommitted` | `stash`.
    pub look: String,
    /// Its tip, as a row's short id writes one.
    pub tip: String,
    /// The remote it was taken from; empty for what was here.
    pub remote: String,
    /// The commits only its tip reaches.
    pub lost: i32,
    /// How many paths a copy or a stash holds.
    pub files: i32,
    /// How many paths were thrown away without a copy (§2.1).
    pub not_copied: i32,
}

pub type DiscardRows = Listed<DiscardRow>;

pub fn discard_rows(found: &[Discard]) -> DiscardRows {
    Listed::new(
        found
            .iter()
            .map(|discard| DiscardRow {
                kind: kind_word(discard.kind).to_string(),
                name: discard.name.clone(),
                remote: discard.remote.clone(),
                copy: discard.copy.clone(),
                at: discard.at,
                lost: count(discard.lost().len()),
                parts: Listed::new(discard.parts.iter().map(part_row).collect()),
                by_part: discard.restores_by_part(),
            })
            .collect(),
    )
}

fn part_row(part: &Part) -> DiscardPart {
    let (restore, name, with) = match &part.restore {
        Restore::Branch { name, upstream, .. } => (
            "branch",
            name.clone(),
            upstream
                .as_ref()
                .map(|(remote, branch)| format!("{remote}/{branch}"))
                .unwrap_or_default(),
        ),
        Restore::Tag { name, .. } => ("tag", name.clone(), String::new()),
        Restore::Worktree { path, branch, .. } => {
            ("worktree", path.clone(), branch.clone().unwrap_or_default())
        }
        Restore::Stash { message, .. } => ("stash", message.clone(), String::new()),
        Restore::Changes { path, .. } => ("changes", folder_of(path), String::new()),
    };
    DiscardPart {
        restore: restore.to_string(),
        name,
        with,
        look: look_word(part.look).to_string(),
        tip: part.tip.short_hex(8),
        remote: part.remote.clone(),
        lost: count(part.lost.len()),
        files: i32::try_from(part.files).unwrap_or(i32::MAX),
        not_copied: count(part.not_copied.len()),
    }
}

pub fn kind_word(kind: DiscardKind) -> &'static str {
    match kind {
        DiscardKind::Reset => "reset",
        DiscardKind::Amend => "amend",
        DiscardKind::Rebase => "rebase",
        DiscardKind::Moved => "moved",
        DiscardKind::LeftDetached => "left-detached",
        DiscardKind::Deleted => "deleted",
        DiscardKind::Discarded => "discarded",
        DiscardKind::DeletedUntracked => "deleted-untracked",
        DiscardKind::DroppedStash => "dropped-stash",
        DiscardKind::PoppedStash => "popped-stash",
        DiscardKind::DeletedTag => "deleted-tag",
        DiscardKind::RemovedWorktree => "removed-worktree",
        DiscardKind::DeletedRemoteBranch => "deleted-remote-branch",
        DiscardKind::DeletedRemoteTag => "deleted-remote-tag",
        DiscardKind::ForcePushed => "force-pushed",
        DiscardKind::ForcePushedTag => "force-pushed-tag",
    }
}

/// How a tip draws on the graph, as `GraphModel.showDiscard` takes it.
pub fn look_word(look: Look) -> &'static str {
    match look {
        Look::Commit => "commit",
        Look::Uncommitted => "uncommitted",
        Look::Stash => "stash",
        Look::Absent => "absent",
    }
}

fn count(n: usize) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

fn folder_of(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

/// Whether a filter's line is in any of the words an entry shows, as the
/// left menu's filter reads a name: case folded, the line taken whole. A
/// line of nothing but whitespace leaves every entry in.
pub fn shown_matches(shown: &[String], line: &str) -> bool {
    if line.trim().is_empty() {
        return true;
    }
    let needle = line.to_lowercase();
    shown
        .iter()
        .any(|words| words.to_lowercase().contains(&needle))
}

impl Record for DiscardRow {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("kind", &self.kind)
            .put("name", &self.name)
            .put("remote", &self.remote)
            .put("copy", &self.copy)
            .put("at", &self.at)
            .put("lost", &self.lost)
            .put("parts", &self.parts)
            .put("byPart", &self.by_part)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            kind: field(map, "kind")?,
            name: field(map, "name")?,
            remote: field(map, "remote")?,
            copy: field(map, "copy")?,
            at: field(map, "at")?,
            lost: field(map, "lost")?,
            parts: field(map, "parts")?,
            by_part: field(map, "byPart")?,
        })
    }
}

impl Record for DiscardPart {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("restore", &self.restore)
            .put("name", &self.name)
            .put("with", &self.with)
            .put("look", &self.look)
            .put("tip", &self.tip)
            .put("remote", &self.remote)
            .put("lost", &self.lost)
            .put("files", &self.files)
            .put("notCopied", &self.not_copied)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            restore: field(map, "restore")?,
            name: field(map, "name")?,
            with: field(map, "with")?,
            look: field(map, "look")?,
            tip: field(map, "tip")?,
            remote: field(map, "remote")?,
            lost: field(map, "lost")?,
            files: field(map, "files")?,
            not_copied: field(map, "notCopied")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shown(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| (*w).to_string()).collect()
    }

    #[test]
    fn the_line_is_found_in_any_words_case_folded() {
        let entry = shown(&["Reset main", "2 commits", "2026-10-02 04:30 (Yesterday)"]);
        assert!(shown_matches(&entry, "MAIN"));
        assert!(shown_matches(&entry, "2026-10"));
        assert!(shown_matches(&entry, "yesterday"));
        assert!(!shown_matches(&entry, "rebased"));
    }

    #[test]
    fn the_line_is_taken_whole_and_blank_leaves_everything_in() {
        let entry = shown(&["Reset main"]);
        assert!(!shown_matches(&entry, "main reset"), "not split on spaces");
        assert!(shown_matches(&entry, ""));
        assert!(shown_matches(&entry, "   "));
    }
}
