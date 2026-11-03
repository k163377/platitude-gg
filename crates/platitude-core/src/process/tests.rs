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
        None,
        Some(Duration::from_millis(300)),
        &cancel,
        &mut Stdout::new(&mut Out::Streamed(&mut |_| {}), false),
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
    let mut on_stdout = |_: &[u8]| {
        ready = true;
        cancel_clone.cancel();
    };
    let outcome = run_child(
        &mut child,
        &mut family,
        None,
        None,
        &cancel,
        &mut Stdout::new(&mut Out::Streamed(&mut on_stdout), false),
    )
    .await
    .unwrap();
    assert!(ready, "the child announced that it had started");
    assert!(matches!(outcome, ChildOutcome::Cancelled));
}

/// What a [`part_reader`] does once it has read the first of its input.
#[derive(Clone, Copy)]
enum Then {
    /// Reads nothing more until it is ended: a send of more than a pipe
    /// holds stays under way.
    Hold,
    /// Reads the rest, and on its end leaves `eof` in its folder.
    Drain,
}

/// Reads the first of its stdin, leaves `got` in its folder and says `got`
/// on stdout, then does what `then` says.
fn part_reader(then: Then) -> Command {
    #[cfg(windows)]
    {
        let first = "$in = [Console]::OpenStandardInput(); $b = New-Object byte[] 4096; \
                     [void]$in.Read($b, 0, 4096); New-Item got -ItemType File | Out-Null; \
                     [Console]::Out.WriteLine('got'); [Console]::Out.Flush()";
        let rest = match then {
            Then::Hold => "Start-Sleep -Seconds 600",
            Then::Drain => {
                "while ($in.Read($b, 0, 4096) -gt 0) {}; New-Item eof -ItemType File | Out-Null"
            }
        };
        let mut command = Command::new("powershell");
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &format!("{first}; {rest}"),
        ]);
        command
    }
    #[cfg(not(windows))]
    {
        let first = "dd bs=1 count=1 > /dev/null 2>&1; : > got; printf 'got\\n'";
        let rest = match then {
            Then::Hold => "exec sleep 600",
            Then::Drain => "cat > /dev/null; : > eof",
        };
        let mut command = Command::new("sh");
        command.args(["-c", &format!("{first}; {rest}")]);
        command
    }
}

/// A [`part_reader`] spawned the way a run is, in `dir`.
fn spawned_reader(
    then: Then,
    dir: &std::path::Path,
) -> (tokio::process::Child, super::super::family::Family) {
    let mut command = prepare(part_reader(then));
    command.stdin(Stdio::piped()).current_dir(dir);
    super::super::family::spawn(&mut command, true, None).unwrap()
}

/// Whether stdin was open each time a stopped run ended its child
/// (`child::ending`).
fn watch_the_end() -> Arc<Mutex<Vec<bool>>> {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let record = Arc::clone(&seen);
    super::super::child::mock_ending(mry::Any)
        .returns_with(move |open| record.lock().unwrap().push(open));
    seen
}

/// Past what a pipe holds, and past the 2 MiB Windows hands one write
/// thread (tokio `io::blocking`): the send's writes return only once the
/// child has read well past its first block.
const PAST_THE_HANDOFF: usize = 3 * 1024 * 1024;

/// A send that fails once part of the input has reached the child — the
/// writes are through, so the child has read past its first block and left
/// `got`; the flush fails — ends the child with its stdin still open, and
/// the send's failure is the run's answer. Open at the end, the child
/// cannot have read an end (the only writer is this side's), so it cannot
/// have taken what came before for the whole of its input.
#[tokio::test]
#[mry::lock(crate::process::child::finish, crate::process::child::ending)]
async fn a_send_failing_past_part_of_the_input_ends_the_child_with_stdin_open() {
    super::super::child::mock_finish()
        .returns_with(|| Err(std::io::Error::other("the pipe gave out")));
    let seen = watch_the_end();
    let dir = crate::temp::dir();
    let (mut child, mut family) = spawned_reader(Then::Drain, dir.path());
    let input = super::super::command::Input::from(vec![b'x'; PAST_THE_HANDOFF]);
    let outcome = crate::wait::bounded(
        "the run after a failed send",
        run_child(
            &mut child,
            &mut family,
            Some(&input),
            None,
            &CancellationToken::new(),
            &mut Stdout::new(&mut Out::Streamed(&mut |_| {}), false),
        ),
    )
    .await;
    match outcome {
        Err(failed) => assert_eq!(failed.to_string(), "the pipe gave out"),
        Ok(_) => panic!("the child answered by itself: it read an end"),
    }
    assert!(
        dir.path().join("got").exists(),
        "part of the input reached the child"
    );
    assert_eq!(
        *seen.lock().unwrap(),
        [true],
        "stdin open as the child was ended"
    );
    assert!(!dir.path().join("eof").exists(), "the child read no end");
}

