//! The roster's claims, end to end, against a repository of the suite's
//! own: what `cargo xtask seat` hands out and refuses, what `seat
//! release` and `seat takeover` move, and what the hooks hold back at
//! every door a seat has.
//!
//! A seat claim is a conversation's, and no process is asked about it.
//! The Claude process behind a conversation ends every time the app
//! restarts, and a claim judged dead by its process was lifted from
//! under a session that was merely between processes — its letter
//! handed to a stranger and its pictures swept (observed 2026-09-19).
//! So the shapes pinned here are the ones the roster has to get right
//! whatever the processes do: a claim with a dead number is still a
//! claim, and only landing, releasing and the user's takeover move one.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::support::{EXE, Sandbox, output_past_a_busy_image};

/// A pid no process answers for (`subprocess::NO_SUCH_PID`): a claim
/// carrying it is what a session whose app restarted leaves behind.
const GONE_PID: u32 = 0x7FFF_FFFD;

/// `cargo xtask seat …` against the sandbox, under one session's marks.
/// The marks are set explicitly because the run that started this suite
/// may be a session itself, and its id would otherwise be the answer.
fn seat(sb: &Sandbox, args: &[&str], session: &str) -> (bool, String) {
    xtask(sb, "seat", args, session)
}

/// `cargo xtask seats` against the sandbox.
fn roster(sb: &Sandbox) -> String {
    xtask(sb, "seats", &[], "a-reader").1
}

