use std::sync::Mutex;

use super::*;
use crate::process::literal_pathspec;

/// What every invocation carries: a working-tree `diff` refreshes the index
/// under `.git/index.lock` unless `diff.autoRefreshIndex=false` says not to
/// (`--no-optional-locks` does not reach `diff`), and a poll's diff on that
/// lock would kill a concurrent commit. Git alone reads the pair, so the
/// command line is where it is pinned; what git leaves of the index behind
/// it is `exec::periodic`'s.
#[test]
fn every_invocation_keeps_a_diff_off_the_index_lock() {
    assert!(
        FIXED_ARGS
            .windows(2)
            .any(|pair| pair == ["-c", "diff.autoRefreshIndex=false"]),
        "{FIXED_ARGS:?}"
    );
    assert!(
        FIXED_ARGS.contains(&"--no-optional-locks"),
        "{FIXED_ARGS:?}"
    );
}

/// Says `ready`, then sleeps: an observable start edge to cancel after.
fn sleeper() -> Command {
    #[cfg(windows)]
    {
        let mut command = Command::new("powershell");
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Write-Output ready; Start-Sleep -Seconds 30",
        ]);
        command
    }
    #[cfg(not(windows))]
    {
        let mut command = Command::new("sh");
        command.args(["-c", "printf 'ready\\n'; sleep 30"]);
        command
    }
}

fn prepare(mut command: Command) -> Command {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    command
}

#[tokio::test]
async fn timeout_kills_the_child() {
    let (mut child, mut family) =
        super::super::family::spawn(&mut prepare(sleeper()), true, None).unwrap();
    let cancel = CancellationToken::new();
    let outcome = run_child(
        &mut child,
        &mut family,
        Some(Duration::from_millis(300)),
        &cancel,
        &mut |_| {},
    )
    .await
    .unwrap();
    assert!(matches!(outcome, ChildOutcome::TimedOut));
}

#[tokio::test]
async fn cancellation_kills_the_child() {
    let (mut child, mut family) =
        super::super::family::spawn(&mut prepare(sleeper()), true, None).unwrap();
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    let mut ready = false;
    let outcome = run_child(&mut child, &mut family, None, &cancel, &mut |_| {
        ready = true;
        cancel_clone.cancel();
    })
    .await
    .unwrap();
    assert!(ready, "the child announced that it had started");
    assert!(matches!(outcome, ChildOutcome::Cancelled));
}

#[test]
fn describe_joins_arguments() {
    let command = GitCommand::new().args(["log", "--topo-order"]);
    assert_eq!(command.describe(), "git log --topo-order");
}

#[test]
fn describe_quotes_what_a_shell_would_split_or_read() {
    let command = GitCommand::new().args(["stash", "push", "-m", "work in progress"]);
    assert_eq!(command.describe(), "git stash push -m 'work in progress'");
    let command = GitCommand::new().args(["add", "--", &literal_pathspec("a b.txt")]);
    assert_eq!(command.describe(), "git add -- ':(literal)a b.txt'");
}

#[test]
fn a_message_of_several_paragraphs_stays_on_the_one_line_of_the_log() {
    let command =
        GitCommand::new().args(["stash", "push", "--message", "Two paragraphs\n\nin one row"]);
    assert_eq!(
        command.describe(),
        r"git stash push --message $'Two paragraphs\n\nin one row'"
    );
}

#[test]
fn control_characters_are_escaped_and_so_is_what_dollar_quotes_read() {
    // A tab is where the copied line cuts its columns.
    assert_eq!(shell_quote("a\tb"), r"$'a\tb'");
    assert_eq!(shell_quote("it's\r\n"), r"$'it\'s\r\n'");
    assert_eq!(shell_quote("C:\\dir\n"), r"$'C:\\dir\n'");
    // Three digits, so the `1` after the escape stays a character.
    assert_eq!(shell_quote("\u{1b}1"), r"$'\0331'");
    // A C1 control is two bytes, each escaped.
    assert_eq!(shell_quote("a\u{85}b"), r"$'a\302\205b'");
    assert_eq!(shell_quote("行\n"), r"$'行\n'");
    // Without a control character the plain quotes stay.
    assert_eq!(shell_quote("it's"), r"'it'\''s'");
}

#[test]
fn the_full_form_spells_out_what_is_always_applied() {
    // `new()` may spell the program as a path; this is about the shape
    // around it.
    let executor = GitExecutor::with_program("git");
    let full = executor.describe_full(&GitCommand::new().args(["status", "--porcelain=v2"]));
    assert!(full.starts_with("LC_ALL=C "), "{full}");
    assert!(full.contains("GIT_TERMINAL_PROMPT=0"), "{full}");
    assert!(
        full.contains("git -c color.ui=false"),
        "the fixed configuration is part of what ran: {full}"
    );
    assert!(full.ends_with(" status --porcelain=v2"), "{full}");
}

