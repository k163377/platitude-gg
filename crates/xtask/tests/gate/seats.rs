//! The roster's claims, end to end: what `cargo xtask seat` hands out and
//! refuses, what `seat release` and `seat takeover` move, and what the
//! hooks hold back at every door a seat has.
//!
//! A claim is a conversation's and no process is asked about it
//! (CLAUDE.md §ビルド・テスト): the Claude process ends whenever the app
//! restarts, so a claim judged by its process is lifted from under a
//! session that is merely between processes.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::support::{EXE, Sandbox, output_past_a_busy_image};

/// A pid no process answers for (`subprocess::NO_SUCH_PID`): a claim
/// carrying it is what a session whose app restarted leaves behind.
const GONE_PID: u32 = 0x7FFF_FFFD;

/// `cargo xtask seat …` under `session`'s marks, set explicitly: the run
/// that started this suite may be a session itself.
fn seat(sb: &Sandbox, args: &[&str], session: &str) -> (bool, String) {
    xtask(sb, &sb.repo, "seat", args, session)
}

/// The same from inside `tree`, with no `--dir`: the roster turns on the
/// working directory, as it does outside the suite.
fn seat_from(sb: &Sandbox, tree: &Path, args: &[&str], session: &str) -> (bool, String) {
    run_xtask(sb, tree, None, "seat", args, session)
}

fn roster(sb: &Sandbox) -> String {
    xtask(sb, &sb.repo, "seats", &[], "a-reader").1
}

fn xtask(sb: &Sandbox, from: &Path, verb: &str, args: &[&str], session: &str) -> (bool, String) {
    run_xtask(sb, from, Some(from), verb, args, session)
}

fn run_xtask(
    sb: &Sandbox,
    from: &Path,
    dir: Option<&Path>,
    verb: &str,
    args: &[&str],
    session: &str,
) -> (bool, String) {
    let mut command = Command::new(EXE);
    command.arg(verb);
    if let Some(dir) = dir {
        command.arg("--dir").arg(dir);
    }
    command.args(args);
    command.current_dir(from);
    sb.env(&mut command);
    command.env("CLAUDE_CODE_SESSION_ID", session);
    command.env("CLAUDE_PID", "4242");
    let output = output_past_a_busy_image(&mut command, || {}).expect("spawn xtask");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.success(), text)
}

/// The letter a `seat` answer names: "seat X is this session's now".
fn letter_of(text: &str) -> String {
    text.split_whitespace()
        .nth(1)
        .unwrap_or_else(|| panic!("no letter in {text:?}"))
        .to_string()
}

/// What a hook says to a tool call from `cwd` under `session`; empty is
/// no objection.
fn hook(sb: &Sandbox, event: &str, session: &str, cwd: &Path, tool: &str, input: &str) -> String {
    let phase = if event.starts_with("post") {
        "PostToolUse"
    } else {
        "PreToolUse"
    };
    sb.hook(
        event,
        &format!(
            "{{\"session_id\":\"{session}\",\"cwd\":\"{}\",\"hook_event_name\":\"{phase}\",\
             \"tool_name\":\"{tool}\",\"tool_input\":{input}}}",
            slashed(cwd)
        ),
    )
}

fn shell(sb: &Sandbox, cwd: &Path, session: &str, command: &str) -> String {
    let input = format!("{{\"command\":\"{}\"}}", command.replace('"', "\\\""));
    hook(sb, "pre-shell", session, cwd, "Bash", &input)
}

fn enter(sb: &Sandbox, cwd: &Path, session: &str, path: &Path) -> String {
    let input = format!("{{\"path\":\"{}\"}}", slashed(path));
    hook(sb, "pre-worktree", session, cwd, "EnterWorktree", &input)
}

/// The entry itself, as the harness reports it once the session is in
/// the tree.
fn entered(sb: &Sandbox, session: &str, path: &Path) -> String {
    let input = format!("{{\"path\":\"{}\"}}", slashed(path));
    hook(sb, "post-worktree", session, path, "EnterWorktree", &input)
}

fn write_door(sb: &Sandbox, cwd: &Path, session: &str, file: &Path) -> String {
    let input = format!("{{\"file_path\":\"{}\"}}", slashed(file));
    hook(sb, "pre-write", session, cwd, "Write", &input)
}

