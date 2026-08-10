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

/// The same, for an instruction that asked for a real window.
const WINDOW_ESCAPE: &str = "PG_ALLOW_GUI";

/// The directory every worktree of this repository sits under.
const WORKTREES: &str = "/.claude/worktrees/";

pub fn run(args: &[String]) -> Result<(), String> {
    let event = args.first().map(String::as_str).unwrap_or("");
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| format!("failed to read hook payload: {e}"))?;
    match event {
        "pre-write" => pre_write(&input),
        "post-write" => post_write(&input),
        "pre-shell" => pre_shell(&input),
        // Worktrees whose branch predates pre-shell still name this event in
        // their own .claude/settings.json, and they keep the git guard until
        // they pick the new wiring up.
        "pre-git" => pre_git(&input).map(|_| ()),
        "session-start" => session_start(&input),
        other => Err(format!("unknown hook event: {other:?}")),
    }
}

/// PreToolUse(Bash|PowerShell): every shell line passes through here. One
/// decision per call — two JSON objects on stdout is not a payload — so the
/// guards run in order and the first refusal is the answer.
fn pre_shell(input: &str) -> Result<(), String> {
    if pre_git(input)? {
        return Ok(());
    }
    pre_launch(input)
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

/// The font rules of デザイン規約 §QML 実装ルール, checked line by line:
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
                 (デザイン規約 §QML 実装ルール).",
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
                 the per-OS fallback chain (デザイン規約 §QML 実装ルール).",
                number + 1
            ));
        }
    }
    notes
}

/// PreToolUse(Bash|PowerShell): the git this repository holds until the
/// user asks for it in so many words (CLAUDE.md Git 運用) — landing a
/// branch on main, and committing the shared session rules from the
/// primary checkout. Answers whether it refused, so the guard after it
/// stays quiet when it did.
fn pre_git(input: &str) -> Result<bool, String> {
    let Some(command) = string_field(input, "command") else {
        return Ok(false);
    };
    if command.contains(MAIN_ESCAPE) {
        return Ok(false);
    }
    let cwd = string_field(input, "cwd").unwrap_or_default();
    Ok(main_landing_denied(&command, &cwd) || shared_rules_denied(&command, &cwd))
}

/// Putting a branch onto main is the user's call. Which worktree branches
/// have landed and which have not is only answerable if every landing was
/// asked for, so a session that merges on its own way out is the thing to
/// stop (CLAUDE.md Git 運用). Prints the refusal and says so.
fn main_landing_denied(command: &str, cwd: &str) -> bool {
    let Some(reflection) = reflection(command) else {
        return false;
    };
    let dir = reflection.dir.unwrap_or(cwd);
    // Any git that cannot answer is git we are not guarding: a throwaway
    // repository (CLAUDE.md Rust 規約: measure git in one) is on main as
    // often as not, and the command would fail here anyway if the path is
    // not a repository at all.
    let (Some(session_repo), Some(target_repo)) = (common_git_dir(cwd), common_git_dir(dir)) else {
        return false;
    };
    if !session_repo.eq_ignore_ascii_case(&target_repo) {
        return false;
    }
    if reflection.only_from_main
        && git_query(dir, &["rev-parse", "--abbrev-ref", "HEAD"]).as_deref() != Some("main")
    {
        return false;
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
    true
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

/// The primary checkout may commit documents directly, except the files
/// every session loads: .claude/skills and .claude/rules ride worktree
/// branches, because parallel sessions keep reaching for the same files
/// and direct commits to main collide (CLAUDE.md Git 運用). A commit is
/// held only when it demonstrably carries them — named on the line,
/// already staged, or swept in by broad staging while they sit changed.
fn shared_rules_denied(command: &str, cwd: &str) -> bool {
    let Some(commit) = commit(command) else {
        return false;
    };
    let dir = commit.dir.unwrap_or(cwd);
    // The same bounds as the merge guard: only this repository answers,
    // and a worktree branch is exactly where these edits belong.
    let (Some(session_repo), Some(target_repo)) = (common_git_dir(cwd), common_git_dir(dir)) else {
        return false;
    };
    if !session_repo.eq_ignore_ascii_case(&target_repo) {
        return false;
    }
    if worktree_root(&resolve(cwd, dir)).is_some() {
        return false;
    }
    let carries =
        commit.named || staged_shared_rules(dir) || (commit.broad && changed_shared_rules(dir));
    if !carries {
        return false;
    }
    println!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
         \"permissionDecision\":\"deny\",\"permissionDecisionReason\":\
         \"This commit would carry .claude/skills or .claude/rules onto main \
         from the primary checkout, and those files ride worktree branches: \
         parallel sessions keep reaching for them, and direct commits to \
         main collide (CLAUDE.md Git 運用). Make the edit on a worktree \
         branch and report the branch as ready to merge. If the user asked \
         for this direct commit in so many words, run the same command again \
         with {}=1 in front of it.\"}}}}",
        MAIN_ESCAPE
    );
    true
}

/// A `git commit` found in a shell line, with what the whole line stages
/// around it.
struct Commit<'a> {
    /// The repository it acts on, read the way Reflection reads it.
    dir: Option<&'a str>,
    /// Whether a pathspec of a staging or commit verb names the shared
    /// rules.
    named: bool,
    /// Whether staging is broad (`-a`, `add -A`, `add .`), sweeping in
    /// whatever sits changed without naming it.
    broad: bool,
}

