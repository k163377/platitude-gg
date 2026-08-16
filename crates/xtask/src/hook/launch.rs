//! The launch guard: a real window and a process nothing ends both take
//! something from the sessions beside this one.

use super::WINDOW_ESCAPE;
use super::git::{unquote, xtask_verb};
use super::payload::string_field;
use crate::seats::worktree_root;

/// PreToolUse(Bash|PowerShell): starting the app from a worktree takes
/// something from the sessions beside it — a real window covers whatever
/// is on the screen, and an unsupervised process can hold the exe against
/// the next build's link. Offscreen QPA alone is not a parent kill guard.
/// Only worktree sessions are held to it — a launch in the primary checkout
/// is the user's own (CLAUDE.md ビルド・テスト).
pub(super) fn pre_launch(input: &str) -> Result<(), String> {
    let Some(command) = string_field(input, "command") else {
        return Ok(());
    };
    if command.contains(WINDOW_ESCAPE) {
        return Ok(());
    }
    let cwd = string_field(input, "cwd").unwrap_or_default();
    let objections = launch_objections(&command, &cwd);
    if objections.is_empty() {
        return Ok(());
    }
    println!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
         \"permissionDecision\":\"deny\",\"permissionDecisionReason\":\
         \"Starting the app this way from a worktree takes something the \
         sessions beside it need: {}. Headless verification takes nothing: \
         use `cargo xtask verify-ui <verb>` or `cargo xtask linux verify-ui \
         <verb>`; those commands provide the automation and parent kill guard. \
         Offscreen QPA alone does not supervise a raw app process. If the \
         user asked for a real window in so many words, run the same command \
         again with {}=1 in front of it.\"}}}}",
        objections.join("; "),
        WINDOW_ESCAPE
    );
    Ok(())
}

/// What `command`, run from `cwd`, would take from the sessions beside it.
/// Empty when it starts nothing, when it starts it the harmless way, or when
/// the session is not in a worktree at all.
fn launch_objections(command: &str, cwd: &str) -> Vec<String> {
    let Some(root) = worktree_root(cwd) else {
        return Vec::new();
    };
    // `cargo xtask launch` reaps its own tree and builds for itself; the
    // one thing left to object to is the window, which is its purpose.
    if xtask_verb(&command.split_whitespace().collect::<Vec<_>>(), "launch") {
        return vec![
            "it opens a real window over whatever is on the screen, which is \
             what `xtask launch` is for"
                .to_string(),
        ];
    }
    let Some(launch) = launch(command) else {
        return Vec::new();
    };
    let mut objections = Vec::new();
    if !(command.contains("QT_QPA_PLATFORM") && command.contains("offscreen")) {
        objections.push(
            "it sets no QT_QPA_PLATFORM=offscreen, so it opens a real window \
             over whatever is on the screen"
                .to_string(),
        );
    }
    objections.push(
        "it launches the app without a parent watchdog; offscreen QPA alone \
         does not bound the process lifetime — use `cargo xtask verify-ui \
         <verb>` or `cargo xtask linux verify-ui <verb>`"
            .to_string(),
    );
    if let Some(exe) = launch.exe {
        let resolved = resolve(cwd, exe);
        if !resolved.to_lowercase().starts_with(&root.to_lowercase()) {
            objections.push(format!(
                "the binary is {resolved}, outside this worktree — the tree \
                 that owns it has to link that file"
            ));
        }
    }
    objections
}

/// A start of the app found in a shell line.
struct Launch<'a> {
    /// The binary as the command names it, when it names a path at all.
    /// `cargo run` leaves the path to cargo, which builds in this tree.
    exe: Option<&'a str>,
}

/// The first start of the app in `command`, if any. `cargo xtask verify-ui`
/// is not one and needs no exception: it names no binary and is not
/// `cargo run`, so nothing here sees it.
fn launch(command: &str) -> Option<Launch<'_>> {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    let mut index = 0;
    while index < tokens.len() {
        let token = unquote(tokens[index]);
        index += 1;
        if names_the_binary(token) {
            return Some(Launch { exe: Some(token) });
        }
        if token != "cargo" || tokens.get(index).map(|t| unquote(t)) != Some("run") {
            continue;
        }
        let arguments: Vec<&str> = tokens[index + 1..]
            .iter()
            .take_while(|token| !matches!(**token, "&&" | "||" | ";" | "|" | "--"))
            .map(|token| unquote(token))
            .collect();
        // A named package that is not the app is someone else's binary — the
        // task runner's, usually. Without one, the workspace's default
        // members leave exactly one runnable target, which is the app.
        let package = arguments
            .iter()
            .position(|argument| matches!(*argument, "-p" | "--package"))
            .and_then(|at| arguments.get(at + 1));
        if package.is_none_or(|package| *package == "platitude-app") {
            return Some(Launch { exe: None });
        }
    }
    None
}

/// Whether `token` names the app's binary. The repository directory is
/// called platitude-gg as well, so a path that only ends in the bare name
/// has to be a built one before it counts.
fn names_the_binary(token: &str) -> bool {
    let path = token.replace('\\', "/");
    let Some(name) = path.rsplit('/').next() else {
        return false;
    };
    name.eq_ignore_ascii_case("platitude-gg.exe")
        || (name == "platitude-gg" && path.split('/').any(|segment| segment == "target"))
}

