//! Claude Code hook handlers (`cargo xtask hook <event>`).
//!
//! Wired from .claude/settings.json. Each handler reads the hook's JSON
//! payload from stdin and answers on stdout; printing nothing means "no
//! objection". These exist to make the rules that keep being forgotten
//! mechanical instead of attentional (CLAUDE.md 規約の置き場所).

use std::io::Read;

/// What a command carries to say an explicit instruction asked for main to
/// move. It rides in the command itself so the transcript records the ask.
const MAIN_ESCAPE: &str = "PG_ALLOW_MAIN";

pub fn run(args: &[String]) -> Result<(), String> {
    let event = args.first().map(String::as_str).unwrap_or("");
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| format!("failed to read hook payload: {e}"))?;
    match event {
        "pre-write" => pre_write(&input),
        "post-write" => post_write(&input),
        "pre-git" => pre_git(&input),
        "session-start" => session_start(&input),
        other => Err(format!("unknown hook event: {other:?}")),
    }
}

/// PreToolUse(Write): a new .rs directly under crates/platitude-core/tests/
/// would become a second, serialized test binary — integration tests are one
/// binary by rule (tests/it/). Deny with the rule spelled out.
fn pre_write(input: &str) -> Result<(), String> {
    let Some(path) = string_field(input, "file_path") else {
        return Ok(());
    };
    let path = path.replace('\\', "/");
    let Some(rest) = path.split("crates/platitude-core/tests/").nth(1) else {
        return Ok(());
    };
    if rest.ends_with(".rs") && !rest.contains('/') {
        println!(
            "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
             \"permissionDecision\":\"deny\",\"permissionDecisionReason\":\
             \"Integration tests are one binary: cargo test runs test binaries \
             one after another, so a file directly under tests/ becomes a second, \
             serialized binary and a second link. Add the test as a module under \
             crates/platitude-core/tests/it/ and register it in tests/it/main.rs \
             (CLAUDE.md ビルド・テスト).\"}}}}"
        );
    }
    Ok(())
}

/// PostToolUse(Write|Edit): rules a QML change keeps missing by
/// attention. A file absent from qmldir or main.rs silently fails to
/// resolve at runtime (qmldir directories only expose enumerated
/// types), and the font rules below dodge review because the wrong
/// form still renders fine on the machine it was written on.
fn post_write(input: &str) -> Result<(), String> {
    let Some(path) = string_field(input, "file_path") else {
        return Ok(());
    };
    let path = path.replace('\\', "/");
    if !path.ends_with(".qml") || !path.contains("crates/platitude-app/src/ui/") {
        return Ok(());
    }
    let Some(file_name) = path.rsplit('/').next().map(str::to_string) else {
        return Ok(());
    };
    let ui_dir = std::path::Path::new(&path)
        .parent()
        .ok_or("qml path has no parent")?;
    let mut notes: Vec<String> = Vec::new();
    let mut missing: Vec<&str> = Vec::new();
    // Missing registries are someone else's layout problem, not this hook's:
    // only judge the files that are actually there.
    if let Ok(qmldir) = std::fs::read_to_string(ui_dir.join("qmldir"))
        && !qmldir.contains(&file_name)
    {
        missing.push("src/ui/qmldir");
    }
    if let Some(src_dir) = ui_dir.parent()
        && let Ok(main_rs) = std::fs::read_to_string(src_dir.join("main.rs"))
        && !main_rs.contains(&file_name)
    {
        missing.push("main.rs (include_bytes_qml!)");
    }
    if !missing.is_empty() {
        notes.push(format!(
            "{file_name} is not registered in: {}. A QML component in a \
             qmldir directory is invisible unless enumerated there, and \
             unbundled unless embedded in main.rs (.claude/rules/app-ui.md).",
            missing.join(" and ")
        ));
    }
    if let Ok(content) = std::fs::read_to_string(&path) {
        notes.extend(qml_font_notes(&content));
    }
    if !notes.is_empty() {
        println!(
            "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PostToolUse\",\
             \"additionalContext\":\"{}\"}}}}",
            notes.join(" ")
        );
    }
    Ok(())
}

/// The font rules of デザイン規約 §QML実装ルール, checked line by line:
/// pointSize drifts with each OS's logical DPI, and a family named
/// outside Theme skips the per-OS fallback chain Theme resolves — both
/// look right on the machine they were written on and break on another.
fn qml_font_notes(content: &str) -> Vec<String> {
    let mut notes = Vec::new();
    for (number, line) in content.lines().enumerate() {
        let code = line.trim_start();
        if code.starts_with("//") {
            continue;
        }
        if code.contains("font.pointSize") {
            notes.push(format!(
                "line {}: font.pointSize drifts with each OS's logical DPI; \
                 use font.pixelSize with a Theme token \
                 (デザイン規約 §QML実装ルール).",
                number + 1
            ));
        }
        let Some(value) = code.split("font.family").nth(1) else {
            continue;
        };
        let Some(value) = value.trim_start().strip_prefix(':') else {
            continue;
        };
        if value.contains('"') || !value.contains("Theme.") {
            notes.push(format!(
                "line {}: font.family may only take a family Theme resolved \
                 (Theme.uiFamily / Theme.monoFamily) — anything else skips \
                 the per-OS fallback chain (デザイン規約 §QML実装ルール).",
                number + 1
            ));
        }
    }
    notes
}