/// A cancel once part of the input has reached the child (it says `got`,
/// and holds with the rest of the send under way) ends it with its stdin
/// still open.
#[tokio::test]
#[mry::lock(crate::process::child::ending)]
async fn a_cancel_past_part_of_the_input_ends_the_child_with_stdin_open() {
    let seen = watch_the_end();
    let dir = crate::temp::dir();
    let (mut child, mut family) = spawned_reader(Then::Hold, dir.path());
    let input = super::super::command::Input::from(vec![b'x'; PAST_THE_HANDOFF]);
    let cancel = CancellationToken::new();
    let stop = cancel.clone();
    let outcome = crate::wait::bounded(
        "the cancelled run",
        run_child(
            &mut child,
            &mut family,
            Some(&input),
            None,
            &cancel,
            &mut Stdout::new(
                &mut Out::Streamed(&mut |chunk| {
                    if chunk.windows(3).any(|w| w == b"got") {
                        stop.cancel();
                    }
                }),
                false,
            ),
        ),
    )
    .await
    .unwrap();
    assert!(matches!(outcome, ChildOutcome::Cancelled));
    assert!(
        dir.path().join("got").exists(),
        "part of the input reached the child"
    );
    assert_eq!(
        *seen.lock().unwrap(),
        [true],
        "stdin open as the child was ended"
    );
}

/// The deadline passing once part of the input has reached the child ends
/// it with its stdin still open. The clock is paused and moved by hand
/// only after the child said `got`; a blocking task held for the run keeps
/// the paused clock from moving by itself (tokio's test-util inhibits its
/// auto-advance while one runs). The hold lets go by itself after the
/// suite's budget, so a child that never speaks ends the run at its
/// deadline and the test fails by name.
#[tokio::test(start_paused = true)]
#[mry::lock(crate::process::child::ending)]
async fn a_deadline_past_part_of_the_input_ends_the_child_with_stdin_open() {
    let (release, held) = std::sync::mpsc::channel::<()>();
    let holder =
        tokio::task::spawn_blocking(move || held.recv_timeout(crate::wait::OVERALL_BUDGET).is_ok());
    let seen = watch_the_end();
    let dir = crate::temp::dir();
    let (mut child, mut family) = spawned_reader(Then::Hold, dir.path());
    let input = super::super::command::Input::from(vec![b'x'; PAST_THE_HANDOFF]);
    let budget = Duration::from_secs(60);
    let (said, heard) = tokio::sync::oneshot::channel();
    let mut said = Some(said);
    let mut on_stdout = |chunk: &[u8]| {
        if chunk.windows(3).any(|w| w == b"got")
            && let Some(said) = said.take()
        {
            said.send(()).unwrap();
        }
    };
    let cancel = CancellationToken::new();
    let mut out = Out::Streamed(&mut on_stdout);
    let mut on_stdout = Stdout::new(&mut out, false);
    let run = run_child(
        &mut child,
        &mut family,
        Some(&input),
        Some(budget),
        &cancel,
        &mut on_stdout,
    );
    tokio::pin!(run);
    tokio::select! {
        biased;
        got = heard => got.unwrap(),
        _ = &mut run => panic!("the run ended before the child read part of its input"),
    }
    tokio::time::advance(budget).await;
    let outcome = run.await.unwrap();
    release.send(()).unwrap();
    assert!(holder.await.unwrap(), "the clock's hold let go when told");
    assert!(matches!(outcome, ChildOutcome::TimedOut));
    assert!(
        dir.path().join("got").exists(),
        "part of the input reached the child"
    );
    assert_eq!(
        *seen.lock().unwrap(),
        [true],
        "stdin open as the child was ended"
    );
}

/// The width a pipe is asked to take is the width the kernel gives it.
#[cfg(target_os = "linux")]
#[test]
fn a_pipe_takes_the_width_it_is_asked_for() {
    use std::os::fd::AsFd;
    let (_reader, writer) = std::io::pipe().unwrap();
    let given =
        super::super::child::linux::set_pipe_size(writer.as_fd(), super::super::child::PIPE_WIDTH)
            .unwrap();
    assert_eq!(given, super::super::child::PIPE_WIDTH);
}

/// An input past the pipe's width has git's stdin asked to widen to it; an
/// input no larger keeps the pipe it was made with. The asks are counted
/// over the real call; what the kernel gives for one is the test above's.
#[cfg(target_os = "linux")]
#[tokio::test]
#[mry::lock(crate::process::child::linux::set_pipe_size)]
async fn an_input_past_the_width_widens_the_stdin_and_one_within_it_does_not() {
    use super::super::child::linux::mock_set_pipe_size;
    mock_set_pipe_size(mry::Any).calls_real_impl();
    let width = super::super::child::PIPE_WIDTH;
    for (size, asked_so_far) in [(width + 1, 1), (width, 1)] {
        let dir = crate::temp::dir();
        let (mut child, mut family) = spawned_reader(Then::Drain, dir.path());
        let input = super::super::command::Input::from(vec![b'x'; size]);
        let outcome = crate::wait::bounded(
            "the run",
            run_child(
                &mut child,
                &mut family,
                Some(&input),
                None,
                &CancellationToken::new(),
                &mut Stdout::new(&mut Out::Streamed(&mut |_| {}), false),
            ),
        )
        .await
        .unwrap();
        assert!(
            matches!(outcome, ChildOutcome::Finished { code: 0, .. }),
            "{size}"
        );
        mock_set_pipe_size(mry::Any).assert_called(asked_so_far);
    }
    mock_set_pipe_size(width).assert_called(1);
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
