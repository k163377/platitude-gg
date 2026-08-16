//! Claude Code hook handlers (`cargo xtask hook <event>`).
//!
//! Wired from .claude/settings.json. Each handler reads the hook's JSON
//! payload from stdin and answers on stdout; printing nothing means "no
//! objection".

use std::io::Read;

use crate::git_query;
use crate::seats::{self, SEATS, commits_in, worktree_root};

/// What a command carries to say an explicit instruction asked for main to
/// move. It rides in the command itself so the transcript records the ask.
const MAIN_ESCAPE: &str = "PG_ALLOW_MAIN";

/// The same, for an instruction that asked for a rebase.
const REBASE_ESCAPE: &str = "PG_ALLOW_REBASE";

/// The same, for an instruction that asked for a real window.
const WINDOW_ESCAPE: &str = "PG_ALLOW_GUI";

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
        "pre-worktree" => pre_worktree(&input),
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
/// binary by rule (tests/it/).
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

/// The font rules of デザイン規約 §QML 実装ルール, checked line by line.
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
/// branch on main, rewriting a branch under the session, and committing
/// the shared session rules from the primary checkout. Answers whether it
/// refused, so the guard after it stays quiet when it did.
fn pre_git(input: &str) -> Result<bool, String> {
    let Some(command) = string_field(input, "command") else {
        return Ok(false);
    };
    let cwd = string_field(input, "cwd").unwrap_or_default();
    // Each rule keeps its own escape, so asking for one is not asking for
    // the others: a merge the user called for still may not rebase.
    Ok(guarded_git_denied(&command, &cwd)
        || (!command.contains(MAIN_ESCAPE) && shared_rules_denied(&command, &cwd)))
}

/// Putting a branch onto main and rewriting the branch under the session
/// are both the user's call (CLAUDE.md Git 運用). Prints the refusal and
/// says so.
fn guarded_git_denied(command: &str, cwd: &str) -> bool {
    let Some(reflection) = reflection(command) else {
        return false;
    };
    if command.contains(reflection.offence.escape()) {
        return false;
    }
    let dir = reflection.dir.unwrap_or(cwd);
    // Any git that cannot answer is git we are not guarding: a throwaway
    // repository (CLAUDE.md Rust 規約: measure git in one) is on main as
    // often as not and rebases freely, and the command would fail here
    // anyway if the path is not a repository at all.
    let (Some(session_repo), Some(target_repo)) = (common_git_dir(cwd), common_git_dir(dir)) else {
        return false;
    };
    if !session_repo.eq_ignore_ascii_case(&target_repo) {
        return false;
    }
    if reflection.offence.only_from_main()
        && git_query(dir, &["rev-parse", "--abbrev-ref", "HEAD"]).as_deref() != Some("main")
    {
        return false;
    }
    println!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
         \"permissionDecision\":\"deny\",\"permissionDecisionReason\":\"{}\"}}}}",
        reflection.offence.reason(reflection.what)
    );
    true
}

/// A git command in a shell line that a rule holds back.
struct Reflection<'a> {
    /// The repository it acts on: `git -C <dir>`, else a `cd` that preceded
    /// it, else wherever the session sits.
    dir: Option<&'a str>,
    offence: Offence,
    what: &'static str,
}

/// Which rule the command runs into.
#[derive(Debug, PartialEq)]
enum Offence {
    /// Writes refs/heads/main. `only_from_main` is whether it reaches main
    /// only while main is the checked-out branch — a refspec or a forced
    /// update names main from anywhere.
    LandsOnMain { only_from_main: bool },
    /// Rewrites the branch it runs on, whichever branch that is.
    Rebase,
}

impl Offence {
    /// What a command carries to say this rule's exception was asked for.
    fn escape(&self) -> &'static str {
        match self {
            Offence::LandsOnMain { .. } => MAIN_ESCAPE,
            Offence::Rebase => REBASE_ESCAPE,
        }
    }

    fn only_from_main(&self) -> bool {
        matches!(
            self,
            Offence::LandsOnMain {
                only_from_main: true
            }
        )
    }

    fn reason(&self, what: &str) -> String {
        match self {
            Offence::LandsOnMain { .. } => format!(
                "{what} would put commits on main, and main moves only when the \
                 user asks for it in so many words (CLAUDE.md Git 運用). Leave \
                 the work on its branch and report it as ready to merge instead. \
                 If the user did ask for this one, run the same command again \
                 with {MAIN_ESCAPE}=1 in front of it."
            ),
            Offence::Rebase => format!(
                "{what} rewrites the branch under the session, and a rebase runs \
                 only when the user asks for it in so many words (CLAUDE.md Git \
                 運用). A branch behind main is a seat's normal resting state, \
                 not something to fix — leave it and report what is on the \
                 branch. A merged seat starts over with `git reset --hard main`, \
                 which is not a rebase and needs nothing. If the user did ask \
                 for this one, run the same command again with \
                 {REBASE_ESCAPE}=1 in front of it."
            ),
        }
    }
}

