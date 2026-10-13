//! Where Claude Code keeps its transcripts: one file per session under
//! `projects/<place>/` of its config directory. The layout is Claude
//! Code's own, and a transcript that cannot be found says nothing.

use std::path::{Path, PathBuf};

/// Every transcript file `session` has: one in the directory of each place
/// it worked from, since a session that enters a seat can go on writing
/// beside where it started.
pub(super) fn transcripts(session: &str) -> Vec<PathBuf> {
    projects().map_or_else(Vec::new, |projects| transcripts_in(&projects, session))
}

fn transcripts_in(projects: &Path, session: &str) -> Vec<PathBuf> {
    // The id becomes a file name.
    if session.is_empty()
        || !session
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Vec::new();
    }
    let name = format!("{session}.jsonl");
    let Ok(places) = std::fs::read_dir(projects) else {
        return Vec::new();
    };
    places
        .flatten()
        .map(|place| place.path().join(&name))
        .filter(|path| path.is_file())
        .collect()
}

/// Where Claude Code keeps transcripts: `projects/` under its config
/// directory, one directory per place a session started or worked from.
fn projects() -> Option<PathBuf> {
    std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
                .map(|home| PathBuf::from(home).join(".claude"))
        })
        .map(|config| config.join("projects"))
}

#[cfg(test)]
mod tests {
    use super::transcripts_in;

    #[test]
    fn a_sessions_transcripts_are_found_by_its_id_in_every_place() {
        let yard = crate::yard::Yard::new("transcripts");
        let projects = yard.join("projects");
        let mut made = Vec::new();
        for place in ["repo", "repo--claude-worktrees-b"] {
            std::fs::create_dir_all(projects.join(place)).unwrap();
            let path = projects.join(place).join("s1.jsonl");
            std::fs::write(&path, "{}\n").unwrap();
            made.push(path);
        }
        let mut found = transcripts_in(&projects, "s1");
        found.sort();
        assert_eq!(found, made);
        assert!(transcripts_in(&projects, "nobody").is_empty());
        assert!(transcripts_in(&projects, "../repo/s1").is_empty());
    }
}