fn xtask(sb: &Sandbox, verb: &str, args: &[&str], session: &str) -> (bool, String) {
    let mut command = Command::new(EXE);
    command.arg(verb).arg("--dir").arg(&sb.repo);
    command.args(args);
    command.current_dir(&sb.repo);
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

/// What a hook says to a tool call from `cwd` under `session`. Empty is
/// the hook's way of saying it has no objection.
fn hook(sb: &Sandbox, event: &str, session: &str, cwd: &Path, tool: &str, input: &str) -> String {
    let file = sb.root.join("hook-payload.json");
    let phase = if event.starts_with("post") {
        "PostToolUse"
    } else {
        "PreToolUse"
    };
    let payload = format!(
        "{{\"session_id\":\"{session}\",\"cwd\":\"{}\",\"hook_event_name\":\"{phase}\",\
         \"tool_name\":\"{tool}\",\"tool_input\":{input}}}",
        slashed(cwd)
    );
    std::fs::write(&file, payload).expect("payload");
    let mut process = Command::new(EXE);
    process
        .args(["hook", event])
        .stdin(Stdio::from(File::open(&file).expect("payload open")));
    process.current_dir(&sb.repo);
    sb.env(&mut process);
    // The payload's id is the session's; the environment's, if the run
    // that started this suite is a session, is not.
    process.env_remove("CLAUDE_CODE_SESSION_ID");
    process.env_remove("CLAUDE_PID");
    let output = output_past_a_busy_image(&mut process, || {}).expect("spawn xtask");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn shell(sb: &Sandbox, cwd: &Path, session: &str, command: &str) -> String {
    let input = format!("{{\"command\":\"{}\"}}", command.replace('"', "\\\""));
    hook(sb, "pre-shell", session, cwd, "Bash", &input)
}

fn enter(sb: &Sandbox, cwd: &Path, session: &str, path: &Path) -> String {
    let input = format!("{{\"path\":\"{}\"}}", slashed(path));
    hook(sb, "pre-worktree", session, cwd, "EnterWorktree", &input)
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
    let path = letter_path(sb, name);
    let branch = format!("worktree-{name}");
    sb.git_ok(
        &sb.repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            &branch,
            &slashed(&path),
            "main",
        ],
    );
    path
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

/// Every roster letter but `spare` made unavailable, so the walk can
/// only end where the test means it to — it tries the letters in an
/// order that differs run to run (`seats::spread_order`), the lock being
/// what decides in earnest. `a` needs nothing done to it: the sandbox's
/// own seat stands on `worktree-a` beside the roster, and a branch
/// checked out twice is a branch git refuses.
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

/// The lock's reason on one letter, empty when unlocked. Read out of the
/// tree's own admin directory rather than out of `worktree list`, whose
/// paths are git's resolved spelling of a temp directory the suite knows
/// only by its short one.
fn lock_on(sb: &Sandbox, name: &str) -> String {
    let locked = sb.repo.join(".git/worktrees").join(name).join("locked");
    std::fs::read_to_string(locked)
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// `cargo xtask shots add` from inside one letter's tree, under one
/// session's marks — the shape verify-ui puts a run on the board in.
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
    command.args(["shots", "add", "--label", "what this run shows"]);
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

/// A run on the board under one letter, as verify-ui leaves one: the
/// file that names the picture, and the picture. Answers the run file.
fn picture_on_the_board(sb: &Sandbox, letter: &str) -> PathBuf {
    let board = sb.repo.join(".shots");
    for dir in ["img", "runs"] {
        std::fs::create_dir_all(board.join(dir)).expect("board dir");
    }
    let stem = format!("1700000000000-{letter}-a-picture");
    let png = format!("img/{stem}-0-shot.png");
    std::fs::write(board.join(&png), b"png").expect("picture");
    let run = board.join("runs").join(format!("{stem}.tsv"));
    std::fs::write(
        &run,
        format!("label\ta picture\nverb\tv\nseat\t{letter}\nat\t1700000000000\nshot\t{png}\tshot.png\t1\t1\t\n"),
    )
    .expect("run");
    run
}

/// The roster hands each session a letter of its own, made at main's
/// tip and claimed in the same step; asked again, the same session gets
/// the same letter.
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

/// The rule the roster hangs on. A claim whose process is gone is still
/// its session's — the app restarted under the conversation — so the
/// roster hands the letter to nobody else, whatever the seat carries,
/// and to the conversation itself when it comes back. Its pictures
/// stand through all of it.
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

/// A letter nobody holds, at main's tip with nothing in it, is the one
/// the roster hands out — put on its branch at main, and its old
/// pictures swept: a fresh stretch of work is beginning there. Released,
/// it is handed out again to whoever asks.
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

/// A takeover moves the letter to the session that was told to take it,
/// with the tree as it stands and the pictures kept: the work goes on
/// rather than ending. A lock a person wrote is nobody's to take.
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

/// A session holding two letters — a takeover on top of its own — is
/// told so, and releases by name; a letter the roster never made is
/// made by the takeover, at main's tip.
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

/// A letter whose tree went away cannot be handed out — its branch
/// carries commits — and cannot be reached; a takeover grows the tree
/// back on that branch, whether git still lists the missing tree or not.
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

/// The tree is what goes missing, not the word that says whose the
/// letter is. A letter at main's tip with nothing in it is what the
/// roster recycles — but not while somebody holds it, and a directory
/// going out from under a claim is no more the roster's business than a
/// process ending is. Clearing the record to make room there would lift
/// a claim nobody asked to move.
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

    // The same letter once the user says the word: the record is cleared
    // and the tree comes back, with whose it was named on the way out.
    let (ok, text) = seat(&sb, &["takeover", "b"], "mine");
    assert!(ok, "{text}");
    assert!(
        text.contains("(it was held by session sleeping (pid 2147483645))"),
        "{text}"
    );
    assert!(tree.exists(), "{text}");
    assert_eq!(lock_on(&sb, "b"), "claude-seat mine pid 4242");
}

/// A picture is work, and work holds the seat. The letter a session
/// just landed from stands unclaimed with its tree warm, and the next
/// thing that session does there is often a picture of what landed —
/// so a run takes the claim back, the way an edit does. A run from a
/// letter somebody else holds takes nothing and says so.
#[test]
fn a_picture_holds_the_seat_it_was_taken_in() {
    let sb = Sandbox::new("seat-board-claim");
    let tree = letter(&sb, "b");

    let (ok, text) = shot(&sb, &tree, "mine");
    assert!(ok, "{text}");
    assert!(
        text.contains("stood unclaimed, and this run claimed it back"),
        "{text}"
    );
    assert_eq!(lock_on(&sb, "b"), "claude-seat mine pid 4242");

    // Quiet once the letter is already this session's.
    let (ok, text) = shot(&sb, &tree, "mine");
    assert!(ok && !text.contains("claimed it back"), "{text}");
    assert_eq!(lock_on(&sb, "b"), "claude-seat mine pid 4242");

    // And a picture of somebody else's tree is theirs: it goes on the
    // board, and takes nothing.
    let (ok, text) = shot(&sb, &tree, "another");
    assert!(ok, "{text}");
    assert!(
        text.contains("seat b is held by session mine (pid 4242)"),
        "{text}"
    );
    assert_eq!(lock_on(&sb, "b"), "claude-seat mine pid 4242");
}

/// The doors a seat has of its own. Only the seat this session holds is
/// entered or written in, and a claim is refused however dead its
/// number; a write into an unclaimed seat the session stands in
/// re-claims it; and the takeover verb goes through only behind the
/// flag that says the user asked for that letter.
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

/// The git that would do a seat verb's work by hand: making, removing,
/// moving, locking or unlocking a letter's tree, and deleting its
/// branch. Every other worktree and every other repository goes without
/// a word — the guard measures the path, not the spelling.
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
    // The same lines run from inside a seat, where the repository is
    // named relatively: a base left unresolved leaves the path after it
    // relative too, which names no seat and let every such line through.
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
