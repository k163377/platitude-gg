//! The versions the shipped build is made from, read at a revision: Qt's
//! (the tree's pin), the toolchain's channel, and every crate the lock resolves
//! for the app's binary — what it links and what builds it, less what only
//! the workspace members' tests build.
//!
//! A narrower question than the gate's (`gate::plan::pins`): the oldest
//! git and the container's base image are what the product is tested
//! against, not what it runs on.

use std::collections::{BTreeMap, BTreeSet};

use crate::subprocess::git_query;

/// The package the shipped binary is built from.
const ROOT: &str = "platitude-app";

pub(super) struct Versions {
    qt: Option<String>,
    toolchain: Option<String>,
    crates: BTreeSet<(String, String)>,
}

/// One dependency whose versions differ between two readings. Empty
/// `old` is a dependency the build gained, empty `new` one it lost.
pub(super) struct Move {
    what: String,
    old: Vec<String>,
    new: Vec<String>,
}

impl std::fmt::Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.old.is_empty(), self.new.is_empty()) {
            (true, _) => write!(f, "+ {} {}", self.what, self.new.join(", ")),
            (_, true) => write!(f, "- {} {}", self.what, self.old.join(", ")),
            _ => write!(
                f,
                "{} {} -> {}",
                self.what,
                self.old.join(", "),
                self.new.join(", ")
            ),
        }
    }
}

impl Versions {
    /// The reading at `rev`; a file the revision does not hold reads as
    /// no version.
    pub(super) fn at(here: &str, rev: &str) -> Self {
        Self::read(&|path| git_query(here, &["show", &format!("{rev}:{path}")]))
    }

    fn read(show: &dyn Fn(&str) -> Option<String>) -> Self {
        Self {
            qt: crate::qt::pinned_at(show),
            toolchain: show("rust-toolchain.toml").and_then(|pin| channel(&pin)),
            crates: show("Cargo.lock")
                .map(|lock| {
                    shipped(&lock, &|member| {
                        show(&format!("crates/{member}/Cargo.toml"))
                            .map(|manifest| dev_only(&manifest))
                            .unwrap_or_default()
                    })
                })
                .unwrap_or_default(),
        }
    }
}

/// What moved from `old` to `new`: Qt, then the toolchain, then the
/// crates by name.
pub(super) fn moved(old: &Versions, new: &Versions) -> Vec<Move> {
    let mut moves = Vec::new();
    for (what, old, new) in [
        ("Qt", &old.qt, &new.qt),
        ("Rust toolchain", &old.toolchain, &new.toolchain),
    ] {
        if old != new {
            moves.push(Move {
                what: what.to_string(),
                old: old.iter().cloned().collect(),
                new: new.iter().cloned().collect(),
            });
        }
    }
    let mut crates: BTreeMap<&str, (Vec<String>, Vec<String>)> = BTreeMap::new();
    for (name, version) in old.crates.difference(&new.crates) {
        crates.entry(name).or_default().0.push(version.clone());
    }
    for (name, version) in new.crates.difference(&old.crates) {
        crates.entry(name).or_default().1.push(version.clone());
    }
    moves.extend(crates.into_iter().map(|(name, (old, new))| Move {
        what: name.to_string(),
        old,
        new,
    }));
    moves
}

/// `channel = "…"` of `rust-toolchain.toml`.
fn channel(pin: &str) -> Option<String> {
    pin.lines()
        .filter_map(|line| line.trim().strip_prefix("channel"))
        .filter_map(|rest| rest.trim_start().strip_prefix('='))
        .map(|value| {
            value
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string()
        })
        .find(|value| !value.is_empty())
}

#[derive(Default)]
struct Package {
    name: String,
    version: String,
    /// No `source`: a workspace member, whose entry lists its
    /// dev-dependencies beside the rest.
    member: bool,
    /// `name` or `name version`, as the lock spells an edge.
    dependencies: Vec<String>,
}

/// Every dependency `(name, version)` the lock resolves for [`ROOT`]'s
/// build. Only a member's entry lists dev-dependencies (cargo resolves no
/// one else's), so a member's are taken off by the names `dev_only` gives
/// for it.
///
/// The lock carries an optional dependency whether or not a feature turns
/// it on, so this can name a crate `cargo tree -e no-dev` does not — one
/// too many, never one missing.
fn shipped(lock: &str, dev_only: &dyn Fn(&str) -> BTreeSet<String>) -> BTreeSet<(String, String)> {
    let packages = packages(lock);
    let mut named: BTreeMap<&str, Vec<&Package>> = BTreeMap::new();
    for package in &packages {
        named.entry(&package.name).or_default().push(package);
    }
    let mut seen = BTreeSet::new();
    let mut next: Vec<&Package> = named
        .get(ROOT)
        .into_iter()
        .flatten()
        .filter(|package| package.member)
        .copied()
        .collect();
    while let Some(package) = next.pop() {
        if !seen.insert((package.name.as_str(), package.version.as_str())) {
            continue;
        }
        let tests_only = if package.member {
            dev_only(&package.name)
        } else {
            BTreeSet::new()
        };
        for edge in &package.dependencies {
            let mut words = edge.split_whitespace();
            let Some(name) = words.next() else {
                continue;
            };
            if tests_only.contains(name) {
                continue;
            }
            let version = words.next();
            next.extend(
                named
                    .get(name)
                    .into_iter()
                    .flatten()
                    .filter(|candidate| version.is_none_or(|v| candidate.version == v)),
            );
        }
    }
    // The members' own version is the product's release, no dependency's.
    packages
        .iter()
        .filter(|package| !package.member)
        .filter(|package| seen.contains(&(package.name.as_str(), package.version.as_str())))
        .map(|package| (package.name.clone(), package.version.clone()))
        .collect()
}

