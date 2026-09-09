//! The stamps: one file per green (step, inputs) pair and one per gated
//! commit, beside the repository's own `.git` so every worktree reads the
//! same ones and a rebase in one seat reuses a run taken in another.
//!
//! Plain files rather than notes or config: a stamp is looked at by a git
//! hook that has to answer in the time a ref update takes, and a file
//! that exists is the cheapest true thing there is.

use std::path::{Path, PathBuf};

use crate::subprocess::git_query;

pub(crate) struct Store {
    root: PathBuf,
}

/// What a commit's stamp says: where main stood when the gate ran, the
/// base the diff was read against, whether the two were one (the branch
/// sat on main), whether both sides ran, and the steps that were green.
pub(crate) struct CommitStamp {
    pub main: String,
    pub base: String,
    pub onto_main: bool,
    pub full: bool,
    pub steps: Vec<String>,
}

impl Store {
    /// The store of the repository `dir` belongs to.
    pub(crate) fn open(dir: &Path) -> Result<Self, String> {
        let common = git_query(
            &dir.display().to_string(),
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        )
        .ok_or_else(|| format!("{} is not a git repository", dir.display()))?;
        Ok(Self {
            root: PathBuf::from(common).join("pgg-gate"),
        })
    }

    pub(crate) fn step_green(&self, key: &str) -> bool {
        self.root.join("steps").join(key).is_file()
    }

    pub(crate) fn mark_step(&self, key: &str, text: &str) -> Result<(), String> {
        write(&self.root.join("steps").join(key), text)
    }

    pub(crate) fn commit(&self, sha: &str) -> Option<CommitStamp> {
        let text = std::fs::read_to_string(self.root.join("commits").join(sha)).ok()?;
        parse(&text)
    }

    pub(crate) fn mark_commit(&self, sha: &str, stamp: &CommitStamp) -> Result<(), String> {
        write(&self.root.join("commits").join(sha), &render(stamp))
    }
}

fn write(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

fn render(stamp: &CommitStamp) -> String {
    let mut text = format!(
        "main={}\nbase={}\nonto_main={}\nfull={}\n",
        stamp.main, stamp.base, stamp.onto_main, stamp.full
    );
    for step in &stamp.steps {
        text.push_str("step=");
        text.push_str(step);
        text.push('\n');
    }
    text
}

fn parse(text: &str) -> Option<CommitStamp> {
    let mut stamp = CommitStamp {
        main: String::new(),
        base: String::new(),
        onto_main: false,
        full: false,
        steps: Vec::new(),
    };
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key {
            "main" => stamp.main = value.to_string(),
            "base" => stamp.base = value.to_string(),
            "onto_main" => stamp.onto_main = value == "true",
            "full" => stamp.full = value == "true",
            "step" => stamp.steps.push(value.to_string()),
            _ => {}
        }
    }
    (!stamp.main.is_empty()).then_some(stamp)
}

#[cfg(test)]
mod tests {
    use super::{CommitStamp, parse, render};

    #[test]
    fn a_stamp_survives_its_own_file() {
        let stamp = CommitStamp {
            main: "aaaa".into(),
            base: "bbbb".into(),
            onto_main: false,
            full: true,
            steps: vec!["test-core 0123".into(), "fmt -".into()],
        };
        let back = parse(&render(&stamp)).expect("parses");
        assert_eq!(back.main, "aaaa");
        assert_eq!(back.base, "bbbb");
        assert!(!back.onto_main);
        assert!(back.full);
        assert_eq!(back.steps, stamp.steps);
        assert!(parse("").is_none(), "an empty file is no stamp");
    }
}
