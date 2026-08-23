//! What a verb needs made for it before the app opens: the repositories
//! its tabs stand on, and the folders it is handed.

use std::path::PathBuf;
use std::process::Command;

/// Names of a spread of lengths, for the one verb whose subject is the
/// tab strip itself.
///
/// Both halves of the rule need seeing at once (デザイン規約 §ウィンドウの縁):
/// the short names keep their own width however crowded the strip gets,
/// and the long ones give way together. A row of repositories all called
/// `repo` can show neither.
const TAB_NAMES: [&str; 16] = [
    "ui",
    "platitude-gg",
    "core",
    "sealed-class-enumizer",
    "notes",
    "a-repository-with-a-rather-long-name",
    "docs",
    "another-repository-with-a-long-name",
    "spike",
    "yet-another-long-repository-name",
    "assets",
    "the-longest-repository-name-in-the-row",
    "ci",
    "one-more-long-repository-name-here",
    "tools",
    "a-final-repository-with-a-long-name",
];

/// The strip the `tab-widths` verb is run against: `count` repositories
/// (default 8) named off the ladder above.
///
/// The last one carries a real history — the tab opened last is the one
/// left in front, so that is the page under the strip in the picture.
/// The rest are bare of commits: what is being looked at is above them,
/// and building sixteen histories to photograph one band would be paying
/// for the wrong thing.
pub(super) fn tab_width_repos(arg: &str) -> Result<Vec<PathBuf>, String> {
    let count: usize = if arg.is_empty() {
        8
    } else {
        arg.parse()
            .map_err(|_| format!("tab-widths takes a number of tabs, not {arg:?}"))?
    };
    if count == 0 || count > TAB_NAMES.len() {
        return Err(format!("tab-widths takes 1..={} tabs", TAB_NAMES.len()));
    }
    let mut made = Vec::with_capacity(count);
    for (position, name) in TAB_NAMES.iter().take(count).enumerate() {
        let preset = if position + 1 == count {
            "basic"
        } else {
            "empty"
        };
        let repo = crate::demo::create_named(preset, None, name)?;
        made.push(repo);
    }
    println!("demo repos (tab-widths): {count} named after their length");
    Ok(made)
}

/// The description a verb needs typed for it, when the run brought none.
///
/// The commit editor starts empty, so the verbs that pull its box open
/// have nothing to open it for unless something is in it — and something
/// longer than the pane is tall, or the pull lands on the end of the text
/// rather than on the bound this is about. The details pane needs no such
/// thing: its messages come out of a repository.
pub(super) fn body_for(verb: &str) -> Option<String> {
    match verb {
        "wip-grow" | "wip-grow-squeeze" => Some(crate::demo::pasted(4000)),
        _ => None,
    }
}

/// The one merge tool the candidate list is guaranteed to hold.
///
/// Named rather than found: `git mergetool --tool-help` is an inventory
/// of the machine, and a container built to run tests has no windowed
/// merge tool on it at all.
const SEEDED_TOOL: &str = "demo-editor";

/// What the seeded tool does once it is launched for real: nothing, out
/// loud, for long enough to be waited on.
///
/// `sleep` is the blocking command all three machines agree on — git runs
/// a tool's `cmd` through its own shell, which on Windows is Git for
/// Windows' `sh` and its `/usr/bin/sleep` (2026-08-23 実測).
///
/// Two seconds, not the thirty the recipe this replaced typed by hand.
/// The verb is a write act, so its one picture is taken behind
/// `AutoActDriver.writeBarrier` and the `treeBarrier` after it — after
/// the tool has exited and the file it resolved has left the conflicted
/// bucket — and no length buys a frame of the wait. What is left for the
/// number to be is a wait a person watching a windowed run can see,
/// against time every `cargo xtask check` pays on both sides.
const TOOL_CMD: &str = "sleep 2";

