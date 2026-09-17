//! Whether the first run's discard is owed at all.
//!
//! The first run of an invocation is discarded because it pays for a
//! cold cache — the exe, the Qt libraries, the corpus's packs — and the
//! record is a warm number. But a record is taken as a run of
//! invocations minutes apart on one exe and one corpus, and only the
//! first of them starts cold: over one such session only the very first
//! invocation's first run stood out, while every later invocation's first
//! run sat among the kept ones (ci/baseline/code-costs-windows-x64.md
//! §テストとハーネス), so each of those discards was a whole run spent on
//! a number already known. An invocation that finished
//! leaves a note of what it warmed; the next one keeps its first run
//! when the note is minutes old, names the same exe, corpus, repository
//! and scenario, and no build of another process has ended since — a
//! build is what evicts the corpus's packs, and every build here is
//! announced (`still::build_ended_since`).
//!
//! The scenario is part of what was warmed: a stage that selected no row
//! never read the trees and blobs a `git show` opens, so the next stage's
//! first details number would be the cold one — the number that is over
//! budget.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::note::{field, now_secs};
use crate::seats::same_tree;

/// How long the cache is trusted to stay warm with no build in between.
/// Long enough for the stage tables of a record, which follow each other
/// by a build check and a minute of runs.
const WARM_FOR: Duration = Duration::from_secs(10 * 60);

/// What an invocation warmed: the exe by its blob id, the corpus by its
/// token (`-` with no repository), the repository's path — the same
/// corpus at another path is other files — and the scenario the runs
/// drove.
pub(super) struct Warmed {
    exe: String,
    corpus: String,
    repo: String,
    scenario: String,
    at: u64,
}

impl Warmed {
    pub(super) fn now(exe: &str, corpus: &str, repo: &str, scenario: &str) -> Self {
        Self {
            exe: exe.to_string(),
            corpus: corpus.to_string(),
            repo: repo.to_string(),
            scenario: scenario.to_string(),
            at: now_secs(),
        }
    }

    fn text(&self) -> String {
        format!(
            "exe {}\ncorpus {}\nrepo {}\nscenario {}\nat {}\n",
            self.exe, self.corpus, self.repo, self.scenario, self.at
        )
    }

    fn parse(text: &str) -> Option<Self> {
        Some(Self {
            exe: field(text, "exe ")?.to_string(),
            corpus: field(text, "corpus ")?.to_string(),
            repo: field(text, "repo ")?.to_string(),
            scenario: field(text, "scenario ")?.to_string(),
            at: field(text, "at ")?.parse().ok()?,
        })
    }

    /// How long ago this note warmed exactly what `wanted` names, or
    /// nothing: another exe, corpus, path or scenario, or too long ago.
    fn covers(&self, wanted: &Warmed, now: u64) -> Option<Duration> {
        let same = self.exe == wanted.exe
            && self.corpus == wanted.corpus
            && self.scenario == wanted.scenario
            && same_tree(&self.repo, &wanted.repo);
        let age = Duration::from_secs(now.checked_sub(self.at)?);
        (same && age <= WARM_FOR).then_some(age)
    }
}

/// How long ago an invocation from this tree warmed what `wanted` names,
/// if recently enough and with no other process's build ended since —
/// the first run then need not be discarded.
pub(super) fn warm(root: &Path, wanted: &Warmed) -> Option<Duration> {
    let text = std::fs::read_to_string(note_path(root)).ok()?;
    let note = Warmed::parse(&text)?;
    let age = note.covers(wanted, now_secs())?;
    (!crate::still::build_ended_since(root, note.at)).then_some(age)
}

/// Leaves the note for the next invocation, stamped now — the
/// runs are what warmed the cache, and they ended now. A note
/// that could not be written costs the next invocation one
/// discarded run, and says so.
pub(super) fn note(root: &Path, warmed: &Warmed) {
    let path = note_path(root);
    let stamped = Warmed::now(&warmed.exe, &warmed.corpus, &warmed.repo, &warmed.scenario);
    let written = path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(&path, stamped.text()));
    if let Err(error) = written {
        println!(
            "  note: could not leave the warm note at {} ({error}) — the next invocation \
             discards its first run",
            path.display()
        );
    }
}

/// Beside the evidence, so that it is cleared with it.
fn note_path(root: &Path) -> PathBuf {
    root.join("target").join("perf").join("warm.txt")
}

#[cfg(test)]
mod tests {
    use super::{WARM_FOR, Warmed};

    fn warmed(exe: &str, corpus: &str, repo: &str, scenario: &str, at: u64) -> Warmed {
        Warmed {
            exe: exe.into(),
            corpus: corpus.into(),
            repo: repo.into(),
            scenario: scenario.into(),
            at,
        }
    }

    /// The same four names, minutes ago, is a warm cache; any other exe,
    /// corpus, path or scenario is not, and neither is the same one too
    /// long ago.
    #[test]
    fn only_the_same_exe_corpus_path_and_scenario_warmed_recently_counts() {
        let then = warmed("e1", "c1", "C:/x/corpus", "first/true/true", 1_000);
        // Windows spells a path in whatever case the writer used, and one
        // path spelled two ways is one path only where the OS says so.
        let spelled = warmed("e1", "c1", "c:\\X\\corpus", "first/true/true", 1_500);
        assert_eq!(
            then.covers(&spelled, 1_500).map(|age| age.as_secs()),
            if cfg!(windows) { Some(500) } else { None }
        );
        let same = warmed("e1", "c1", "C:/x/corpus", "first/true/true", 1_500);
        assert_eq!(
            then.covers(&same, 1_500).map(|age| age.as_secs()),
            Some(500)
        );
        assert!(then.covers(&same, 1_000 + WARM_FOR.as_secs() + 1).is_none());
        for other in [
            warmed("e2", "c1", "C:/x/corpus", "first/true/true", 0),
            warmed("e1", "c2", "C:/x/corpus", "first/true/true", 0),
            warmed("e1", "c1", "C:/y/corpus", "first/true/true", 0),
            warmed("e1", "c1", "C:/x/corpus", "none/false/false", 0),
        ] {
            assert!(then.covers(&other, 1_100).is_none());
        }
        // A note from the future is nobody's: the clock moved.
        assert!(then.covers(&same, 900).is_none());
    }

    #[test]
    fn a_note_reads_back_as_it_was_written() {
        let note = Warmed::now("abc", "-", "C:/x/repo", "first/true/true/true//");
        let back = Warmed::parse(&note.text()).expect("a whole note parses");
        assert_eq!(
            (
                back.exe.as_str(),
                back.corpus.as_str(),
                back.repo.as_str(),
                back.scenario.as_str(),
                back.at
            ),
            ("abc", "-", "C:/x/repo", "first/true/true/true//", note.at)
        );
        assert!(Warmed::parse("something else").is_none());
    }
}
