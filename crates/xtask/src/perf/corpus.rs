//! What the benchmark repository *is*, as one token that changes whenever
//! the measurement would.
//!
//! `HEAD` is the wrong thing to watch and watching it is worse than
//! watching nothing, because it holds still while everything that decides
//! the numbers moves. The graph is `HEAD --branches --remotes --tags`, the
//! memory is dominated by the ref tables, and the interaction is timed
//! against the newest ref-reachable commit — so a `git fetch` that leaves
//! the local branch alone still changes the rows, the ref counts and the
//! commit whose diff is being opened.
//!
//! So the token is taken over the whole ref listing, and the
//! parts that made it are printed beside it so a mismatch says
//! what moved.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Corpus {
    pub(super) token: String,
    pub(super) head: String,
    pub(super) commits: usize,
    pub(super) refs: usize,
    pub(super) tags: usize,
    pub(super) remotes: usize,
    /// Tracked paths the working tree has changed. The app puts a working
    /// -tree row at the top of the graph, so this is part of the scenario.
    pub(super) dirty: usize,
}

impl std::fmt::Display for Corpus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (head={} commits={} refs={} tags={} remotes={} dirty={})",
            self.token,
            self.head.get(..12).unwrap_or(&self.head),
            self.commits,
            self.refs,
            self.tags,
            self.remotes,
            self.dirty
        )
    }
}

pub(super) fn describe(repo: &Path) -> Result<Corpus, String> {
    let refs = read(repo, &["show-ref"])?;
    let head = read(repo, &["rev-parse", "--verify", "--quiet", "HEAD"])
        .unwrap_or_default()
        .trim()
        .to_string();
    let commits = read(repo, &["rev-list", "--all", "--count"])?
        .trim()
        .parse()
        .map_err(|_| "the corpus did not answer a commit count".to_string())?;
    let dirty = read(repo, &["status", "--porcelain=v2", "--untracked-files=no"])?
        .lines()
        .filter(|line| !line.is_empty())
        .count();
    Ok(Corpus {
        token: token(repo, &refs)?,
        head,
        commits,
        tags: count(&refs, " refs/tags/"),
        remotes: count(&refs, " refs/remotes/"),
        refs: refs.lines().filter(|line| !line.is_empty()).count(),
        dirty,
    })
}

/// One line of prose about a corpus that is not the one asked for.
pub(super) fn mismatch(found: &Corpus, wanted: &str) -> Option<String> {
    (!wanted.is_empty() && found.token != wanted).then(|| {
        format!(
            "the benchmark repository is not the one this measurement asked for.\n  \
             wanted {wanted}\n  found  {found}\n\
             A repository that was fetched is a different benchmark: the rows, the ref tables the \
             memory is mostly made of, and the commit the interaction opens all move with it. \
             Re-take the record deliberately, or measure against the corpus the record names."
        )
    })
}

fn count(refs: &str, kind: &str) -> usize {
    refs.lines().filter(|line| line.contains(kind)).count()
}

/// A blob id over the ref listing, taken by the git that is already
/// a dependency.
fn token(repo: &Path, refs: &str) -> Result<String, String> {
    let mut child = Command::new("git")
        .current_dir(repo)
        .args(["hash-object", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not fingerprint the corpus: {e}"))?;
    child
        .stdin
        .take()
        .ok_or("git took no stdin")?
        .write_all(refs.as_bytes())
        .map_err(|e| e.to_string())?;
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("could not fingerprint the corpus".into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn read(repo: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .current_dir(repo)
        .args(args)
        .output()
        .map_err(|e| format!("could not read the corpus: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "the corpus refused `git {}`: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::{Corpus, count, mismatch};

    fn corpus(token: &str) -> Corpus {
        Corpus {
            token: token.into(),
            head: "db1bc5055f24c7227a7d2cc37d058432007d288f".into(),
            commits: 227_051,
            refs: 54_286,
            tags: 46_463,
            remotes: 7_821,
            dirty: 1,
        }
    }

    #[test]
    fn a_corpus_says_what_it_is_in_one_line() {
        let said = corpus("abc123").to_string();
        assert!(said.starts_with("abc123 (head=db1bc5055f24 "), "{said}");
        assert!(said.contains("commits=227051 refs=54286"), "{said}");
    }

    #[test]
    fn only_a_named_and_different_corpus_is_a_mismatch() {
        assert!(mismatch(&corpus("abc"), "").is_none());
        assert!(mismatch(&corpus("abc"), "abc").is_none());
        let complaint = mismatch(&corpus("abc"), "def").expect("a different listing");
        assert!(complaint.contains("wanted def"), "{complaint}");
        assert!(complaint.contains("found  abc ("), "{complaint}");
    }

    #[test]
    fn refs_are_counted_by_kind() {
        let listing =
            "a refs/heads/master\nb refs/tags/v1\nc refs/remotes/o/main\nd refs/tags/v2\n";
        assert_eq!(count(listing, " refs/tags/"), 2);
        assert_eq!(count(listing, " refs/remotes/"), 1);
    }
}
