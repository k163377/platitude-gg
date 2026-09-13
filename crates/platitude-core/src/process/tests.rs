use std::sync::Mutex;

use super::*;
use crate::process::literal_pathspec;

/// A cross-platform command that announces it is running, then sleeps for
/// ~30s, used to exercise cancellation after an observable start edge.
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
    let mut child = prepare(sleeper()).spawn().unwrap();
    let cancel = CancellationToken::new();
    let outcome = run_child(
        &mut child,
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
    let mut child = prepare(sleeper()).spawn().unwrap();
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();
    let mut ready = false;
    let outcome = run_child(&mut child, None, &cancel, &mut |_| {
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
fn the_full_form_spells_out_what_is_always_applied() {
    // Named, not resolved: `new()` may spell the program as a path
    // (`program`), and this is about the shape around it.
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
