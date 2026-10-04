//! `cargo xtask versions` — the versions a file format gives no way to
//! name once, held to one. Each place that needs one writes it, and the
//! copies agree with their source:
//!
//! - every runner name of an OS across the workflows and the local actions
//!   is one release, and Ubuntu's is the one the Dockerfile's `UBUNTU`
//!   names: the container stands for CI's Linux machine. A `-latest` name
//!   names no release at all. The Dockerfile's own stages that start from
//!   Ubuntu take the release from that line.
//! - every action they use is at one commit under one release comment, as
//!   Dependabot rewrites them all at once.
//!
//! What a file names once is not read here: the crates (`Cargo.lock`), the
//! toolchain (`rust-toolchain.toml`), Qt (`.qt-version`), and aqtinstall
//! (the Dockerfile's `AQT_SOURCE`, which CI's Qt action reads).

use std::collections::BTreeMap;
use std::path::Path;

use crate::command::{self, Permission, Where};

pub(crate) static VERSIONS: command::Command = command::Command {
    id: "versions.agree",
    call: "versions",
    purpose: "the versions the workflows copy agree with their source",
    run_in: Where::Either,
    needs: &[],
    permission: Permission::Plain,
};

pub(crate) static COMMANDS: &[&command::Command] = &[&VERSIONS];

const DOCKERFILE: &str = "ci/linux/Dockerfile";

/// The line of the Dockerfile that names the Ubuntu release, and how a
/// stage that starts from Ubuntu takes it.
const UBUNTU_ARG: &str = "ARG UBUNTU=";
const UBUNTU_IMAGE: &str = "FROM ubuntu:";
const UBUNTU_TAG: &str = "${UBUNTU}";

/// The OS a GitHub runner name starts with.
const FAMILIES: [&str; 3] = ["ubuntu", "windows", "macos"];

/// Where the workflows and the local actions sit, under CI's directory.
const WORKFLOWS: &str = "workflows";
const ACTIONS: &str = "actions";

pub fn run(args: &[String]) -> Result<(), String> {
    if let Some(arg) = args.first() {
        return Err(format!("unknown option {arg:?} (versions takes none)"));
    }
    let root = crate::tree::workspace_root();
    let docker = read(&root, DOCKERFILE)?;
    let files = workflow_files(&root)?;
    let found = disagreements(&docker, &files);
    if found.is_empty() {
        println!(
            "versions: ok — {} workflow files, each OS's runners at one release and every \
             action at one commit",
            files.len()
        );
        return Ok(());
    }
    for line in &found {
        println!("versions: {line}");
    }
    Err(format!("{} version copies disagree", found.len()))
}

fn read(root: &Path, relative: &str) -> Result<String, String> {
    let path = root.join(relative);
    std::fs::read_to_string(&path).map_err(|e| format!("failed to read {}: {e}", path.display()))
}

/// Every workflow, and every local action's `action.yml`, as (path from
/// the root, text), in path order.
fn workflow_files(root: &Path) -> Result<Vec<(String, String)>, String> {
    // In pieces: whole, the name would make this file a reader of CI's,
    // and an edit there would reach the runner (`graph::tests::
    // nothing_reads_a_file_of_cis`). This check is an always-step: it
    // runs whatever the edit.
    let github = root.join(format!(".{}", "github"));
    let listed = |dir: &Path| -> Result<Vec<std::path::PathBuf>, String> {
        let entries =
            std::fs::read_dir(dir).map_err(|e| format!("failed to list {}: {e}", dir.display()))?;
        let mut paths: Vec<_> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect();
        paths.sort();
        Ok(paths)
    };
    let yaml = |path: &Path| {
        path.extension()
            .is_some_and(|ext| ext == "yml" || ext == "yaml")
    };
    let mut paths: Vec<_> = listed(&github.join(WORKFLOWS))?
        .into_iter()
        .filter(|path| yaml(path))
        .collect();
    for action in listed(&github.join(ACTIONS))? {
        paths.extend(
            ["action.yml", "action.yaml"]
                .map(|name| action.join(name))
                .into_iter()
                .filter(|path| path.is_file()),
        );
    }
    paths
        .into_iter()
        .map(|path| {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let text = std::fs::read_to_string(&path)
                .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
            Ok((relative, text))
        })
        .collect()
}

/// A place in a file, as `path:line`.
type At = String;