/// `path` as the shell would reach it from `cwd`, with `.` and `..` folded
/// out so that a way back up into another tree shows in the text.
pub(super) fn resolve(cwd: &str, path: &str) -> String {
    let path = path.replace('\\', "/");
    let rooted = path.starts_with('/');
    // A drive letter is the other way a Windows path says it starts at a root.
    let absolute = rooted || path.as_bytes().get(1) == Some(&b':');
    let joined = if absolute {
        path
    } else {
        format!("{}/{path}", cwd.replace('\\', "/"))
    };
    let mut segments: Vec<&str> = Vec::new();
    for segment in joined.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            other => segments.push(other),
        }
    }
    let mut resolved = segments.join("/");
    if joined.starts_with('/') {
        resolved.insert(0, '/');
    }
    resolved
}

#[cfg(test)]
mod tests {
    use super::{launch_objections, resolve};

    const IN_WORKTREE: &str = "C:/Users/x/IdeaProjects/platitude-gg/.claude/worktrees/launch";
    const PRIMARY: &str = "C:/Users/x/IdeaProjects/platitude-gg";

    #[test]
    fn refuses_a_worktree_launch_that_opens_a_window_or_keeps_the_exe() {
        let window = launch_objections("./target/release/platitude-gg.exe", IN_WORKTREE);
        assert_eq!(window.len(), 2, "{window:?}");
        assert!(window[0].contains("QT_QPA_PLATFORM"));
        assert!(window[1].contains("parent watchdog"));

        let offscreen_only = launch_objections(
            "QT_QPA_PLATFORM=offscreen ./target/release/platitude-gg",
            IN_WORKTREE,
        );
        assert_eq!(offscreen_only.len(), 1, "{offscreen_only:?}");
        assert!(offscreen_only[0].contains("parent watchdog"));

        let by_cargo = launch_objections("cargo run --release", IN_WORKTREE);
        assert_eq!(by_cargo.len(), 2, "{by_cargo:?}");
    }

    #[test]
    fn lets_the_headless_shapes_through() {
        for command in [
            "cargo xtask verify-ui commit --preset basic",
            "cargo xtask demo-repo basic",
            // A container start is headless by construction: it has no
            // display to take and it holds this tree's exe not at all.
            "cargo xtask linux test -p platitude-core",
            "cargo xtask linux verify-ui commit --preset basic",
            "cargo build --release",
            "cargo run --quiet -p xtask -- hook pre-write",
            "cd C:/Users/x/IdeaProjects/platitude-gg && git status",
        ] {
            assert!(
                launch_objections(command, IN_WORKTREE).is_empty(),
                "{command}"
            );
        }
    }

    #[test]
    fn refuses_the_binary_of_another_tree_and_leaves_this_one_alone() {
        let elsewhere = launch_objections(
            "QT_QPA_PLATFORM=offscreen \
             ../../../target/release/platitude-gg.exe",
            IN_WORKTREE,
        );
        assert_eq!(elsewhere.len(), 2, "{elsewhere:?}");
        assert!(
            elsewhere
                .iter()
                .any(|reason| reason.contains("parent watchdog"))
        );
        assert!(
            elsewhere
                .iter()
                .any(|reason| reason.contains("outside this worktree")),
            "{elsewhere:?}"
        );

        let named_absolutely = launch_objections(
            "QT_QPA_PLATFORM=offscreen \
             \"C:\\Users\\x\\IdeaProjects\\platitude-gg\\.claude\\worktrees\\launch\\target\\release\\platitude-gg.exe\"",
            IN_WORKTREE,
        );
        assert_eq!(named_absolutely.len(), 1, "{named_absolutely:?}");
        assert!(named_absolutely[0].contains("parent watchdog"));
    }

    #[test]
    fn holds_only_worktree_sessions_to_it() {
        assert!(
            launch_objections("./target/release/platitude-gg.exe", PRIMARY).is_empty(),
            "a launch asked for in the primary checkout is the user's own"
        );
    }

    #[test]
    fn folds_a_way_back_up_out_of_the_path() {
        assert_eq!(resolve("C:/a/b/c", "../../x/app.exe"), "C:/a/x/app.exe");
        assert_eq!(
            resolve("C:/a/b", "./target/app.exe"),
            "C:/a/b/target/app.exe"
        );
        assert_eq!(resolve("/home/x/w", "../t/app"), "/home/x/t/app");
        assert_eq!(resolve("/home/x/w", "/opt/app"), "/opt/app");
    }

    #[test]
    fn objects_to_an_xtask_launch_only_for_its_window() {
        let objections = launch_objections("cargo xtask launch", IN_WORKTREE);
        assert_eq!(objections.len(), 1, "{objections:?}");
        assert!(objections[0].contains("real window"));
        assert!(
            launch_objections("cargo xtask launch", PRIMARY).is_empty(),
            "a primary-checkout launch is the user's own"
        );
    }
}