fn packages(lock: &str) -> Vec<Package> {
    let mut packages = Vec::new();
    for block in lock.split("[[package]]").skip(1) {
        let mut package = Package {
            member: true,
            ..Package::default()
        };
        let mut in_list = false;
        for line in block.lines().map(str::trim) {
            if in_list {
                if line.starts_with(']') {
                    in_list = false;
                } else {
                    package
                        .dependencies
                        .push(line.trim_end_matches(',').trim_matches('"').to_string());
                }
                continue;
            }
            // A table after the last package (`[metadata]`) is no package's.
            if line.starts_with('[') {
                break;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim().trim_matches('"');
            match key.trim() {
                "name" => package.name = value.to_string(),
                "version" => package.version = value.to_string(),
                "source" => package.member = false,
                "dependencies" => in_list = value.starts_with('[') && !value.ends_with(']'),
                _ => {}
            }
        }
        packages.push(package);
    }
    packages
}

/// The names a manifest lists under dev-dependencies alone — under
/// `[dev-dependencies]` or a target's, and in no dependencies or
/// build-dependencies table.
fn dev_only(manifest: &str) -> BTreeSet<String> {
    let mut dev = BTreeSet::new();
    let mut built = BTreeSet::new();
    let mut table: Option<bool> = None;
    for line in manifest.lines() {
        if let Some(header) = line.trim().strip_prefix('[') {
            let header = header.trim_end_matches(']');
            table = if header == "dev-dependencies" || header.ends_with(".dev-dependencies") {
                Some(true)
            } else if header.ends_with("dependencies") {
                Some(false)
            } else {
                None
            };
            continue;
        }
        let Some(tests_only) = table else {
            continue;
        };
        if !line.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            continue;
        }
        let key: String = line
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
            .collect();
        let into = if tests_only { &mut dev } else { &mut built };
        into.insert(key);
    }
    dev.difference(&built).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::{Move, Versions, dev_only, moved, shipped};
    use std::collections::BTreeSet;

    const LOCK: &str = "version = 4\n\n\
        [[package]]\nname = \"platitude-app\"\nversion = \"0.0.0\"\ndependencies = [\n \"platitude-core\",\n \"qtbridge\",\n]\n\n\
        [[package]]\nname = \"platitude-core\"\nversion = \"0.0.0\"\ndependencies = [\n \"insta\",\n \"tokio\",\n \"windows-sys 0.61.2\",\n]\n\n\
        [[package]]\nname = \"xtask\"\nversion = \"0.0.0\"\n\n\
        [[package]]\nname = \"qtbridge\"\nversion = \"0.2.1\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\ndependencies = [\n \"cc\",\n]\n\n\
        [[package]]\nname = \"cc\"\nversion = \"1.2.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n\n\
        [[package]]\nname = \"tokio\"\nversion = \"1.47.1\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\ndependencies = [\n \"windows-sys 0.59.0\",\n]\n\n\
        [[package]]\nname = \"insta\"\nversion = \"1.43.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\ndependencies = [\n \"similar\",\n]\n\n\
        [[package]]\nname = \"similar\"\nversion = \"2.7.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n\n\
        [[package]]\nname = \"windows-sys\"\nversion = \"0.59.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n\n\
        [[package]]\nname = \"windows-sys\"\nversion = \"0.61.2\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n";

    const CORE: &str = "[package]\nname = \"platitude-core\"\n\n[dependencies]\n\
        tokio.workspace = true\nwindows-sys = { version = \"0.61\", features = [\n    \"Win32\",\n] }\n\n\
        [dev-dependencies]\ninsta = \"1\"\ntokio = { workspace = true, features = [\"test-util\"] }\n\n[lints]\nworkspace = true\n";

    fn core_dev(member: &str) -> BTreeSet<String> {
        if member == "platitude-core" {
            dev_only(CORE)
        } else {
            BTreeSet::new()
        }
    }

    fn pairs(items: &[(&str, &str)]) -> BTreeSet<(String, String)> {
        items
            .iter()
            .map(|(name, version)| ((*name).to_string(), (*version).to_string()))
            .collect()
    }

    /// A crate both a member's dependencies and its dev-dependencies name
    /// is built; one the dev-dependencies alone name is the tests'.
    #[test]
    fn a_member_s_dev_dependencies_alone_are_left_out() {
        assert_eq!(dev_only(CORE), BTreeSet::from(["insta".to_string()]));
    }

    /// The build is what the app's entry reaches: two versions of one
    /// crate each by its own edge, the tests' crates and whatever only
    /// they pull in left out, and a member the app does not use as well.
    /// The members it does use are the product, not its dependencies.
    #[test]
    fn the_build_is_the_app_s_graph_less_the_tests() {
        assert_eq!(
            shipped(LOCK, &core_dev),
            pairs(&[
                ("cc", "1.2.0"),
                ("qtbridge", "0.2.1"),
                ("tokio", "1.47.1"),
                ("windows-sys", "0.59.0"),
                ("windows-sys", "0.61.2"),
            ])
        );
    }

    /// The product's own release moves no dependency.
    #[test]
    fn a_release_of_the_product_is_no_move() {
        let released = LOCK.replace("\"0.0.0\"", "\"0.1.0\"");
        assert_eq!(shipped(&released, &core_dev), shipped(LOCK, &core_dev));
    }

    fn reading(files: &[(&str, &str)]) -> Versions {
        Versions::read(&|path| {
            files
                .iter()
                .find(|(name, _)| *name == path)
                .map(|(_, text)| (*text).to_string())
        })
    }

    /// Qt and the toolchain first, then the crates by name: an upgrade on
    /// one line, a crate gained or lost with its sign. A tests' crate that
    /// moved is no move of the build.
    #[test]
    fn a_move_names_both_sides() {
        let old = reading(&[
            (crate::qt::PIN, "6.10.3\n"),
            ("rust-toolchain.toml", "[toolchain]\nchannel = \"stable\"\n"),
            ("Cargo.lock", LOCK),
            ("crates/platitude-core/Cargo.toml", CORE),
        ]);
        let lock = LOCK
            .replace("\"0.2.1\"", "\"0.3.0\"")
            .replace("\"1.43.0\"", "\"1.44.0\"")
            .replace(" \"cc\",\n", "")
            .replace("\"2.7.0\"", "\"2.8.0\"")
            .replace(" \"windows-sys 0.59.0\",\n", " \"windows-sys 0.61.2\",\n");
        let new = reading(&[
            (crate::qt::PIN, "6.11.0\n"),
            ("rust-toolchain.toml", "[toolchain]\nchannel = \"stable\"\n"),
            ("Cargo.lock", lock.as_str()),
            ("crates/platitude-core/Cargo.toml", CORE),
        ]);
        let said: Vec<String> = moved(&old, &new).iter().map(Move::to_string).collect();
        assert_eq!(
            said,
            [
                "Qt 6.10.3 -> 6.11.0",
                "- cc 1.2.0",
                "qtbridge 0.2.1 -> 0.3.0",
                "- windows-sys 0.59.0",
            ]
        );
        assert!(moved(&old, &old).is_empty());
    }

    /// A commit from before the pin had a file of its own names its Qt in
    /// CI's workflow: read there, the land that gave the pin its file moved
    /// no version, and one that bumps Qt across it still says so.
    #[test]
    fn the_pin_changing_places_is_no_move_of_qt() {
        let workflow = crate::qt::former_pin();
        let before = reading(&[(workflow.as_str(), "env:\n  QT_VERSION: \"6.12.0\"\n")]);
        let after = reading(&[(crate::qt::PIN, "6.12.0\n"), (workflow.as_str(), "jobs:\n")]);
        assert!(moved(&before, &after).is_empty());
        let bumped = reading(&[(crate::qt::PIN, "6.13.0\n")]);
        let said: Vec<String> = moved(&before, &bumped)
            .iter()
            .map(Move::to_string)
            .collect();
        assert_eq!(said, ["Qt 6.12.0 -> 6.13.0"]);
    }

    /// The toolchain moves on its channel; a component list is no version.
    #[test]
    fn the_toolchain_moves_on_its_channel() {
        let stable = reading(&[(
            "rust-toolchain.toml",
            "[toolchain]\nchannel = \"stable\"\ncomponents = [\"clippy\"]\n",
        )]);
        let components = reading(&[(
            "rust-toolchain.toml",
            "[toolchain]\nchannel = \"stable\"\ncomponents = [\"clippy\", \"rustfmt\"]\n",
        )]);
        let pinned = reading(&[("rust-toolchain.toml", "[toolchain]\nchannel = \"1.91.0\"\n")]);
        assert!(moved(&stable, &components).is_empty());
        let said: Vec<String> = moved(&stable, &pinned)
            .iter()
            .map(Move::to_string)
            .collect();
        assert_eq!(said, ["Rust toolchain stable -> 1.91.0"]);
    }
}
