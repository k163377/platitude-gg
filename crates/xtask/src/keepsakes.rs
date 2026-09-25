//! What a container run leaves behind for a person to look at: a host
//! directory mounted at /out, since nobody can open the container's own.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// What one container run leaves on this side, for as long as it has it.
pub(crate) struct Keepsake {
    /// Where the container's mount lands out here.
    pub(crate) dir: PathBuf,
    /// The run's ownership of `dir`, released however the process ends.
    /// `None` only for a path this run had claimed already, which it never
    /// has.
    _claim: Option<crate::verify::ResourceClaim>,
}

/// Where a run's directory lands inside the container.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Landing<'a> {
    /// The directory is the mount itself: a container of the run's own
    /// mounts it there (`linux::in_container`).
    Whole(&'a str),
    /// The mount is the whole [`keepsake_base`], and the run's directory
    /// is a leaf under it: the gate's container mounts the base once
    /// and every run in it names its own leaf (`linux::container::exec_in`).
    Leaf(&'a str),
}

/// Adds the arguments that send a run's pictures out to where `landing`
/// says, and answers where they land on this side. None when the command
/// leaves nothing.
///
/// `--no-board` rides along: the board is the host's (`onto_the_board`).
///
/// The claim is taken on this side: the one the run inside makes lands in
/// the container's own `/tmp`, where it can never refuse anybody.
pub(crate) fn bridge(
    command: &mut Vec<String>,
    landing: Landing<'_>,
) -> Result<Option<Keepsake>, String> {
    let Some(dir) = keepsakes(command)? else {
        return Ok(None);
    };
    let mut mine = BTreeSet::new();
    let claim = crate::verify::claim_resource(&dir, "shot directory", &mut mine)?;
    let inside = match landing {
        Landing::Whole(mount) => mount.to_string(),
        Landing::Leaf(mount) => format!(
            "{mount}/{}",
            dir.file_name()
                .ok_or_else(|| format!("{} has no name to land under", dir.display()))?
                .to_string_lossy()
        ),
    };
    command.push("--shot-dir".to_string());
    command.push(inside);
    command.push("--no-board".to_string());
    println!("screenshots and settings: {}", dir.display());
    Ok(Some(Keepsake { dir, _claim: claim }))
}

/// The directory every container run's leaf is claimed under
/// ([`keepsake_dir`]), made if it is not there: what the gate's
/// container mounts whole.
pub(crate) fn keepsake_base() -> Result<PathBuf, String> {
    let base = std::env::temp_dir().join("pgg-linux");
    std::fs::create_dir_all(&base)
        .map_err(|e| format!("could not make {}: {e}", base.display()))?;
    Ok(base)
}

/// Whatever the run left behind (`out` = what `bridge` answered), onto
/// the host's board.
///
/// Marked `— linux`: Done is both OSes photographed for the same verb
/// (CLAUDE.md ビルド・テスト), so each picture has to say its side.
///
/// The command has to be the one that was typed — see [`boarding`].
pub(crate) fn onto_the_board(out: Option<&Path>, command: &[String]) {
    let Some(out) = out else {
        return;
    };
    if !boarding(command) {
        return;
    }
    let (label, verb) = naming(command);
    match crate::shots::record_dir(out, &label, &verb) {
        Ok(page) => println!("board: {}", crate::shots::shown(&page)),
        // The board is not what a container run is judging.
        Err(message) => println!("board: not updated ({message})"),
    }
}

/// Whether the run's pictures are filed on the board once it is over.
/// Only the line as typed can answer: [`bridge`] adds `--no-board` to
/// every line it hands the container. A suite's runs are typed with it
/// (`verify::suite_words`).
fn boarding(command: &[String]) -> bool {
    !command.iter().any(|word| word == "--no-board")
}

/// What to call the run on the board, out of the command line it was.
fn naming(command: &[String]) -> (String, String) {
    let word_after = |flag: &str| {
        command
            .iter()
            .position(|word| word == flag)
            .and_then(|at| command.get(at + 1))
            .cloned()
    };
    let verb = word_after("verify-ui").unwrap_or_default();
    let label = word_after("--label").unwrap_or_else(|| verb.clone());
    (format!("{label} — linux"), verb)
}

/// A host directory for what verify-ui writes to --shot-dir (screenshot
/// and settings), or None when the command leaves nothing.
///
/// One per run: `/out` is the same path in every container, so a second
/// run handed the same directory finds `/out/config` held
/// (`settings::Store::claim`) and waits out its watchdog.
fn keepsakes(command: &[String]) -> Result<Option<PathBuf>, String> {
    if !command.iter().any(|word| word == "verify-ui") {
        return Ok(None);
    }
    if command.iter().any(|word| word == "--shot-dir") {
        // Named by the caller, who then owns where it lands.
        return Ok(None);
    }
    keepsake_dir("shots").map(Some)
}

pub(crate) fn keepsake_dir(kind: &str) -> Result<PathBuf, String> {
    crate::verify::claim_dir(&keepsake_base()?, kind)
}

#[cfg(test)]
mod tests {
    use super::{Landing, boarding, bridge, keepsakes, naming};

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn a_container_run_is_named_after_its_verb_and_its_side() {
        assert_eq!(
            naming(&words("cargo xtask verify-ui commit-menu")),
            ("commit-menu — linux".to_string(), "commit-menu".to_string())
        );
        let told = [
            "cargo",
            "xtask",
            "verify-ui",
            "row-card",
            "--label",
            "チップのバッジ",
        ]
        .map(String::from);
        assert_eq!(
            naming(&told),
            ("チップのバッジ — linux".to_string(), "row-card".to_string())
        );
    }

    /// `bridge` writes `--no-board` into every line it sends in, so a
    /// caller that asked the mutated line would file nothing at all.
    #[test]
    fn what_the_container_was_told_cannot_answer_for_the_board() {
        let typed = words("cargo xtask verify-ui commit");
        assert!(boarding(&typed));
        let mut sent = typed.clone();
        let out = bridge(&mut sent, Landing::Whole("/out"))
            .expect("a bridged run")
            .expect("a directory to bring the pictures back to");
        assert!(!boarding(&sent), "bridge says it whether the caller did");
        assert!(
            sent.windows(2).any(|pair| pair == ["--shot-dir", "/out"]),
            "a container of the run's own mounts the directory at /out: {sent:?}"
        );
        std::fs::remove_dir_all(&out.dir).expect("the directory bridge just made");

        // In the gate's container the mount is the whole base, and the
        // run names the directory this side claimed as a leaf of it.
        let mut leafed = typed.clone();
        let out = bridge(&mut leafed, Landing::Leaf("/out"))
            .expect("a bridged run")
            .expect("a directory to bring the pictures back to");
        let leaf = out.dir.file_name().expect("a leaf").to_string_lossy();
        assert!(
            leafed
                .windows(2)
                .any(|pair| pair[0] == "--shot-dir" && pair[1] == format!("/out/{leaf}")),
            "the run in the gate's container names its own leaf: {leafed:?}"
        );
        assert!(
            out.dir
                .starts_with(super::keepsake_base().expect("the base"))
        );
        std::fs::remove_dir_all(&out.dir).expect("the directory bridge just made");

        let suite = crate::verify::suite_words("commit");
        assert!(!boarding(&suite), "a suite's run stays off the board");
    }

    #[test]
    fn only_a_verify_run_without_a_directory_of_its_own_gets_one() {
        assert!(
            keepsakes(&words("cargo xtask verify-ui commit"))
                .expect("temp dir")
                .is_some()
        );
        assert_eq!(
            keepsakes(&words("cargo xtask verify-ui commit --shot-dir /somewhere")).expect("none"),
            None
        );
        assert_eq!(
            keepsakes(&words("cargo test -p platitude-core")).expect("none"),
            None
        );
    }

    /// The gate starts a side's verbs together (`gate::sides::verbs`),
    /// where a name from a clock read inside one tick would hand two of
    /// them one `/out`.
    #[test]
    fn container_runs_started_together_are_handed_a_directory_each() {
        let start = std::sync::Arc::new(std::sync::Barrier::new(16));
        let threads: Vec<_> = (0..16)
            .map(|_| {
                let start = start.clone();
                std::thread::spawn(move || {
                    start.wait();
                    super::keepsake_dir("shots").expect("a directory for this run")
                })
            })
            .collect();
        let made: Vec<_> = threads
            .into_iter()
            .map(|thread| thread.join().expect("a claiming thread"))
            .collect();

        let unique: std::collections::BTreeSet<_> = made.iter().collect();
        assert_eq!(unique.len(), made.len(), "two runs were handed one /out");
        for dir in made {
            // Empty, and so nobody else's: the claim made it, which
            // is what `create_dir_all` could not say.
            std::fs::remove_dir(&dir).expect("an empty directory this call created");
        }
    }
}