/// PreToolUse(Bash|PowerShell): putting a branch onto main is the user's
/// call. Which worktree branches have landed and which have not is only
/// answerable if every landing was asked for, so a session that merges on
/// its own way out is the thing to stop (CLAUDE.md Git 運用).
fn pre_git(input: &str) -> Result<(), String> {
    let Some(command) = string_field(input, "command") else {
        return Ok(());
    };
    if command.contains(MAIN_ESCAPE) {
        return Ok(());
    }
    let Some(reflection) = reflection(&command) else {
        return Ok(());
    };
    let cwd = string_field(input, "cwd").unwrap_or_default();
    let dir = reflection.dir.unwrap_or(&cwd);
    // Any git that cannot answer is git we are not guarding: a throwaway
    // repository (CLAUDE.md Rust 規約: measure git in one) is on main as
    // often as not, and the command would fail here anyway if the path is
    // not a repository at all.
    let (Some(session_repo), Some(target_repo)) = (common_git_dir(&cwd), common_git_dir(dir))
    else {
        return Ok(());
    };
    if !session_repo.eq_ignore_ascii_case(&target_repo) {
        return Ok(());
    }
    if reflection.only_from_main
        && git_query(dir, &["rev-parse", "--abbrev-ref", "HEAD"]).as_deref() != Some("main")
    {
        return Ok(());
    }
    println!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
         \"permissionDecision\":\"deny\",\"permissionDecisionReason\":\
         \"{} would put commits on main, and main moves only when the user \
         asks for it in so many words (CLAUDE.md Git 運用). Leave the work on \
         its branch and report it as ready to merge instead. If the user did \
         ask for this one, run the same command again with {}=1 in front of \
         it.\"}}}}",
        reflection.what, MAIN_ESCAPE
    );
    Ok(())
}

/// A git command in a shell line that would land commits on main.
struct Reflection<'a> {
    /// The repository it acts on: `git -C <dir>`, else a `cd` that preceded
    /// it, else wherever the session sits.
    dir: Option<&'a str>,
    /// Whether it only reaches main when main is the checked-out branch.
    /// A refspec or a forced update names main from anywhere.
    only_from_main: bool,
    what: &'static str,
}

/// The first main-landing git invocation in `command`, if any. Read-only git
/// and git that names main as a source (`git log main`, `git switch main`)
/// are not it — only the verbs that write refs/heads/main.
fn reflection(command: &str) -> Option<Reflection<'_>> {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    let mut cd_dir = None;
    let mut index = 0;
    while index < tokens.len() {
        if tokens[index] == "cd" {
            cd_dir = tokens.get(index + 1).map(|dir| unquote(dir));
            index += 2;
            continue;
        }
        if tokens[index] != "git" {
            index += 1;
            continue;
        }
        // git's own options come before the subcommand; -C and -c take a
        // separate value, so stepping one token at a time would read that
        // value as the subcommand.
        let mut dir = None;
        index += 1;
        while let Some(option) = tokens.get(index).filter(|token| token.starts_with('-')) {
            if *option == "-C" {
                dir = tokens.get(index + 1).map(|dir| unquote(dir));
            }
            index += if matches!(*option, "-C" | "-c") { 2 } else { 1 };
        }
        let Some(subcommand) = tokens.get(index) else {
            break;
        };
        let arguments: Vec<&str> = tokens[index + 1..]
            .iter()
            .take_while(|token| !matches!(**token, "&&" | "||" | ";" | "|" | "git"))
            .copied()
            .collect();
        let landing = match *subcommand {
            // --abort and --quit walk a merge back; they never move the branch on.
            "merge" if !arguments.iter().any(|a| matches!(*a, "--abort" | "--quit")) => {
                Some(("`git merge`", true))
            }
            "push" | "fetch" | "pull" if arguments.iter().any(|a| writes_main(a)) => {
                Some(("A refspec writing main", false))
            }
            "branch"
                if arguments
                    .iter()
                    .any(|a| matches!(*a, "-f" | "--force" | "-M"))
                    && arguments.iter().any(|a| is_main_ref(a)) =>
            {
                Some(("Forcing the main branch", false))
            }
            "update-ref" if arguments.iter().any(|a| is_main_ref(a)) => {
                Some(("Updating refs/heads/main", false))
            }
            _ => None,
        };
        if let Some((what, only_from_main)) = landing {
            return Some(Reflection {
                dir: dir.or(cd_dir),
                only_from_main,
                what,
            });
        }
    }
    None
}