/// The first `git commit` in `command`, if any, folding in every staging
/// verb on the line: `git add X && git commit` stages X only after this
/// hook has answered, so the line is the only place X shows in time.
fn commit(command: &str) -> Option<Commit<'_>> {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    let mut cd_dir = None;
    let mut dir = None;
    let mut seen = false;
    let mut named = false;
    let mut broad = false;
    let mut index = 0;
    while index < tokens.len() {
        if tokens[index] == "cd" {
            cd_dir = tokens.get(index + 1).map(|d| unquote(d));
            index += 2;
            continue;
        }
        if tokens[index] != "git" {
            index += 1;
            continue;
        }
        let mut git_dir = None;
        index += 1;
        while let Some(option) = tokens.get(index).filter(|token| token.starts_with('-')) {
            if *option == "-C" {
                git_dir = tokens.get(index + 1).map(|d| unquote(d));
            }
            index += if matches!(*option, "-C" | "-c") { 2 } else { 1 };
        }
        let Some(subcommand) = tokens.get(index).copied() else {
            break;
        };
        index += 1;
        if !matches!(subcommand, "commit" | "add" | "stage" | "mv" | "rm") {
            continue;
        }
        while let Some(token) = tokens.get(index).copied() {
            if token == "git" {
                break;
            }
            if matches!(token, "&&" | "||" | ";" | "|") {
                index += 1;
                break;
            }
            let span = quoted_span(&tokens, index);
            if token.starts_with('-') {
                broad |= stages_broadly(subcommand, token);
                index = if span == index + 1 && takes_value(subcommand, token) {
                    quoted_span(&tokens, index + 1)
                } else {
                    span
                };
                continue;
            }
            let path = unquote(token).replace('\\', "/");
            named |= names_shared_rules(&path);
            let path = path.trim_end_matches('/');
            broad |= matches!(path, "." | ":/" | ".claude") || path.ends_with("/.claude");
            index = span;
        }
        if subcommand == "commit" && !seen {
            seen = true;
            dir = git_dir.or(cd_dir);
        }
    }
    seen.then_some(Commit { dir, named, broad })
}

/// Index past the token at `at`, extended to the closing quote when the
/// token opens one it does not close: a quoted value is one word to the
/// shell however many words this whitespace split made of it.
fn quoted_span(tokens: &[&str], at: usize) -> usize {
    let Some(first) = tokens.get(at) else {
        return at;
    };
    let Some(open) = first.find(['"', '\'']) else {
        return at + 1;
    };
    let quote = char::from(first.as_bytes()[open]);
    if first[open + 1..].contains(quote) {
        return at + 1;
    }
    let mut end = at + 1;
    while let Some(token) = tokens.get(end) {
        end += 1;
        if token.contains(quote) {
            break;
        }
    }
    end
}

/// Whether `option` makes `subcommand` stage broadly — sweeping in
/// whatever sits changed without naming it.
fn stages_broadly(subcommand: &str, option: &str) -> bool {
    let staging = matches!(subcommand, "add" | "stage");
    match option {
        "--all" => staging || subcommand == "commit",
        "--update" => staging,
        _ => option.strip_prefix('-').is_some_and(|cluster| {
            !cluster.starts_with('-')
                && ((subcommand == "commit" && cluster.contains('a'))
                    || (staging && cluster.contains(['A', 'u'])))
        }),
    }
}

