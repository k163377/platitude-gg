//! Whether the first run's discard is owed at all.
//!
//! The first run of an invocation is discarded because it pays for a
//! cold cache — the exe, the Qt libraries, the corpus's packs — and the
//! record is a warm number. But a record is taken as a run of
//! invocations minutes apart on one exe and one corpus, and only the
//! first of them starts cold: measured over one such session, the first
//! invocation's first run stood out (startup 1226ms) while every later
//! invocation's first run sat among the kept ones (1081–1178ms), so each
//! of those discards was fourteen seconds spent on a number already
//! known. An invocation that finished leaves a note of what it warmed;
//! the next one keeps its first run when the note is minutes old and
//! names the same exe, corpus and repository.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// How long the cache is trusted to stay warm. Long enough for the stage
/// tables of a record, which follow each other by a build check and a
/// minute of runs; short enough that a release build in between — which
/// is what evicts the corpus's packs — has probably not happened.
const WARM_FOR: Duration = Duration::from_secs(10 * 60);

/// What an invocation warmed: the exe by its blob id, the corpus by its
/// token (`-` with no repository), and the repository's path, because the
/// same corpus at another path is other files.
pub(super) struct Warmed {
    exe: String,
    corpus: String,
    repo: String,
    at: u64,
}

impl Warmed {
    pub(super) fn now(exe: &str, corpus: &str, repo: &str) -> Self {
        Self {
            exe: exe.to_string(),
            corpus: corpus.to_string(),
            repo: repo.replace('\\', "/"),
            at: now_secs(),
        }
    }

    fn text(&self) -> String {
        format!(
            "exe {}\ncorpus {}\nrepo {}\nat {}\n",
            self.exe, self.corpus, self.repo, self.at
        )
    }

    fn parse(text: &str) -> Option<Self> {
        let field = |key: &str| {
            text.lines()
                .find_map(|line| line.strip_prefix(key))
                .map(str::trim)
        };
        Some(Self {
            exe: field("exe ")?.to_string(),
            corpus: field("corpus ")?.to_string(),
            repo: field("repo ")?.to_string(),
            at: field("at ")?.parse().ok()?,
        })
    }

    /// How long ago this note warmed exactly what `wanted` names, or
    /// nothing: another exe, another corpus, another path, or too long
    /// ago.
    fn covers(&self, wanted: &Warmed, now: u64) -> Option<Duration> {
        let same = self.exe == wanted.exe
            && self.corpus == wanted.corpus
            && self.repo.eq_ignore_ascii_case(&wanted.repo);
        let age = Duration::from_secs(now.checked_sub(self.at)?);
        (same && age <= WARM_FOR).then_some(age)
    }
}

/// How long ago an invocation from this tree warmed what `wanted` names,
/// if recently enough that the first run need not be discarded.
pub(super) fn warm(root: &Path, wanted: &Warmed) -> Option<Duration> {
    let text = std::fs::read_to_string(note_path(root)).ok()?;
    Warmed::parse(&text)?.covers(wanted, now_secs())
}

/// Leaves the note for the next invocation. A note that could not be
/// written costs the next invocation one discarded run, and says so.
pub(super) fn note(root: &Path, warmed: &Warmed) {
    let path = note_path(root);
    let written = path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(&path, warmed.text()));
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

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

#[cfg(test)]
mod tests {
    use super::{WARM_FOR, Warmed};

    fn warmed(exe: &str, corpus: &str, repo: &str, at: u64) -> Warmed {
        Warmed {
            exe: exe.into(),
            corpus: corpus.into(),
            repo: repo.into(),
            at,
        }
    }

    /// The same three names, minutes ago, is a warm cache; any other
    /// exe, corpus or path is not, and neither is the same one too long
    /// ago.
    #[test]
    fn only_the_same_exe_corpus_and_path_warmed_recently_counts() {
        let then = warmed("e1", "c1", "C:/x/corpus", 1_000);
        // Windows spells a path in whatever case the writer used.
        let same = warmed("e1", "c1", "c:/X/corpus", 1_500);
        assert_eq!(
            then.covers(&same, 1_500).map(|age| age.as_secs()),
            Some(500)
        );
        assert!(then.covers(&same, 1_000 + WARM_FOR.as_secs() + 1).is_none());
        assert!(
            then.covers(&warmed("e2", "c1", "C:/x/corpus", 0), 1_100)
                .is_none()
        );
        assert!(
            then.covers(&warmed("e1", "c2", "C:/x/corpus", 0), 1_100)
                .is_none()
        );
        assert!(
            then.covers(&warmed("e1", "c1", "C:/y/corpus", 0), 1_100)
                .is_none()
        );
        // A note from the future is nobody's: the clock moved.
        assert!(then.covers(&same, 900).is_none());
    }

    #[test]
    fn a_note_reads_back_as_it_was_written() {
        let note = Warmed::now("abc", "-", "C:\\x\\repo");
        let back = Warmed::parse(&note.text()).expect("a whole note parses");
        assert_eq!(
            (
                back.exe.as_str(),
                back.corpus.as_str(),
                back.repo.as_str(),
                back.at
            ),
            ("abc", "-", "C:/x/repo", note.at)
        );
        assert!(Warmed::parse("something else").is_none());
    }
}