#[test]
fn executor_and_command_environment_are_both_in_the_full_form() {
    let executor = GitExecutor::new().with_env([("GIT_CONFIG_NOSYSTEM", "1")]);
    let full = executor.describe_full(
        &GitCommand::new()
            .env("GIT_EDITOR", "pgg-todo-editor")
            .arg("rebase"),
    );
    assert!(full.contains("GIT_CONFIG_NOSYSTEM=1"), "{full}");
    assert!(full.contains("GIT_EDITOR=pgg-todo-editor"), "{full}");
}

#[derive(Default)]
struct Recorder {
    seen: Mutex<Vec<(u64, String, CommandEnd, String)>>,
}

impl CommandObserver for Recorder {
    fn records(&self, _kept: Kept) -> bool {
        true
    }

    fn started(
        &self,
        display: &str,
        _full: &str,
        _kept: Kept,
        _operation: Option<OperationId>,
    ) -> u64 {
        let mut seen = self.seen.lock().unwrap();
        let id = seen.len() as u64;
        seen.push((id, display.to_string(), CommandEnd::Failed, String::new()));
        id
    }

    fn finished(&self, id: u64, end: CommandEnd, _waited_ms: u64, _elapsed_ms: u64, message: &str) {
        let mut seen = self.seen.lock().unwrap();
        if let Some(entry) = seen.get_mut(id as usize) {
            entry.2 = end;
            entry.3 = message.to_string();
        }
    }
}

#[tokio::test]
async fn the_observer_hears_about_a_command_that_never_started() {
    let recorder = Arc::new(Recorder::default());
    let executor = GitExecutor::with_program("pgg-no-such-program").observed(
        Arc::clone(&recorder) as Arc<dyn CommandObserver>,
        Kept::Asked,
    );
    let output = executor
        .run_unchecked(GitCommand::new().arg("status"), &CancellationToken::new())
        .await;
    assert!(output.is_err());
    let seen = recorder.seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].1, "git status");
    assert_eq!(seen[0].2, CommandEnd::Failed);
    assert!(!seen[0].3.is_empty(), "the reason is reported");
}

/// The token is asked before the slot, so even a free slot is not taken.
/// The program is missing so that a spawn would end `Failed`, not
/// `Cancelled`.
#[tokio::test]
async fn a_token_cancelled_before_the_ask_spawns_nothing() {
    let recorder = Arc::new(Recorder::default());
    let executor = GitExecutor::with_program("pgg-no-such-program").observed(
        Arc::clone(&recorder) as Arc<dyn CommandObserver>,
        Kept::Asked,
    );
    let cancel = CancellationToken::new();
    cancel.cancel();
    let err = executor
        .run(GitCommand::new().arg("status"), &cancel)
        .await
        .expect_err("a cancelled token runs nothing");
    assert!(err.is_cancelled(), "{err:?}");
    let seen = recorder.seen.lock().unwrap();
    assert_eq!(seen.len(), 1, "the ask is still a row");
    assert_eq!(
        seen[0].2,
        CommandEnd::Cancelled,
        "it ended before any spawn"
    );
}

/// Whatever launched this process may carry a repository of its own in the
/// environment; every spawn here takes it off before its own layers go on.
#[test]
fn a_repository_the_environment_names_is_taken_off_every_spawn() {
    let executor = GitExecutor::with_program("git").with_env([("GIT_CONFIG_NOSYSTEM", "1")]);
    let command = executor.assemble(
        &GitCommand::new()
            .env("GIT_INDEX_FILE", "scratch")
            .arg("status"),
    );
    let envs: Vec<(String, Option<String>)> = command
        .as_std()
        .get_envs()
        .map(|(key, value)| {
            (
                key.to_string_lossy().into_owned(),
                value.map(|v| v.to_string_lossy().into_owned()),
            )
        })
        .collect();
    for taken_off in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_EXTERNAL_DIFF",
        "GIT_LITERAL_PATHSPECS",
        "GIT_NAMESPACE",
        "GIT_REFLOG_ACTION",
    ] {
        assert!(
            envs.contains(&(taken_off.to_string(), None)),
            "{taken_off} is taken off: {envs:?}"
        );
    }
    // The executor's and the command's own settings are applied after.
    assert!(envs.contains(&("GIT_CONFIG_NOSYSTEM".to_string(), Some("1".to_string()))));
    assert!(
        envs.contains(&("GIT_INDEX_FILE".to_string(), Some("scratch".to_string()))),
        "a command's scratch index survives the clearing: {envs:?}"
    );
}