/// Whether a commit option takes the next token as its value. Only commit
/// is read this closely — its message is where pathspec-looking words
/// live; the staging verbs take no values worth skipping.
fn takes_value(subcommand: &str, option: &str) -> bool {
    subcommand == "commit"
        && (matches!(
            option,
            "--message"
                | "--file"
                | "--author"
                | "--date"
                | "--template"
                | "--cleanup"
                | "--fixup"
                | "--squash"
                | "--trailer"
                | "--reuse-message"
                | "--reedit-message"
                | "--pathspec-from-file"
        ) || option.strip_prefix('-').is_some_and(|cluster| {
            !cluster.starts_with('-') && cluster.ends_with(['m', 'F', 'C', 'c', 't'])
        }))
}

/// Whether `text` — a pathspec or a line of git status output — names
/// .claude/skills or .claude/rules as a path segment. .claude/settings.json
/// stays directly committable; only the files every session loads as rules
/// ride worktree branches.
fn names_shared_rules(text: &str) -> bool {
    let text = text.replace('\\', "/");
    [".claude/skills", ".claude/rules"].iter().any(|shared| {
        text.match_indices(*shared).any(|(at, _)| {
            let before = text[..at].chars().next_back();
            let after = text[at + shared.len()..].chars().next();
            before.is_none_or(|c| matches!(c, '/' | '"' | '\'' | ' '))
                && after.is_none_or(|c| matches!(c, '/' | '"' | '\'' | ' '))
        })
    })
}

/// Whether the index already carries the shared rules — staged by an
/// earlier tool call, with nothing left on this line to name them.
fn staged_shared_rules(dir: &str) -> bool {
    git_query(dir, &["diff", "--cached", "--name-only"])
        .is_some_and(|paths| paths.lines().any(names_shared_rules))
}

