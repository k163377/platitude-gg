//! The versions the product is built and tested against — the crates',
//! the toolchain's, Qt's, and the oldest git it supports. A branch that
//! moves one owes the full tier, not the reach of its diff: a new version
//! breaks what no source edge leads to (反映前テストの機械化.md
//! §版を動かす変更は段 3).

use super::{CI, DOCKERFILE, core};
use crate::subprocess::git_query;

/// A file that holds a version, and which of its lines carry it.
struct Pin {
    path: String,
    carries: fn(&str) -> bool,
}

/// The core file spelled in pieces for the reason `plan::core` gives.
fn pins() -> [Pin; 5] {
    [
        // Every crate's resolved version; a manifest's requirement moves
        // nothing the lock does not.
        Pin {
            path: "Cargo.lock".to_string(),
            carries: |_| true,
        },
        Pin {
            path: "rust-toolchain.toml".to_string(),
            carries: |_| true,
        },
        Pin {
            path: format!("{}/src/version.rs", core()),
            carries: |line| line.trim_start().starts_with("pub const MINIMUM_GIT"),
        },
        // The base image is the container's git, the oldest supported one
        // (git最低バージョン整合.md), and its system libraries.
        Pin {
            path: DOCKERFILE.to_string(),
            carries: |line| line.trim_start().starts_with("FROM "),
        },
        // Qt's version, which the container and CI install.
        Pin {
            path: CI.to_string(),
            carries: |line| line.trim_start().starts_with("QT_VERSION:"),
        },
    ]
}

/// The first pin the branch moves between `base` and `head`, said as the
/// reason the gate runs the full tier. A side git cannot show (the file
/// added or taken out) counts as a move.
pub(super) fn moved(here: &str, base: &str, head: &str, changed: &[String]) -> Option<String> {
    pins()
        .into_iter()
        .filter(|pin| changed.contains(&pin.path))
        .find(|pin| {
            let at = |rev: &str| git_query(here, &["show", &format!("{rev}:{}", pin.path)]);
            match (at(base), at(head)) {
                (Some(old), Some(new)) => carried(pin, &old) != carried(pin, &new),
                _ => true,
            }
        })
        .map(|pin| format!("{} moves a version", pin.path))
}

fn carried<'a>(pin: &Pin, text: &'a str) -> Vec<&'a str> {
    text.lines()
        .map(str::trim_end)
        .filter(|line| (pin.carries)(line))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{carried, pins};

    fn pin(path_end: &str) -> super::Pin {
        pins()
            .into_iter()
            .find(|pin| pin.path.ends_with(path_end))
            .expect("pin")
    }

    /// The minimum's line moves the pin; the rest of the file does not.
    #[test]
    fn the_git_minimum_is_read_off_its_constant_alone() {
        let version = pin("version.rs");
        let before = "/// doc\npub const MINIMUM_GIT: (u32, u32) = (2, 43);\nfn a() {}\n";
        let edited = "/// other doc\npub const MINIMUM_GIT: (u32, u32) = (2, 43);\nfn b() {}\n";
        let raised = "/// doc\npub const MINIMUM_GIT: (u32, u32) = (2, 47);\nfn a() {}\n";
        assert_eq!(carried(&version, before), carried(&version, edited));
        assert_ne!(carried(&version, before), carried(&version, raised));
    }

    /// Qt's version is the one env line; a job's steps are not.
    #[test]
    fn qt_is_read_off_the_ci_env_line() {
        let ci = pin("ci.yml");
        let before = "env:\n  QT_VERSION: \"6.10.3\"\njobs:\n  test:\n";
        let steps = "env:\n  QT_VERSION: \"6.10.3\"\njobs:\n  lint:\n";
        let bumped = "env:\n  QT_VERSION: \"6.11.0\"\njobs:\n  test:\n";
        assert_eq!(carried(&ci, before), carried(&ci, steps));
        assert_ne!(carried(&ci, before), carried(&ci, bumped));
    }

    /// The base image moves the container's git; a package list does not.
    #[test]
    fn the_containers_git_is_read_off_its_base_images() {
        let docker = pin("Dockerfile");
        let before = "FROM ubuntu:24.04 AS core\nRUN apt-get install git\nFROM core AS app\n";
        let packages =
            "FROM ubuntu:24.04 AS core\nRUN apt-get install git curl\nFROM core AS app\n";
        let newer = "FROM ubuntu:26.04 AS core\nRUN apt-get install git\nFROM core AS app\n";
        assert_eq!(carried(&docker, before), carried(&docker, packages));
        assert_ne!(carried(&docker, before), carried(&docker, newer));
    }
}