/// What disagrees, one line each; empty when every copy agrees.
fn disagreements(docker: &str, files: &[(String, String)]) -> Vec<String> {
    let mut found = Vec::new();
    let ubuntu = docker
        .lines()
        .find_map(|line| line.trim().strip_prefix(UBUNTU_ARG))
        .map(|release| format!("ubuntu-{}", release.trim().trim_matches('"')));
    if ubuntu.is_none() {
        found.push(format!(
            "{DOCKERFILE} names no Ubuntu release ({UBUNTU_ARG}…)"
        ));
    }
    // Its own base stages take the release from the line, not a copy.
    for (index, line) in docker.lines().enumerate() {
        let tag = line
            .trim()
            .strip_prefix(UBUNTU_IMAGE)
            .and_then(|rest| rest.split_whitespace().next());
        if let Some(tag) = tag.filter(|tag| *tag != UBUNTU_TAG) {
            found.push(format!(
                "{DOCKERFILE}:{} starts from ubuntu:{tag}, not ubuntu:{UBUNTU_TAG}",
                index + 1
            ));
        }
    }
    let mut runners: BTreeMap<&str, BTreeMap<String, Vec<At>>> = BTreeMap::new();
    let mut actions: BTreeMap<String, BTreeMap<String, Vec<At>>> = BTreeMap::new();
    for (path, text) in files {
        for (index, line) in text.lines().enumerate() {
            let at = format!("{path}:{}", index + 1);
            for (family, name) in runner_names(line) {
                runners
                    .entry(family)
                    .or_default()
                    .entry(name)
                    .or_default()
                    .push(at.clone());
            }
            if let Some((repo, pinned)) = action_use(line) {
                actions
                    .entry(repo)
                    .or_default()
                    .entry(pinned)
                    .or_default()
                    .push(at.clone());
            }
        }
    }
    for (family, names) in &runners {
        for (name, places) in names {
            if name.ends_with("-latest") {
                found.push(format!("{name} names no release ({})", places.join(", ")));
            }
        }
        if names.len() > 1 {
            found.push(format!(
                "{family} runners at {} releases: {}",
                names.len(),
                listing(names)
            ));
        }
    }
    if let (Some(ubuntu), Some(names)) = (&ubuntu, runners.get("ubuntu")) {
        for (name, places) in names.iter().filter(|(name, _)| *name != ubuntu) {
            found.push(format!(
                "{name} is not the release {DOCKERFILE} names ({ubuntu}): {}",
                places.join(", ")
            ));
        }
    }
    for (repo, pins) in &actions {
        if pins.len() > 1 {
            found.push(format!(
                "{repo} at {} commits: {}",
                pins.len(),
                listing(pins)
            ));
        }
    }
    found
}

fn listing(copies: &BTreeMap<String, Vec<At>>) -> String {
    copies
        .iter()
        .map(|(copy, places)| format!("{copy} ({})", places.join(", ")))
        .collect::<Vec<_>>()
        .join(" / ")
}

/// The runner names a line holds, with their OS, as GitHub reads a label
/// (case aside): the OS, a dash, then a release in digits — `macos-26`,
/// `ubuntu-24.04`, the `24.04` of `ubuntu-24.04-arm` — or `latest`. The
/// name stands as a word: the `windows` of a target triple is not one, nor
/// is a package such as `windows-sys` or `ubuntu-keyring`, and a sentence's
/// full stop is not part of the release.
fn runner_names(line: &str) -> Vec<(&'static str, String)> {
    let line = line.to_ascii_lowercase();
    let word = |c: char| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_';
    let mut names = Vec::new();
    for family in FAMILIES {
        let prefix = format!("{family}-");
        for (start, _) in line.match_indices(&prefix) {
            if line[..start].chars().next_back().is_some_and(word) {
                continue;
            }
            let rest = &line[start + prefix.len()..];
            let digits: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            let release = digits.trim_end_matches('.');
            let latest = rest.strip_prefix("latest").is_some_and(|after| {
                !after
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphanumeric())
            });
            if release.starts_with(|c: char| c.is_ascii_digit()) {
                names.push((family, format!("{prefix}{release}")));
            } else if latest {
                names.push((family, format!("{prefix}latest")));
            }
        }
    }
    names
}

/// A `uses:` line's action, as its repository and what pins it: the ref
/// and the release comment beside it. A local action (`./…`) and an image
/// (`docker://…`) are not pinned here.
fn action_use(line: &str) -> Option<(String, String)> {
    let line = line.trim_start();
    let line = line.strip_prefix("- ").unwrap_or(line).trim_start();
    let value = line.strip_prefix("uses:")?;
    let (spec, comment) = value.split_once('#').unwrap_or((value, ""));
    let spec = spec.trim().trim_matches(|c| c == '"' || c == '\'');
    if spec.starts_with("./") || spec.starts_with("docker://") {
        return None;
    }
    let (path, reference) = spec.split_once('@')?;
    let repo = path.split('/').take(2).collect::<Vec<_>>().join("/");
    let comment = comment.trim();
    let pinned = if comment.is_empty() {
        reference.to_string()
    } else {
        format!("{reference} # {comment}")
    };
    Some((repo, pinned))
}

#[cfg(test)]
mod tests {
    use super::{action_use, disagreements, runner_names};

    const DOCKER: &str = "ARG UBUNTU=24.04\nFROM ubuntu:${UBUNTU} AS core\n";