/// Puts a tool in the repository's own config, for the verbs whose
/// picture is the list of them and the one that hands a file over.
///
/// The candidates arrive in two waves — names written in config, then
/// the installed sweep — and only the first is the run's to decide. Where
/// the second answers with nothing the list is empty, and an empty list
/// is an answer: the card closes itself (`AppCombo.hasList`), leaving the
/// verb waiting on a popup that will not open again. So the run brings
/// its own row, and the first wave carries the picture on every machine.
///
/// For the settings verbs that is the whole of it: the command is never
/// run, and no tool is named as the one to launch — `merge.guitool` would
/// land in the dialog's own field, which is the thing they photograph.
///
/// `open-mergetool` needs the name launchable rather than merely listed,
/// so it gets that key and one more. Without `merge.guitool` there is
/// nothing for `conflict::configured_tool` to answer with, the menu row
/// becomes the door to the settings instead, and the verb queues no
/// write. Without `trustExitCode` git asks a closed stdin whether the
/// merge went well, reads EOF, and calls the file failed (実測).
///
/// Whatever repositories the run is about are written to, a `--repo` of
/// one's own included: the run owns them for its length
/// (`claim_resource`), and a merge tool named in one is overwritten.
pub(super) fn seed_merge_tool(
    verb: &str,
    repos: &[PathBuf],
    path: &std::ffi::OsStr,
) -> Result<(), String> {
    let launched = match verb {
        "settings-tools" | "settings-tools-loading" => false,
        "open-mergetool" => true,
        _ => return Ok(()),
    };
    let mut keys = vec![(format!("mergetool.{SEEDED_TOOL}.cmd"), TOOL_CMD.to_string())];
    if launched {
        keys.push((
            format!("mergetool.{SEEDED_TOOL}.trustExitCode"),
            "true".to_string(),
        ));
        keys.push(("merge.guitool".to_string(), SEEDED_TOOL.to_string()));
    }
    for repo in repos {
        for (key, value) in &keys {
            let status = Command::new("git")
                .arg("-C")
                .arg(repo)
                .args(["config", key, value])
                .env("PATH", path)
                .status()
                .map_err(|e| format!("failed to run git: {e}"))?;
            if !status.success() {
                return Err(format!("could not name a merge tool in {}", repo.display()));
            }
        }
    }
    if launched {
        println!("merge editor seeded: {SEEDED_TOOL} runs {TOOL_CMD:?}, and git would launch it");
    } else {
        println!("merge editor named in config: {SEEDED_TOOL} (the list's first wave)");
    }
    Ok(())
}

/// The folder a verb needs handed to it, made on the spot.
///
/// The open-refused verbs are the only ones whose subject is a folder no
/// demo repository can be — being one is the whole point — so there is
/// nothing to name with `--preset`. Both sit one level down, so the
/// folder the second try opens at is a real one with the failed pick
/// inside it. Empty for every other verb: nothing is invented for a verb
/// that was simply run without its argument.
pub(super) fn folder_for(
    verb: &str,
    shot_dir: &std::path::Path,
    path: &std::ffi::OsStr,
) -> Result<String, String> {
    let made = shot_dir.join("picked");
    match verb {
        "open-not-a-repo"
        | "open-not-a-repo-retry"
        | "open-not-a-repo-cancel"
        | "open-fail-tab"
        | "open-fail-tab-log" => {
            let dir = made.join("notes");
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            Ok(dir.display().to_string())
        }
        "open-bare" | "open-fail-tab-bare" => {
            let dir = made.join("origin.git");
            std::fs::create_dir_all(&made).map_err(|e| e.to_string())?;
            let status = Command::new("git")
                .args(["init", "--bare", "--quiet"])
                .arg(&dir)
                .env("PATH", path)
                .status()
                .map_err(|e| format!("failed to run git: {e}"))?;
            if !status.success() {
                return Err("git init --bare failed".into());
            }
            Ok(dir.display().to_string())
        }
        _ => Ok(String::new()),
    }
}
