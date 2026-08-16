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
