//! What a container run leaves behind for a person to look at.
//!
//! A run inside the container writes where nobody can open it, so
//! anything meant to be read afterwards goes to a host directory bridged
//! in over /out (`linux`).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// What one container run leaves on this side, for as long as it has it.
pub(crate) struct Keepsake {
    /// Where the container's mount lands out here.
    pub(crate) dir: PathBuf,
    /// Never read. Holding it *is* the run's ownership of `dir`, and it
    /// is released however the process ends. `None` would mean this run
    /// had already claimed the path, which it never has: the directory
    /// was made a line earlier.
    _claim: Option<crate::verify::ResourceClaim>,
}

/// The arguments that send a run's pictures out to `mount`, and where
/// they land on this side. None when the command leaves nothing.
///
/// `--no-board` rides along: the board is the host's. The run in there
/// keeps its census and its board out of the tree by itself
/// (`verify::options`, on `linux::IN_CONTAINER`), and the seat these
/// pictures belong to is the one out here — which is what
/// `onto_the_board` is for, once the run is done.
///
/// **The claim is taken here rather than in there.** The run inside makes
/// one too (`verify::run` claims its `--shot-dir`), but it writes that
/// lock into the container's own `/tmp`, which is empty in every
/// container — a claim that can never refuse anybody. What two runs can
/// actually collide over is this directory, and it is only on this side
/// that a second asker can be told so.
pub(crate) fn bridge(command: &mut Vec<String>, mount: &str) -> Result<Option<Keepsake>, String> {
    let Some(dir) = keepsakes(command)? else {
        return Ok(None);
    };
    let mut mine = BTreeSet::new();
    let claim = crate::verify::claim_resource(&dir, "shot directory", &mut mine)?;
    command.push("--shot-dir".to_string());
    command.push(mount.to_string());
    command.push("--no-board".to_string());
    println!("screenshots and settings: {}", dir.display());
    Ok(Some(Keepsake { dir, _claim: claim }))
}

/// Whatever the run left behind, onto the host's board. Nothing left is
/// nothing to do — the caller hands over what `bridge` answered and is
/// not made to ask again.
///
/// Marked `— linux` because Done is both OSes photographed for the same
/// verb (CLAUDE.md ビルド・テスト): a board holding two pictures that do
/// not say which side each came from cannot show that.
///
/// **The command has to be the one that was typed**, not the one `bridge`
/// handed the container — see [`boarding`].
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
///
/// **Only the line as it was typed can answer**: [`bridge`] says
/// `--no-board` into the line it hands the container regardless, the
/// board being this side's to write. A suite's runs are typed with it
/// (`verify::suite_words`) — their pictures still travel out to a
/// directory a person can open, and nothing is filed.
fn boarding(command: &[String]) -> bool {
    !command.iter().any(|word| word == "--no-board")
}

/// What to call the run on the board, out of the command line it was.
/// Pure so the tests can ask.
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

/// A host directory for what a run means to be looked at afterwards, or
/// None when the command leaves nothing. verify-ui writes its screenshot
/// and the settings it ran with into --shot-dir; inside a container that is
/// a place nobody can open, and the whole verdict is a PNG.
///
/// **One per run, or two containers share a settings store.** `/out` is
/// the same path in every container, so what keeps two of them apart is
/// this directory alone: hand the same one twice and the second app to
/// start finds the first still holding `/out/config`
/// (`settings::Store::claim`), opens the window that says so instead of
/// the one the verb is about, and waits out its watchdog.
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
    crate::verify::claim_dir(&std::env::temp_dir().join("pg-linux"), kind)
}

#[cfg(test)]
mod tests {
    use super::{boarding, bridge, keepsakes, naming};

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(String::from).collect()
    }

    /// The verb names the run when nobody named it, and either way the
    /// board says which side of Done this picture is.
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
            "the chip's badge",
        ]
        .map(String::from);
        assert_eq!(
            naming(&told),
            (
                "the chip's badge — linux".to_string(),
                "row-card".to_string()
            )
        );
    }

    /// The board is answered by the line as typed. `bridge` writes
    /// `--no-board` into every line it sends in, so a caller that asked
    /// the mutated line would file nothing at all.
    #[test]
    fn what_the_container_was_told_cannot_answer_for_the_board() {
        let typed = words("cargo xtask verify-ui commit");
        assert!(boarding(&typed));
        let mut sent = typed.clone();
        let out = bridge(&mut sent, "/out")
            .expect("a bridged run")
            .expect("a directory to bring the pictures back to");
        assert!(!boarding(&sent), "bridge says it whether the caller did");
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

    /// `/out` is the same path in every container, so the directory it is
    /// mounted from is the whole of what keeps two runs of a side apart —
    /// their settings stores, their screenshots and the git configuration
    /// they read an identity from all sit in it. The gate starts a side's
    /// verbs together (`gate::verbs`), which is where a clock that two of
    /// them read inside one tick would have handed them one directory.
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
            // Empty, and so nobody else's: the claim made it rather than
            // finding it, which is what `create_dir_all` could not say.
            std::fs::remove_dir(&dir).expect("an empty directory this call created");
        }
    }
}