    fn files(texts: &[(&str, &str)]) -> Vec<(String, String)> {
        texts
            .iter()
            .map(|(path, text)| ((*path).to_string(), (*text).to_string()))
            .collect()
    }

    const CI: &str = "\
jobs:
  test:
    strategy:
      matrix:
        os:
          - &ubuntu ubuntu-24.04
          - windows-2025
          - macos-26
    steps:
      - &checkout
        uses: actions/checkout@aaaa # v7.0.1
      - uses: ./local/qt
  deny:
    runs-on: *ubuntu
";

    const SHOTS: &str = "\
jobs:
  shots:
    runs-on: macos-26
    steps:
      - uses: actions/checkout@aaaa # v7.0.1
";

    #[test]
    fn copies_that_agree_say_nothing() {
        let found = disagreements(DOCKER, &files(&[("ci.yml", CI), ("shots.yml", SHOTS)]));
        assert!(found.is_empty(), "{found:?}");
    }

    /// Ubuntu's runners follow the Dockerfile, whichever file names them.
    #[test]
    fn an_ubuntu_runner_off_the_dockerfiles_release_is_named() {
        let found = disagreements(
            "ARG UBUNTU=26.04\n",
            &files(&[("ci.yml", CI), ("shots.yml", SHOTS)]),
        );
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].starts_with("ubuntu-24.04 is not the release"),
            "{found:?}"
        );
        assert!(found[0].contains("ci.yml:6"), "{found:?}");
    }

    #[test]
    fn one_os_at_two_releases_is_named_with_both_places() {
        let newer = SHOTS.replace("macos-26", "macos-27");
        let found = disagreements(DOCKER, &files(&[("ci.yml", CI), ("shots.yml", &newer)]));
        assert_eq!(
            found,
            ["macos runners at 2 releases: macos-26 (ci.yml:8) / macos-27 (shots.yml:3)"]
        );
    }

    #[test]
    fn a_latest_runner_names_no_release() {
        let latest = SHOTS.replace("macos-26", "macos-latest");
        let found = disagreements(DOCKER, &files(&[("shots.yml", &latest)]));
        assert_eq!(found, ["macos-latest names no release (shots.yml:3)"]);
    }

    #[test]
    fn an_action_at_two_commits_is_named() {
        let moved = SHOTS.replace("aaaa # v7.0.1", "bbbb # v7.1.0");
        let found = disagreements(DOCKER, &files(&[("ci.yml", CI), ("shots.yml", &moved)]));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].starts_with("actions/checkout at 2 commits"),
            "{found:?}"
        );
    }

    /// `actions/cache/restore` and `actions/cache/save` are one repository,
    /// so one commit.
    #[test]
    fn an_actions_sub_path_is_its_repository() {
        assert_eq!(
            action_use("      uses: actions/cache/restore@cccc # v6.1.0"),
            Some(("actions/cache".to_string(), "cccc # v6.1.0".to_string()))
        );
        assert_eq!(action_use("      - uses: ./local/qt"), None);
        assert_eq!(action_use("        uses: docker://alpine:3"), None);
    }

    /// A stage that writes the release out is a second copy of it.
    #[test]
    fn a_base_stage_off_the_arg_is_named() {
        let docker = format!("{DOCKER}FROM ubuntu:24.04 AS bare\n");
        let found = disagreements(&docker, &files(&[("ci.yml", CI)]));
        assert_eq!(
            found,
            ["ci/linux/Dockerfile:3 starts from ubuntu:24.04, not ubuntu:${UBUNTU}"]
        );
    }

    #[test]
    fn a_runner_name_is_an_os_and_a_release() {
        assert!(runner_names("cargo tree -i windows-sys").is_empty());
        assert!(runner_names("apt-get install ubuntu-keyring ubuntu-desktop-minimal").is_empty());
        assert_eq!(
            runner_names("# CI runs on macos-26."),
            [("macos", "macos-26".to_string())]
        );
        assert_eq!(
            runner_names("    runs-on: Ubuntu-24.04-arm"),
            [("ubuntu", "ubuntu-24.04".to_string())]
        );
        assert_eq!(
            runner_names("runs-on: windows-latest"),
            [("windows", "windows-latest".to_string())]
        );
        assert!(runner_names("ubuntu-latestish").is_empty());
    }

    #[test]
    fn a_runner_name_stands_as_a_word() {
        assert!(runner_names("stable-x86_64-pc-windows-msvc").is_empty());
        assert!(runner_names("host=windows arch=win64_msvc2022_64").is_empty());
        assert!(runner_names("Named, not -latest: a new release").is_empty());
        assert_eq!(
            runner_names("    os: [ubuntu-24.04, windows-2025]"),
            [
                ("ubuntu", "ubuntu-24.04".to_string()),
                ("windows", "windows-2025".to_string())
            ]
        );
    }
}