/// Whether a refspec's destination — the half after the colon — is main.
/// The colon is what separates a write from a read: `git push origin main`
/// sends main somewhere, `git push . x:main` rewrites it here.
fn writes_main(token: &str) -> bool {
    token
        .split_once(':')
        .is_some_and(|(_, destination)| is_main_ref(destination))
}

fn is_main_ref(token: &str) -> bool {
    matches!(token, "main" | "heads/main" | "refs/heads/main")
}

fn unquote(token: &str) -> &str {
    token.trim_matches(['"', '\''])
}

/// The repository `dir` belongs to, shared by all of its worktrees, so that
/// a merge run from a worktree is recognised as the same repository as the
/// session that runs it.
fn common_git_dir(dir: &str) -> Option<String> {
    git_query(
        dir,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
}

fn git_query(dir: &str, arguments: &[&str]) -> Option<String> {
    if dir.is_empty() {
        return None;
    }
    let mut command = std::process::Command::new("git");
    command.arg("-C").arg(dir).args(arguments);
    let output = crate::run_captured(&mut command).ok()?;
    output.status.success().then(|| {
        String::from_utf8_lossy(&output.stdout)
            .trim()
            .replace('\\', "/")
    })
}

/// SessionStart: sessions opened in the primary checkout get the worktree
/// rule injected while worktree sessions stay quiet. Plain stdout becomes
/// session context for this event.
fn session_start(input: &str) -> Result<(), String> {
    let cwd = string_field(input, "cwd").unwrap_or_default();
    if !cwd.replace('\\', "/").contains("/.claude/worktrees/") {
        println!(
            "This session runs in the primary checkout. Implementation work \
             belongs in a reused fixed-name worktree (`claude --worktree <name>`) \
             so parallel sessions do not fight over target/ and the release exe \
             — see CLAUDE.md ビルド・テスト. Document edits and review are fine here."
        );
    }
    Ok(())
}

/// Returns the first JSON string value for `key` in `input`, unescaped just
/// enough for paths (\\ \" \/). The payload is machine-produced JSON, so the
/// first occurrence of a key like "file_path" is the tool input's.
fn string_field(input: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let after_key = &input[input.find(&needle)? + needle.len()..];
    let after_colon = after_key.trim_start().strip_prefix(':')?.trim_start();
    let body = after_colon.strip_prefix('"')?;
    let mut value = String::new();
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(value),
            '\\' => match chars.next()? {
                'n' => value.push('\n'),
                't' => value.push('\t'),
                other => value.push(other),
            },
            other => value.push(other),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{qml_font_notes, reflection};

    #[test]
    fn flags_every_verb_that_writes_main() {
        for command in [
            "git merge --ff-only worktree-labels",
            "git push . worktree-labels:main",
            "git fetch . worktree-labels:refs/heads/main",
            "git branch -f main worktree-labels",
            "git update-ref refs/heads/main worktree-labels",
        ] {
            assert!(reflection(command).is_some(), "{command}");
        }
    }

    #[test]
    fn leaves_git_that_only_reads_main_alone() {
        for command in [
            "git merge --abort",
            "git merge-base main HEAD",
            "git log main",
            "git switch main",
            "git show HEAD:main",
            "git push origin worktree-labels",
            "git branch main-ish",
        ] {
            assert!(reflection(command).is_none(), "{command}");
        }
    }

    #[test]
    fn reads_the_directory_the_merge_would_run_in() {
        let from_option = reflection("git -c core.pager=cat -C ../.. merge worktree-labels");
        assert_eq!(from_option.map(|r| r.dir), Some(Some("../..")));
        let from_cd = reflection("cd \"C:/IdeaProjects/platitude-gg\" && git merge --ff-only x");
        assert_eq!(
            from_cd.map(|r| r.dir),
            Some(Some("C:/IdeaProjects/platitude-gg"))
        );
        let refspec = reflection("git push . HEAD:main");
        assert_eq!(refspec.map(|r| r.only_from_main), Some(false));
    }

    #[test]
    fn flags_point_size_and_families_named_outside_theme() {
        let notes =
            qml_font_notes("Text {\n    font.pointSize: 12\n    font.family: \"Segoe UI\"\n}\n");
        assert_eq!(notes.len(), 2, "{notes:?}");
        assert!(notes[0].contains("line 2"));
        assert!(notes[1].contains("line 3"));
    }

    #[test]
    fn accepts_theme_resolved_families_and_comments() {
        let notes = qml_font_notes(
            "// font.pointSize in a comment is fine\n\
             Text { font.family: Theme.monoFamily }\n\
             Text { font.family: code ? Theme.monoFamily : Theme.uiFamily }\n",
        );
        assert!(notes.is_empty(), "{notes:?}");
    }
}