fn wrote(sb: &Sandbox, cwd: &Path, session: &str, file: &Path) -> String {
    let input = format!("{{\"file_path\":\"{}\"}}", slashed(file));
    hook(sb, "post-write", session, cwd, "Write", &input)
}

fn slashed(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn letter_path(sb: &Sandbox, name: &str) -> PathBuf {
    sb.repo.join(".claude/worktrees").join(name)
}

/// A roster letter with a tree of its own at main's tip, unclaimed.
fn letter(sb: &Sandbox, name: &str) -> PathBuf {
    sb.roster_tree(name, &format!("worktree-{name}"))
}

/// The same, one commit ahead of main.
fn letter_carrying_work(sb: &Sandbox, name: &str) -> PathBuf {
    let path = letter(sb, name);
    sb.write(&path, "internal-docs/notes.md", "# notes\n\nthe work\n");
    sb.commit_all(
        &path,
        "docs: the work in this letter",
        &[("PGG_GATE_SKIP", "1")],
    );
    path
}

/// Every roster letter but `spare` made unavailable, as the walk's order
/// differs run to run (`seats::spread_order`). `a` is taken already: the
/// sandbox's own seat has `worktree-a` checked out.
fn only_letter_free(sb: &Sandbox, spare: &str) {
    for name in ["b", "c", "d", "e", "f"] {
        if name != spare {
            letter_carrying_work(sb, name);
        }
    }
}

fn lock(sb: &Sandbox, path: &Path, reason: &str) {
    sb.git_ok(
        &sb.repo,
        &["worktree", "lock", "--reason", reason, &slashed(path)],
    );
}

/// The lock's reason on one letter, empty when unlocked. Read from the
/// admin directory: `worktree list` spells the temp path resolved, not
/// the short form the suite knows.
fn lock_on(sb: &Sandbox, name: &str) -> String {
    let locked = sb.repo.join(".git/worktrees").join(name).join("locked");
    std::fs::read_to_string(locked)
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// `cargo xtask shots add` from inside `tree` under `session`'s marks, as
/// verify-ui runs it.
fn shot(sb: &Sandbox, tree: &Path, session: &str) -> (bool, String) {
    let png = sb.root.join("shot.png");
    // A PNG header is all the board reads of a picture (`png_size`):
    // the signature, then the IHDR length, type, width and height.
    let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    bytes.extend([0, 0, 0, 0x0d]);
    bytes.extend(b"IHDR");
    bytes.extend([0, 0, 0, 1, 0, 0, 0, 1]);
    std::fs::write(&png, &bytes).expect("a picture");
    let mut command = Command::new(EXE);
    command.args(["shots", "add", "--label", "この run で何を見るか"]);
    command.arg(&png);
    command.current_dir(tree);
    sb.env(&mut command);
    command.env("CLAUDE_CODE_SESSION_ID", session);
    command.env("CLAUDE_PID", "4242");
    let output = output_past_a_busy_image(&mut command, || {}).expect("spawn xtask");
    (
        output.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    )
}

/// A run on the board under `letter`, as verify-ui leaves one; answers
/// the run file.
fn picture_on_the_board(sb: &Sandbox, letter: &str) -> PathBuf {
    sb.put_up(letter, "a picture", 1_700_000_000_000)
}

/// Made at main's tip and claimed in the same step.
#[test]
fn a_letter_is_handed_out_once_per_session_and_stays_that_sessions() {
    let sb = Sandbox::new("seat-handout");
    let (ok, first) = seat(&sb, &[], "one");
    assert!(ok && first.contains("newly created"), "{first}");
    let taken = letter_of(&first);
    assert_eq!(lock_on(&sb, &taken), "claude-seat one pid 4242");
    assert_eq!(sb.head(&letter_path(&sb, &taken)), sb.main_sha());

    let (ok, again) = seat(&sb, &[], "one");
    assert!(
        ok && again.contains(&format!("seat {taken} was already this session's")),
        "{again}"
    );
    let (ok, other) = seat(&sb, &[], "two");
    assert!(ok, "{other}");
    assert_ne!(letter_of(&other), taken, "{other}");
    assert_eq!(lock_on(&sb, &letter_of(&other)), "claude-seat two pid 4242");
}

/// Emptiness is asked of a letter only before handing it to somebody
/// else. A claim comes off a tree its session never left whenever it
/// lands or releases; answering with a different letter there strands the
/// work in one the roster can hand to nobody.
#[test]
fn the_letter_a_session_stands_in_comes_back_with_its_work() {
    let sb = Sandbox::new("seat-standing-in");
    let tree = letter_carrying_work(&sb, "b");
    let carried = sb.head(&tree);
    let run = picture_on_the_board(&sb, "b");

    let (ok, text) = seat_from(&sb, &tree, &[], "mine");
    assert!(ok, "{text}");
    assert!(text.contains("seat b is this session's now"), "{text}");
    assert!(
        text.contains("it carries 1 commit(s) main does not have"),
        "what it carries is said, not silently left behind: {text}"
    );
    assert_eq!(lock_on(&sb, "b"), "claude-seat mine pid 4242");
    assert_eq!(sb.head(&tree), carried, "nothing was moved: {text}");
    assert!(run.exists(), "and the board kept this letter's pictures");

    // Asked again from the same tree, the same letter, now by its claim.
    let (ok, again) = seat_from(&sb, &tree, &[], "mine");
    assert!(
        ok && again.contains("seat b was already this session's"),
        "{again}"
    );
}

/// Standing in a tree is not a claim on it — the one path by which the
/// answer above could hand out a seat it should not.
#[test]
fn a_letter_somebody_else_holds_is_passed_over_even_from_inside_it() {
    let sb = Sandbox::new("seat-standing-in-theirs");
    let tree = letter_carrying_work(&sb, "b");
    let theirs = format!("claude-seat theirs pid {GONE_PID}");
    lock(&sb, &tree, &theirs);
    let carried = sb.head(&tree);

    let (ok, text) = seat_from(&sb, &tree, &[], "mine");
    assert!(ok, "{text}");
    assert_ne!(letter_of(&text), "b", "the letter is theirs: {text}");
    assert_eq!(lock_on(&sb, "b"), theirs, "their claim stood: {text}");
    assert_eq!(sb.head(&tree), carried, "and their work: {text}");
}

/// A letter is empty when its branch is: a seat detached at main's tip
/// reads as clean while `worktree-<letter>` carries commits, and a fresh
/// stretch there resets that branch onto main.
#[test]
fn a_detached_seat_over_a_branch_that_carries_work_begins_nothing() {
    let sb = Sandbox::new("seat-detached");
    let tree = letter_carrying_work(&sb, "b");
    let carried = sb.head(&tree);
    sb.git_ok(&tree, &["switch", "--detach", "main"]);
    assert_eq!(sb.head(&tree), sb.main_sha(), "the tree looks empty");

    let (ok, text) = seat_from(&sb, &tree, &[], "mine");
    assert!(
        ok && text.contains("seat b is this session's now"),
        "{text}"
    );
    assert!(
        !text.contains("started at main's tip"),
        "nothing begins here: {text}"
    );
    assert_eq!(
        sb.git_ok(&sb.repo, &["rev-parse", "worktree-b"]),
        carried,
        "the letter's branch kept its commits: {text}"
    );
}

/// The letter goes to nobody else, whatever the seat carries, and back to
/// the conversation when it returns; its pictures stand.
#[test]
fn a_claim_with_a_dead_number_is_still_a_claim() {
    let sb = Sandbox::new("seat-claim-stands");
    let empty = letter(&sb, "b");
    let claim = format!("claude-seat sleeping pid {GONE_PID} born 1 as claude.exe");
    lock(&sb, &empty, &claim);
    let run = picture_on_the_board(&sb, "b");
    only_letter_free(&sb, "b");

    let (ok, text) = seat(&sb, &[], "another");
    assert!(!ok, "no letter is free to hand out: {text}");
    assert!(
        text.contains("b: held by session sleeping (pid 2147483645)"),
        "{text}"
    );
    assert!(
        text.contains("c: unclaimed, carrying 1 commit(s) main has not and 0 uncommitted file(s)"),
        "{text}"
    );
    assert!(
        text.contains("PGG_ALLOW_TAKEOVER=1 cargo xtask seat takeover"),
        "the refusal names the one thing that moves a letter: {text}"
    );
    assert_eq!(lock_on(&sb, "b"), claim, "the claim stood");
    assert!(run.exists(), "and so did the pictures");

    let (ok, text) = seat(&sb, &[], "sleeping");
    assert!(
        ok && text.contains("seat b was already this session's"),
        "{text}"
    );
    assert!(run.exists());
    let table = roster(&sb);
    assert!(
        table.contains("held by session sleeping (pid 2147483645)"),
        "{table}"
    );
    assert!(table.contains("no process is asked"), "{table}");
}

/// Its old pictures are swept, as a fresh stretch begins there; released,
/// it is handed out again.
#[test]
fn an_unclaimed_letter_at_main_is_handed_out_and_its_old_pictures_go() {
    let sb = Sandbox::new("seat-fresh");
    letter(&sb, "b");
    let run = picture_on_the_board(&sb, "b");
    only_letter_free(&sb, "b");

    let (ok, text) = seat(&sb, &[], "next");
    assert!(
        ok && text.contains("seat b is this session's now"),
        "{text}"
    );
    assert_eq!(lock_on(&sb, "b"), "claude-seat next pid 4242");
    assert!(
        !run.exists(),
        "the pictures of the stretch that ended here left with it"
    );

    let (ok, text) = seat(&sb, &["release"], "next");
    assert!(ok && text.contains("seat b released"), "{text}");
    assert_eq!(lock_on(&sb, "b"), "");
    let (ok, text) = seat(&sb, &["release"], "next");
    assert!(!ok && text.contains("holds no seat"), "{text}");
    let (ok, text) = seat(&sb, &[], "after");
    assert!(
        ok && text.contains("seat b is this session's now"),
        "{text}"
    );
}

/// The work goes on rather than ending. A lock a person wrote is nobody's
/// to take.
#[test]
fn a_takeover_moves_the_letter_with_its_work_and_pictures() {
    let sb = Sandbox::new("seat-takeover");
    let tree = letter_carrying_work(&sb, "b");
    let carried = sb.head(&tree);
    lock(&sb, &tree, "claude-seat theirs pid 123");
    let run = picture_on_the_board(&sb, "b");

    let (ok, text) = seat(&sb, &["takeover", "b"], "mine");
    assert!(ok, "{text}");
    assert!(
        text.contains("seat b is this session's now (it was held by session theirs (pid 123))"),
        "{text}"
    );
    assert!(
        text.contains("it carries 1 commit(s) main does not have and 0 uncommitted file(s)"),
        "{text}"
    );
    assert_eq!(lock_on(&sb, "b"), "claude-seat mine pid 4242");
    assert_eq!(sb.head(&tree), carried, "nothing moved: {text}");
    assert!(run.exists(), "the pictures are the new holder's to read");

    let (ok, text) = seat(&sb, &[], "mine");
    assert!(
        ok && text.contains("seat b was already this session's"),
        "{text}"
    );
    let (ok, text) = seat(&sb, &["takeover", "b"], "mine");
    assert!(
        ok && text.contains("seat b was already this session's"),
        "{text}"
    );

    letter_carrying_work(&sb, "c");
    let (ok, text) = seat(&sb, &["takeover", "c"], "mine");
    assert!(ok && text.contains("(it was unclaimed)"), "{text}");

    let parked = letter(&sb, "d");
    lock(&sb, &parked, "parked by hand");
    let (ok, text) = seat(&sb, &["takeover", "d"], "mine");
    assert!(!ok && text.contains("locked by hand"), "{text}");
    assert_eq!(lock_on(&sb, "d"), "parked by hand");

    let (ok, text) = seat(&sb, &["takeover", "z"], "mine");
    assert!(!ok && text.contains("no roster letter"), "{text}");
}

/// A letter the roster never made is made by the takeover, at main's tip.
#[test]
fn a_session_holding_two_letters_releases_by_name() {
    let sb = Sandbox::new("seat-two");
    let (ok, text) = seat(&sb, &[], "mine");
    assert!(ok, "{text}");
    let first = letter_of(&text);
    let other = if first == "b" { "c" } else { "b" };

    let (ok, text) = seat(&sb, &["takeover", other], "mine");
    assert!(ok && text.contains("newly created"), "{text}");
    assert_eq!(sb.head(&letter_path(&sb, other)), sb.main_sha());
    assert_eq!(lock_on(&sb, other), "claude-seat mine pid 4242");

    let (ok, text) = seat(&sb, &["release"], "mine");
    assert!(!ok && text.contains("name the one to release"), "{text}");
    let (ok, text) = seat(&sb, &["release", "z"], "mine");
    assert!(!ok && text.contains("does not hold seat z"), "{text}");
    let (ok, text) = seat(&sb, &["release", other], "mine");
    assert!(
        ok && text.contains(&format!("seat {other} released")),
        "{text}"
    );
    let (ok, text) = seat(&sb, &["release"], "mine");
    assert!(
        ok && text.contains(&format!("seat {first} released")),
        "{text}"
    );
    assert_eq!(lock_on(&sb, &first), "");
    assert_eq!(lock_on(&sb, other), "");
}

/// It cannot be handed out (its branch carries commits); a takeover grows
/// it back whether or not git still lists the missing tree.
#[test]
fn a_letter_that_lost_its_tree_is_grown_back_by_a_takeover() {
    let sb = Sandbox::new("seat-grow-back");
    let tree = letter_carrying_work(&sb, "c");
    let carried = sb.head(&tree);
    sb.git_ok(
        &sb.repo,
        &["worktree", "remove", "--force", &slashed(&tree)],
    );
    assert!(!tree.exists(), "the tree is gone, the branch is not");
    let table = roster(&sb);
    assert!(
        table.contains("worktree-c") && table.contains("no tree"),
        "{table}"
    );
    only_letter_free(&sb, "c");
    let (ok, text) = seat(&sb, &[], "another");
    assert!(
        !ok && text.contains("c: no tree on disk while worktree-c carries 1 commit(s)"),
        "{text}"
    );

    let (ok, text) = seat(&sb, &["takeover", "c"], "the-work-s-session");
    assert!(
        ok && text.contains("grown back on worktree-c with its 1 commit(s)"),
        "{text}"
    );
    assert!(tree.exists(), "{text}");
    assert_eq!(sb.head(&tree), carried, "a takeover moves nothing: {text}");
    assert_eq!(lock_on(&sb, "c"), "claude-seat the-work-s-session pid 4242");

    // The directory going while git still lists the tree: the same
    // letter, the same way out.
    let (ok, text) = seat(&sb, &["release", "c"], "the-work-s-session");
    assert!(ok, "{text}");
    std::fs::remove_dir_all(&tree).expect("the directory going out from under the letter");
    let table = roster(&sb);
    assert!(table.contains("no tree"), "{table}");
    let (ok, text) = seat(&sb, &["takeover", "c"], "again");
    assert!(ok && text.contains("grown back"), "{text}");
    assert_eq!(sb.head(&tree), carried, "{text}");
}

/// An empty letter at main's tip is recycled, but not while somebody holds
/// it: clearing the record there would lift a claim nobody asked to move.
#[test]
fn a_letter_whose_tree_went_missing_is_still_its_sessions() {
    let sb = Sandbox::new("seat-missing-tree-claim");
    let tree = letter(&sb, "b");
    let claim = format!("claude-seat sleeping pid {GONE_PID}");
    lock(&sb, &tree, &claim);
    std::fs::remove_dir_all(&tree).expect("the directory going out from under the letter");
    only_letter_free(&sb, "b");

    let (ok, text) = seat(&sb, &[], "another");
    assert!(
        !ok,
        "a letter somebody holds is not the roster's to hand out: {text}"
    );
    assert!(
        text.contains("b: held by session sleeping (pid 2147483645), and no tree on disk"),
        "the refusal names the holder and what went missing: {text}"
    );
    assert_eq!(lock_on(&sb, "b"), claim, "the claim stood: {text}");
    assert!(!tree.exists(), "and nothing was made in its place: {text}");

    // Once the user says the word, the tree comes back, naming whose it was.
    let (ok, text) = seat(&sb, &["takeover", "b"], "mine");
    assert!(ok, "{text}");
    assert!(
        text.contains("(it was held by session sleeping (pid 2147483645))"),
        "{text}"
    );
    assert!(tree.exists(), "{text}");
    assert_eq!(lock_on(&sb, "b"), "claude-seat mine pid 4242");
}

/// A letter just landed from stands unclaimed, and a picture of what
/// landed would hold it for a session whose work is done (CLAUDE.md §Git
/// 運用): a run claims nothing, so it goes up only from a letter its
/// session holds — and never from somebody else's.
#[test]
fn a_picture_goes_up_only_from_a_seat_its_session_holds() {
    let sb = Sandbox::new("seat-board-claim");
    let tree = letter(&sb, "b");
    let runs = sb.repo.join(".shots").join("runs");
    let on_the_board = || std::fs::read_dir(&runs).map_or(0, Iterator::count);

    let (ok, text) = shot(&sb, &tree, "mine");
    assert!(!ok, "{text}");
    assert!(
        text.contains("carries no claim of this session's"),
        "{text}"
    );
    assert_eq!(lock_on(&sb, "b"), "", "a picture claims nothing");
    assert_eq!(on_the_board(), 0, "{text}");

    lock(&sb, &tree, "claude-seat mine pid 4242");
    let (ok, text) = shot(&sb, &tree, "mine");
    assert!(ok, "{text}");
    assert_eq!(on_the_board(), 1, "{text}");

    let (ok, text) = shot(&sb, &tree, "another");
    assert!(!ok, "{text}");
    assert!(
        text.contains("seat b is held by session mine (pid 4242)"),
        "{text}"
    );
    assert_eq!(lock_on(&sb, "b"), "claude-seat mine pid 4242");
    assert_eq!(on_the_board(), 1, "{text}");
}

/// A seat its landing handed back stays free through a conversation
/// carried on — a compaction or a resume in it claims nothing, and the
/// next edit claims it if the work goes on — while a conversation that
/// begins in it claims it as it starts.
#[test]
fn only_a_conversation_beginning_in_a_seat_claims_it_as_it_starts() {
    let sb = Sandbox::new("seat-start-claim");
    let tree = letter(&sb, "b");
    let start = |source: &str| {
        sb.hook(
            "session-start",
            &format!(
                "{{\"session_id\":\"mine\",\"cwd\":\"{}\",\"hook_event_name\":\"SessionStart\",\
                 \"source\":\"{source}\"}}",
                slashed(&tree)
            ),
        )
    };
    for carried_on in ["compact", "resume"] {
        start(carried_on);
        assert_eq!(lock_on(&sb, "b"), "", "a {carried_on} claims nothing");
    }
    start("startup");
    assert_eq!(lock_on(&sb, "b"), "claude-seat mine");
}

/// Only the seat this session holds is entered or written in, however
/// dead another claim's number; a write into an unclaimed seat the
/// session stands in re-claims it; takeover needs the user's flag.
#[test]
fn the_hooks_hold_every_door_a_seat_has() {
    let sb = Sandbox::new("seat-doors");
    let mine = letter(&sb, "b");
    lock(&sb, &mine, "claude-seat mine pid 4242");
    let theirs = letter(&sb, "c");
    lock(&sb, &theirs, &format!("claude-seat theirs pid {GONE_PID}"));
    let free = letter(&sb, "d");

    assert_eq!(
        enter(&sb, &sb.repo, "mine", &mine),
        "",
        "the claim is the proof the door asks for"
    );
    let refusal = enter(&sb, &sb.repo, "mine", &theirs);
    assert!(
        refusal.contains("\"deny\"") && refusal.contains("held by session theirs"),
        "{refusal}"
    );
    assert!(refusal.contains("PGG_ALLOW_TAKEOVER=1"), "{refusal}");
    let refusal = enter(&sb, &sb.repo, "mine", &free);
    assert!(
        refusal.contains("\"deny\"") && refusal.contains("carries no claim"),
        "{refusal}"
    );

    assert_eq!(write_door(&sb, &mine, "mine", &mine.join("notes.md")), "");
    let refusal = write_door(&sb, &sb.repo, "mine", &theirs.join("notes.md"));
    assert!(
        refusal.contains("\"deny\"") && refusal.contains("held by session theirs"),
        "{refusal}"
    );
    assert_eq!(
        write_door(&sb, &free, "mine", &free.join("notes.md")),
        "",
        "an unclaimed seat is written in, and re-claimed after"
    );
    let note = wrote(&sb, &free, "mine", &free.join("notes.md"));
    assert!(note.contains("re-claimed"), "{note}");
    assert_eq!(lock_on(&sb, "d"), "claude-seat mine");

    let refusal = shell(&sb, &sb.repo, "mine", "cargo xtask seat takeover c");
    assert!(
        refusal.contains("\"deny\"") && refusal.contains("PGG_ALLOW_TAKEOVER=1"),
        "{refusal}"
    );
    for command in [
        "PGG_ALLOW_TAKEOVER=1 cargo xtask seat takeover c",
        "cargo xtask seat",
        "cargo xtask seat release",
        "cargo xtask seats",
    ] {
        assert_eq!(shell(&sb, &sb.repo, "mine", command), "", "{command}");
    }

    let greeting = hook(&sb, "session-start", "mine", &sb.repo, "", "{}");
    assert!(
        greeting.contains("in use: b (locked), c (locked)"),
        "a claim is a session in the seat, whatever became of its process: {greeting}"
    );
}

/// Every other worktree and repository goes through: the guard measures
/// the path, not the spelling.
#[test]
fn the_git_that_would_move_a_letter_by_hand_is_held_back() {
    let sb = Sandbox::new("seat-hand-git");
    let mine = letter(&sb, "b");
    lock(&sb, &mine, "claude-seat mine pid 4242");
    letter(&sb, "c");

    for command in [
        "git worktree remove .claude/worktrees/c",
        "git worktree remove --force .claude/worktrees/c",
        "git worktree lock --reason \"claude-seat mine\" .claude/worktrees/c",
        "git worktree unlock .claude/worktrees/b",
        "git worktree add .claude/worktrees/f worktree-f",
        "git worktree move .claude/worktrees/c ../elsewhere",
        "git branch -D worktree-c",
    ] {
        let refusal = shell(&sb, &sb.repo, "mine", command);
        assert!(refusal.contains("\"deny\""), "{command}: {refusal}");
        assert!(refusal.contains("seat takeover"), "{command}: {refusal}");
    }
    // From inside a seat the repository is named relatively: a base left
    // unresolved leaves the path after it relative, naming no seat.
    for command in [
        "git -C ../../.. worktree remove .claude/worktrees/c",
        "cd ../../.. && git worktree remove .claude/worktrees/c",
        "git worktree remove ../c",
    ] {
        let refusal = shell(&sb, &mine, "mine", command);
        assert!(refusal.contains("\"deny\""), "{command}: {refusal}");
    }
    let topical = sb.root.join("topical");
    sb.git_ok(
        &sb.repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "topical",
            &slashed(&topical),
            "main",
        ],
    );
    let remove = format!("git worktree remove {}", slashed(&topical));
    assert_eq!(shell(&sb, &sb.repo, "mine", &remove), "", "nobody's seat");
    assert_eq!(
        shell(&sb, &sb.repo, "mine", "git branch -D topical"),
        "",
        "nobody's letter"
    );
    // A repository that keeps no seats has no letters, whatever its
    // branches are called — the throwaway a session measures git in.
    let throwaway = sb.root.join("throwaway");
    std::fs::create_dir_all(&throwaway).expect("a repository of its own");
    sb.git_ok(&throwaway, &["init", "-q", "-b", "main"]);
    sb.git_ok(
        &throwaway,
        &["commit", "-q", "--allow-empty", "-m", "probe"],
    );
    sb.git_ok(&throwaway, &["branch", "worktree-c"]);
    assert_eq!(
        shell(&sb, &throwaway, "mine", "git branch -D worktree-c"),
        "",
        "a throwaway repository's branches are its own"
    );
}

/// Settled by the entry the session made, not by where the reporting
/// shell stands: `cd <checkout> && cargo xtask seat` runs in the checkout
/// while the session works in the seat, and EnterWorktree refuses the
/// tree it is run from.
#[test]
fn a_seat_this_session_entered_is_not_one_to_enter_again() {
    let sb = Sandbox::new("entered");
    let (ok, first) = seat(&sb, &[], "s1");
    assert!(ok, "{first}");
    assert!(first.contains("enter it with EnterWorktree"), "{first}");
    let tree = sb.repo.join(".claude/worktrees").join(letter_of(&first));

    entered(&sb, "s1", &tree);

    let (ok, again) = seat(&sb, &[], "s1");
    assert!(ok, "{again}");
    assert!(again.contains("nothing to enter"), "{again}");
    assert!(!again.contains("EnterWorktree"), "{again}");

    // The mark is one session's.
    let (ok, other) = seat(&sb, &[], "s2");
    assert!(ok, "{other}");
    assert!(other.contains("enter it with EnterWorktree"), "{other}");
}