/// The first guarded git invocation in `command`, if any. Read-only git and
/// git that names main as a source (`git log main`, `git switch main`) are
/// not it — the verbs that write refs/heads/main, and rebase, which
/// rewrites whichever branch it runs on.
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
        let lands = |only_from_main| Offence::LandsOnMain { only_from_main };
        let guarded = match *subcommand {
            // --abort and --quit walk a merge back; they never move the branch on.
            "merge" if !arguments.iter().any(|a| matches!(*a, "--abort" | "--quit")) => {
                Some(("`git merge`", lands(true)))
            }
            // The same two exits walk a rebase back. Every other form moves
            // the rewrite on, --continue and --skip included: a rebase that
            // stopped is one nobody asked to start.
            "rebase" if !arguments.iter().any(|a| matches!(*a, "--abort" | "--quit")) => {
                Some(("`git rebase`", Offence::Rebase))
            }
            "push" | "fetch" | "pull" if arguments.iter().any(|a| writes_main(a)) => {
                Some(("A refspec writing main", lands(false)))
            }
            "pull" if arguments.iter().any(|a| matches!(*a, "--rebase" | "-r")) => {
                Some(("`git pull --rebase`", Offence::Rebase))
            }
            "branch"
                if arguments
                    .iter()
                    .any(|a| matches!(*a, "-f" | "--force" | "-M"))
                    && arguments.iter().any(|a| is_main_ref(a)) =>
            {
                Some(("Forcing the main branch", lands(false)))
            }
            "update-ref" if arguments.iter().any(|a| is_main_ref(a)) => {
                Some(("Updating refs/heads/main", lands(false)))
            }
            _ => None,
        };
        if let Some((what, offence)) = guarded {
            return Some(Reflection {
                dir: dir.or(cd_dir),
                offence,
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
/// branches (CLAUDE.md Git 運用). A commit is held only when it
/// demonstrably carries them — named on the line, already staged, or
/// swept in by broad staging while they sit changed.
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
         \"This commit would carry .claude/skills, .claude/rules or \
         .claude/rules-refs onto main from the primary checkout, and those \
         files ride worktree branches: parallel sessions keep reaching for \
         them, and direct commits to main collide (CLAUDE.md Git 運用). \
         Make the edit on a worktree branch and report the branch as ready \
         to merge. If the user asked for this direct commit in so many \
         words, run the same command again with {}=1 in front of it.\"}}}}",
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
/// .claude/skills, .claude/rules or .claude/rules-refs as a path segment.
/// .claude/settings.json stays directly committable; only the files every
/// session loads or greps as rules ride worktree branches.
fn names_shared_rules(text: &str) -> bool {
    let text = text.replace('\\', "/");
    [".claude/skills", ".claude/rules-refs", ".claude/rules"]
        .iter()
        .any(|shared| {
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

/// PreToolUse(Bash|PowerShell): starting the app from a worktree takes
/// something from the sessions beside it — a real window covers whatever
/// is on the screen, and a process nothing ends holds the exe against
/// the next build's link. Offscreen QPA and PG_AUTO_QUIT_MS take
/// neither. Only worktree sessions are held to it — a launch in the
/// primary checkout is the user's own (CLAUDE.md ビルド・テスト).
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

/// PreToolUse(EnterWorktree): a worktree name outside the seat roster
/// starts a cold target/ nobody will reuse (CLAUDE.md ビルド・テスト).
/// Entering by path is how a session takes an existing seat, and creating
/// a missing seat by its own letter is fine; anything else waits for the
/// user to say so.
fn pre_worktree(input: &str) -> Result<(), String> {
    let name = string_field(input, "name");
    let path = string_field(input, "path");
    if let Some(objection) = worktree_objection(name.as_deref(), path.as_deref()) {
        println!(
            "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\
             \"permissionDecision\":\"ask\",\"permissionDecisionReason\":\
             \"{objection} Worktrees are six reusable seats, a-f: enter a \
             free one with path (the session greeting lists them), or create \
             a missing seat by passing its letter as name (CLAUDE.md \
             ビルド・テスト). A worktree outside the roster needs the user's \
             say-so.\"}}}}"
        );
    }
    Ok(())
}

/// Why an EnterWorktree call is held, if it is. Pure so the tests can ask.
fn worktree_objection(name: Option<&str>, path: Option<&str>) -> Option<&'static str> {
    if path.is_some() || name.is_some_and(|name| SEATS.contains(&name)) {
        return None;
    }
    Some(match name {
        Some(_) => {
            "A worktree under a topical name is never reused, so its cold target/ build and its gigabytes are paid for one session."
        }
        None => {
            "A worktree under a generated name is never reused, so its cold target/ build and its gigabytes are paid for one session."
        }
    })
}

/// SessionStart: sessions opened in the primary checkout get the worktree
/// rule injected, and every session gets told where the seats stand, so
/// taking a free one needs no survey. Plain stdout becomes session
/// context for this event.
fn session_start(input: &str) -> Result<(), String> {
    let cwd = string_field(input, "cwd").unwrap_or_default();
    let seats = seat_report(&cwd).unwrap_or_default();
    match worktree_root(&cwd) {
        None => println!(
            "This session runs in the primary checkout. Implementation work \
             belongs in a worktree seat (`claude --worktree <letter>`, or \
             EnterWorktree by path) so parallel sessions do not fight over \
             target/ and the release exe — see CLAUDE.md ビルド・テスト. \
             Document edits and review are fine here, except .claude/skills, \
             .claude/rules and .claude/rules-refs: parallel sessions keep \
             reaching for those same files, and a direct commit to main \
             collides with theirs — edit them on a worktree branch and \
             report the branch as ready to merge (CLAUDE.md Git 運用). \
             {seats}"
        ),
        Some(root) => {
            let name = root.rsplit('/').next().unwrap_or_default();
            if SEATS.contains(&name) {
                if let Some(stand) = seat_stand(&cwd, &seats) {
                    println!("{stand}");
                }
            } else {
                println!(
                    "This session runs in worktree '{name}', outside the \
                     seat roster a-f. Continue this branch's pending work if \
                     that is what the session is for; otherwise take a seat \
                     with EnterWorktree by path (CLAUDE.md ビルド・テスト). \
                     {seats}"
                );
            }
        }
    }
    Ok(())
}

/// One line about the seat this session sits in. A merged seat starts
/// over from main's tip; a seat carrying unmerged commits is a merge
/// waiting to happen, and only its own continuation should build on it.
fn seat_stand(cwd: &str, seats: &str) -> Option<String> {
    let branch = git_query(cwd, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let ahead = commits_in(cwd, &format!("main..{branch}"))?;
    if ahead > 0 {
        return Some(format!(
            "This seat's branch {branch} carries {ahead} unmerged commit(s) \
             — continue that work, or take another seat and leave this one \
             for its merge. {seats}"
        ));
    }
    let behind = commits_in(cwd, &format!("{branch}..main"))?;
    (behind > 0).then(|| {
        format!(
            "This seat's branch {branch} is merged and {behind} behind main \
             — start with `git reset --hard main` so the work begins at the \
             tip (CLAUDE.md ビルド・テスト)."
        )
    })
}

/// The seat roster in one line, read from the same survey `cargo xtask
/// seats` prints, so the answer is the repository's and not a guess.
fn seat_report(cwd: &str) -> Option<String> {
    let survey = seats::survey(cwd)?;
    let buckets = seat_buckets(&survey);
    let mut parts = Vec::new();
    if !buckets.free.is_empty() {
        parts.push(format!("free: {}", buckets.free.join(", ")));
    }
    if !buckets.pending.is_empty() {
        parts.push(format!("waiting for merge: {}", buckets.pending.join(", ")));
    }
    if !buckets.in_use.is_empty() {
        parts.push(format!("in use: {}", buckets.in_use.join(", ")));
    }
    if !buckets.missing.is_empty() {
        parts.push(format!("not created yet: {}", buckets.missing.join(", ")));
    }
    let mut report = format!("Worktree seats — {}.", parts.join("; "));
    if let Some(pick) = spread_pick(&buckets.takeable) {
        report.push_str(&format!(
            " Take seat {pick} this session — the recommendation is \
             randomized so sessions started in one burst spread out. Seats \
             are first come, first served: if creating or entering {pick} \
             fails because another session already has it, take a different \
             free letter instead of retrying this one. A seat's standing \
             here is from this session's start — `cargo xtask seats` is the \
             live check before entering."
        ));
    }
    Some(report)
}

/// The greeting's buckets, and the seats a new session may take.
struct SeatBuckets {
    free: Vec<String>,
    pending: Vec<String>,
    in_use: Vec<String>,
    missing: Vec<&'static str>,
    takeable: Vec<&'static str>,
}

/// Sorts a survey into the greeting's buckets. A lock is a session's own
/// claim, uncommitted changes are a session's work in progress, and
/// commits ahead of main are a merge waiting to happen. Only a seat with
/// none of those is takeable. Pure so the tests can hand it surveys git
/// never produced.
fn seat_buckets(survey: &[seats::Seat]) -> SeatBuckets {
    let mut buckets = SeatBuckets {
        free: Vec::new(),
        pending: Vec::new(),
        in_use: Vec::new(),
        missing: Vec::new(),
        takeable: Vec::new(),
    };
    for seat in survey {
        let name = seat.name;
        let Some(state) = &seat.state else {
            buckets.missing.push(name);
            buckets.takeable.push(name);
            continue;
        };
        if state.locked {
            buckets.in_use.push(format!("{name} (locked)"));
            continue;
        }
        match (state.ahead, state.behind, state.dirty) {
            (Some(ahead), _, dirty) if ahead > 0 => {
                let branch = if state.branch.is_empty() {
                    "detached"
                } else {
                    state.branch.as_str()
                };
                let uncommitted = match dirty {
                    Some(dirty) if dirty > 0 => format!(", {dirty} uncommitted"),
                    _ => String::new(),
                };
                buckets
                    .pending
                    .push(format!("{name} ({branch} +{ahead}{uncommitted})"));
            }
            (Some(0), _, Some(dirty)) if dirty > 0 => {
                buckets
                    .in_use
                    .push(format!("{name} ({dirty} uncommitted change(s))"));
            }
            (Some(0), Some(behind), Some(0)) => {
                if state.branch.is_empty() {
                    // Detached HEAD: nothing pre-git guards stands on it, so
                    // treat it like a merged seat that wants resetting to
                    // the tip.
                    buckets
                        .free
                        .push(format!("{name} (detached — reset --hard main first)"));
                } else if behind > 0 {
                    buckets
                        .free
                        .push(format!("{name} (reset --hard main first)"));
                } else {
                    buckets.free.push(format!("{name} (at main)"));
                }
                buckets.takeable.push(name);
            }
            _ => buckets.in_use.push(format!("{name} (state unreadable)")),
        }
    }
    buckets
}

/// One takeable seat, chosen off the clock's nanoseconds. Sessions started
/// in one burst all read the same inventory, and a deterministic "first
/// free letter" would send every one of them to the same seat. A spread
/// recommendation lets a burst self-assign; the losers of any remaining
/// race are told above to move on rather than retry.
fn spread_pick(takeable: &[&'static str]) -> Option<&'static str> {
    if takeable.is_empty() {
        return None;
    }
    let entropy = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.subsec_nanos() as usize)
        .unwrap_or(0);
    takeable.get(entropy % takeable.len()).copied()
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
        Offence, commit, launch_objections, names_shared_rules, qml_font_notes, reflection,
        resolve, seat_buckets, string_field, worktree_objection,
    };
    use crate::seats::{Seat, SeatState, worktree_root};

    const IN_WORKTREE: &str = "C:/Users/x/IdeaProjects/platitude-gg/.claude/worktrees/launch";
    const PRIMARY: &str = "C:/Users/x/IdeaProjects/platitude-gg";

    #[test]
    fn reads_a_backslashed_cwd_out_of_a_payload_and_into_a_root() {
        let payload = r#"{"session_id":"x","cwd":"C:\\Users\\x\\IdeaProjects\\platitude-gg\\.claude\\worktrees\\nice-satoshi-45da22"}"#;
        let cwd = string_field(payload, "cwd").expect("cwd");
        assert_eq!(
            cwd,
            "C:\\Users\\x\\IdeaProjects\\platitude-gg\\.claude\\worktrees\\nice-satoshi-45da22"
        );
        assert_eq!(
            worktree_root(&cwd).as_deref(),
            Some("C:/Users/x/IdeaProjects/platitude-gg/.claude/worktrees/nice-satoshi-45da22")
        );
    }

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
            // How a merged seat starts over (CLAUDE.md ビルド・テスト): it
            // writes the seat's own branch, and rewrites no history.
            "git reset --hard main",
        ] {
            assert!(reflection(command).is_none(), "{command}");
        }
    }

    #[test]
    fn flags_a_rebase_however_it_starts_and_lets_its_exits_alone() {
        for command in [
            "git rebase main",
            "git rebase -i HEAD~3",
            "git rebase --onto main HEAD~2",
            "git rebase --continue",
            "git rebase --skip",
            "git pull --rebase",
            "git pull -r origin main",
        ] {
            assert_eq!(
                reflection(command).map(|r| r.offence),
                Some(Offence::Rebase),
                "{command}"
            );
        }
        for command in ["git rebase --abort", "git rebase --quit"] {
            assert!(reflection(command).is_none(), "{command}");
        }
        // The demo repositories rebase on purpose, through the task runner —
        // no `git` token, so nothing here sees them.
        for command in [
            "cargo xtask demo-repo rebase-conflict",
            "cargo xtask verify-ui rebase-stop --preset rebase-conflict",
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
        assert_eq!(
            refspec.map(|r| r.offence),
            Some(Offence::LandsOnMain {
                only_from_main: false
            })
        );
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
        assert!(names_shared_rules(".claude/rules-refs/app-ui.md"));
        assert!(!names_shared_rules(".claude/settings.json"));
        assert!(!names_shared_rules("docs/.claude/rules-of-thumb.md"));
        assert!(!names_shared_rules("internal-docs/skills.md"));
    }

    #[test]
    fn holds_worktree_names_outside_the_seat_roster() {
        assert!(worktree_objection(Some("feature-x"), None).is_some());
        assert!(worktree_objection(None, None).is_some());
        assert!(worktree_objection(Some("c"), None).is_none());
        assert!(worktree_objection(None, Some("C:/x/platitude-gg/.claude/worktrees/a")).is_none());
    }

    fn surveyed(branch: &str, locked: bool, ahead: u32, behind: u32, dirty: usize) -> SeatState {
        SeatState {
            branch: branch.to_string(),
            locked,
            ahead: Some(ahead),
            behind: Some(behind),
            dirty: Some(dirty),
            index_age: None,
        }
    }

    #[test]
    fn a_dirty_seat_is_in_use_not_free() {
        let survey = vec![Seat {
            name: "a",
            state: Some(surveyed("worktree-a", false, 0, 0, 13)),
        }];
        let buckets = seat_buckets(&survey);
        assert_eq!(buckets.in_use, vec!["a (13 uncommitted change(s))"]);
        assert!(buckets.free.is_empty(), "{:?}", buckets.free);
        assert!(buckets.takeable.is_empty(), "{:?}", buckets.takeable);
    }

    #[test]
    fn sorts_a_survey_into_the_greeting_buckets() {
        let survey = vec![
            Seat {
                name: "a",
                state: Some(surveyed("worktree-a", false, 1, 0, 13)),
            },
            Seat {
                name: "b",
                state: Some(surveyed("worktree-b", false, 0, 0, 0)),
            },
            Seat {
                name: "c",
                state: Some(surveyed("", false, 0, 3, 0)),
            },
            Seat {
                name: "d",
                state: None,
            },
            Seat {
                name: "e",
                state: Some(surveyed("worktree-e", false, 0, 2, 0)),
            },
            Seat {
                name: "f",
                state: Some(surveyed("worktree-f", true, 0, 0, 0)),
            },
        ];
        let buckets = seat_buckets(&survey);
        assert_eq!(buckets.pending, vec!["a (worktree-a +1, 13 uncommitted)"]);
        assert_eq!(
            buckets.free,
            vec![
                "b (at main)",
                "c (detached — reset --hard main first)",
                "e (reset --hard main first)",
            ]
        );
        assert_eq!(buckets.in_use, vec!["f (locked)"]);
        assert_eq!(buckets.missing, vec!["d"]);
        assert_eq!(buckets.takeable, vec!["b", "c", "d", "e"]);
    }

    #[test]
    fn a_seat_git_cannot_answer_for_is_not_offered() {
        let survey = vec![Seat {
            name: "e",
            state: Some(SeatState {
                branch: "worktree-e".to_string(),
                locked: false,
                ahead: None,
                behind: None,
                dirty: None,
                index_age: None,
            }),
        }];
        let buckets = seat_buckets(&survey);
        assert_eq!(buckets.in_use, vec!["e (state unreadable)"]);
        assert!(buckets.takeable.is_empty(), "{:?}", buckets.takeable);
    }
}
