//! The launch guard: a real window and a process nothing ends both take
//! something from the sessions beside this one, and a launch someone reads
//! the output of takes the turn it was started from.

use super::WINDOW_ESCAPE;
use super::git::{unquote, xtask_verb};
use super::payload::{bool_field, string_field};
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
    // Held to before the escape and outside any worktree: the escape records
    // that the user asked for a window, not that the session may stop
    // answering, and a launch takes the turn wherever it is started from.
    if reads_the_launch(&command) {
        println!(
            "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
             \"permissionDecision\":\"deny\",\"permissionDecisionReason\":\
             \"A launch must not feed a pipe or a command substitution. The \
             window it starts inherits the write end of the shell's pipe and \
             holds it open for as long as it lives, so the reader never sees \
             end-of-file: the call does not come back until the window closes, \
             and until then the turn cannot end and whatever the user types \
             next waits in the queue. Measured: piped launches came back after \
             23s to 603s (the tool's own timeout), the same launch without a \
             pipe after 8.7s. `cargo xtask launch` prints three lines, so there \
             is nothing to trim — run it on its own, report, and end the \
             turn.\"}}}}"
        );
        return Ok(());
    }
    // Same reason from the other side: a background task keeps the session
    // busy while it runs and wakes the agent again when it lands, and a
    // launch has nothing to wait for once the window is up.
    if !launch_segments(&command).is_empty() && bool_field(input, "run_in_background") == Some(true)
    {
        println!(
            "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
             \"permissionDecision\":\"deny\",\"permissionDecisionReason\":\
             \"A launch is not background work. It reaps this tree's stale \
             runs, builds, starts the window detached and answers, all in the \
             foreground; a background task instead leaves the session busy \
             until it lands and then wakes the agent again, which is the one \
             thing a launch is not supposed to do. Run it in the foreground \
             (raise the tool's timeout if the release build needs it), report, \
             and end the turn.\"}}}}"
        );
        return Ok(());
    }
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

/// Whether `command` puts a reader on `cargo xtask launch`'s output. The
/// launch is detached, but Windows hands a new process every inheritable
/// handle the starting one holds — the write end of a shell pipe included —
/// so the reader waits for an end-of-file that only the window's death
/// brings. A redirect has no reader and is harmless; a pipe and a
/// substitution are readers. Only the detached launch is held to this:
/// `verify-ui` ends the app it starts, and `cargo run` holds it in the
/// foreground on purpose.
fn reads_the_launch(command: &str) -> bool {
    launch_segments(command)
        .iter()
        .any(|segment| segment.contains('|') || segment.contains("$("))
}

/// The commands in the line that start the detached window.
fn launch_segments(command: &str) -> Vec<&str> {
    shell_segments(command)
        .into_iter()
        .filter(|segment| {
            // A substitution wraps the launch in parentheses, which would
            // otherwise stick to the verb.
            let bare = segment.replace(['(', ')'], " ");
            let tokens: Vec<&str> = bare.split_whitespace().collect();
            xtask_verb(&tokens, "launch")
        })
        .collect()
}

/// `command` cut where one command in the line ends and the next begins, so
/// that a pipe belonging to a neighbour is not read as the launch's own.
fn shell_segments(command: &str) -> Vec<&str> {
    command
        .split(['\n', ';'])
        .flat_map(|part| part.split("&&"))
        .flat_map(|part| part.split("||"))
        .collect()
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
    use super::{launch_objections, reads_the_launch, resolve};

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
    fn refuses_a_launch_whose_output_something_waits_on() {
        for command in [
            "PG_ALLOW_GUI=1 cargo xtask launch 2>&1 | tail -4",
            "PG_ALLOW_GUI=1 cargo xtask launch --no-build | Select-Object -Last 3",
            "pid=$(PG_ALLOW_GUI=1 cargo xtask launch)",
        ] {
            assert!(reads_the_launch(command), "{command}");
        }
    }

    #[test]
    fn lets_through_a_launch_nothing_is_waiting_on() {
        for command in [
            "PG_ALLOW_REBASE=1 git rebase main && PG_ALLOW_GUI=1 cargo xtask launch",
            // Nobody reads a file, and stderr joining stdout adds no reader.
            "PG_ALLOW_GUI=1 cargo xtask launch > launch.log 2>&1",
            // The pipe belongs to the command beside it, not to the launch.
            "git rebase main 2>&1 | tail -2 && PG_ALLOW_GUI=1 cargo xtask launch",
            // These two end their app themselves, so their reader sees an end.
            "cargo xtask verify-ui commit --preset basic | tail -5",
            "cargo run --release -p platitude-app 2>&1 | tail -50",
        ] {
            assert!(!reads_the_launch(command), "{command}");
        }
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