/// Whether they sit changed at all — what broad staging would sweep into
/// the commit, untracked files included.
fn changed_shared_rules(dir: &str) -> bool {
    git_query(dir, &["status", "--porcelain"])
        .is_some_and(|status| status.lines().any(names_shared_rules))
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

/// PreToolUse(Bash|PowerShell): a worktree session exists so that it takes
/// nothing from the sessions beside it, and there are two ways starting the
/// app takes something anyway — a real window puts itself over whatever is
/// on the screen (and over the window another session is trying to grab),
/// and a process nothing ends keeps holding the exe it runs, so the build
/// after it cannot link. Both have a form that takes neither: offscreen QPA
/// wants no screen, PG_AUTO_QUIT_MS makes the process let go on its own.
/// Only worktree sessions are held to it — a launch asked for in the primary
/// checkout is the user's own, and two of those may collide (CLAUDE.md
/// ビルド・テスト).
fn pre_launch(input: &str) -> Result<(), String> {
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
         sessions beside it need: {}. Headless takes nothing: \
         `cargo xtask verify-ui <verb>` sets offscreen QPA and a quit timer \
         and kills the run if it hangs (verify-ui skill). Running the binary \
         by hand works too, with QT_QPA_PLATFORM=offscreen and \
         PG_AUTO_QUIT_MS set and this worktree's own target/ as the path. If \
         the user asked for a real window in so many words, run the same \
         command again with {}=1 in front of it.\"}}}}",
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
    if !command.contains("PG_AUTO_QUIT_MS") {
        objections.push(
            "it sets no PG_AUTO_QUIT_MS, so nothing ends the process and it \
             holds the exe against the next build"
                .to_string(),
        );
    }
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

/// The worktree `cwd` sits in: the path down to the directory named under
/// .claude/worktrees/, and None for the primary checkout.
fn worktree_root(cwd: &str) -> Option<String> {
    let cwd = cwd.replace('\\', "/");
    let at = cwd.find(WORKTREES)? + WORKTREES.len();
    if at >= cwd.len() {
        return None;
    }
    let end = cwd[at..].find('/').map_or(cwd.len(), |slash| at + slash);
    Some(cwd[..end].to_string())
}

/// `path` as the shell would reach it from `cwd`, with `.` and `..` folded
/// out so that a way back up into another tree shows in the text.
fn resolve(cwd: &str, path: &str) -> String {
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
             — see CLAUDE.md ビルド・テスト. Document edits and review are fine \
             here, except .claude/skills and .claude/rules: parallel sessions \
             keep reaching for those same files, and a direct commit to main \
             collides with theirs — edit them on a worktree branch and report \
             the branch as ready to merge (CLAUDE.md Git 運用)."
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
    use super::{
        commit, launch_objections, names_shared_rules, qml_font_notes, reflection, resolve,
    };

    const IN_WORKTREE: &str = "C:/Users/x/IdeaProjects/platitude-gg/.claude/worktrees/launch";
    const PRIMARY: &str = "C:/Users/x/IdeaProjects/platitude-gg";

    #[test]
    fn refuses_a_worktree_launch_that_opens_a_window_or_keeps_the_exe() {
        let window = launch_objections("./target/release/platitude-gg.exe", IN_WORKTREE);
        assert_eq!(window.len(), 2, "{window:?}");
        assert!(window[0].contains("QT_QPA_PLATFORM"));
        assert!(window[1].contains("PG_AUTO_QUIT_MS"));

        let offscreen_only = launch_objections(
            "QT_QPA_PLATFORM=offscreen ./target/release/platitude-gg",
            IN_WORKTREE,
        );
        assert_eq!(offscreen_only.len(), 1, "{offscreen_only:?}");
        assert!(offscreen_only[0].contains("PG_AUTO_QUIT_MS"));

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
            "QT_QPA_PLATFORM=offscreen PG_AUTO_QUIT_MS=3000 ./target/release/platitude-gg.exe",
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
            "QT_QPA_PLATFORM=offscreen PG_AUTO_QUIT_MS=3000 \
             ../../../target/release/platitude-gg.exe",
            IN_WORKTREE,
        );
        assert_eq!(elsewhere.len(), 1, "{elsewhere:?}");
        assert!(
            elsewhere[0].contains("outside this worktree"),
            "{elsewhere:?}"
        );

        let named_absolutely = launch_objections(
            "QT_QPA_PLATFORM=offscreen PG_AUTO_QUIT_MS=3000 \
             \"C:\\Users\\x\\IdeaProjects\\platitude-gg\\.claude\\worktrees\\launch\\target\\release\\platitude-gg.exe\"",
            IN_WORKTREE,
        );
        assert!(named_absolutely.is_empty(), "{named_absolutely:?}");
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

    #[test]
    fn holds_a_commit_that_names_the_shared_rules() {
        let staged = commit("git add .claude/rules/core.md && git commit -m \"x\"").unwrap();
        assert!(staged.named);
        let direct = commit("git commit .claude/skills/verify-ui/SKILL.md -m \"x\"").unwrap();
        assert!(direct.named);
        let quoted =
            commit("git add \".claude/skills/a b/SKILL.md\" && git commit -m \"x\"").unwrap();
        assert!(quoted.named);
        let removed = commit("git rm -r .claude/rules && git commit -m \"x\"").unwrap();
        assert!(removed.named);
        assert!(
            commit("git add .claude/rules/core.md").is_none(),
            "a line with no commit stages nothing to hold"
        );
    }

    #[test]
    fn reads_the_message_as_one_value_not_as_pathspecs() {
        let mention = commit("git commit -m \"docs: note .claude/rules/core.md moved\"").unwrap();
        assert!(!mention.named);
        let glued =
            commit("git commit --message=\"see .claude/skills for verbs\" CLAUDE.md").unwrap();
        assert!(!glued.named);
        let heredoc =
            commit("git commit -m \"$(cat <<'EOF'\ndocs: .claude/rules/core.md notes\nEOF\n)\"")
                .unwrap();
        assert!(!heredoc.named);
    }

    #[test]
    fn sees_broad_staging_for_what_it_would_sweep() {
        assert!(commit("git commit -am \"x\"").unwrap().broad);
        assert!(commit("git add -A && git commit -m \"x\"").unwrap().broad);
        assert!(commit("git add . && git commit -m \"x\"").unwrap().broad);
        assert!(
            commit("git add .claude && git commit -m \"x\"")
                .unwrap()
                .broad
        );
        let narrow = commit("git add CLAUDE.md && git commit -m \"x\"").unwrap();
        assert!(!narrow.broad && !narrow.named);
    }

    #[test]
    fn reads_the_directory_the_commit_would_run_in() {
        let by_option = commit("git -C ../.. commit -m \"x\"").unwrap();
        assert_eq!(by_option.dir, Some("../.."));
        let by_cd = commit("cd C:/x/platitude-gg && git commit -am \"x\"").unwrap();
        assert_eq!(by_cd.dir, Some("C:/x/platitude-gg"));
    }

    #[test]
    fn knows_the_shared_directories_by_their_segments() {
        assert!(names_shared_rules(".claude/rules/core.md"));
        assert!(names_shared_rules(
            "C:/x/platitude-gg/.claude/skills/verify-ui/SKILL.md"
        ));
        assert!(names_shared_rules(".claude\\rules\\core.md"));
        assert!(names_shared_rules(".claude/skills"));
        assert!(!names_shared_rules(".claude/settings.json"));
        assert!(!names_shared_rules("docs/.claude/rules-of-thumb.md"));
        assert!(!names_shared_rules("internal-docs/skills.md"));
    }
}
