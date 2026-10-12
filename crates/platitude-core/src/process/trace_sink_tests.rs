//! The receiver against real gits: ones that end, ones that came and went
//! before it looked, and one that outlives the command, as the fsmonitor
//! daemon does; and a receiver its command never used.

use std::path::Path;
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use super::TraceSink;
use crate::wait::bounded;

/// git in `dir`, its record going to `sink`, with no configuration from
/// outside (rules/core.md §非同期・並行テストの実装方針) and no repository
/// above `dir`. Run by itself, it holds the test's thread, so the
/// receiver's tasks do not run until it has ended.
fn blocking(sink: &TraceSink, dir: &Path, args: &[&str]) -> std::process::Command {
    let nowhere = dir.join("no-config");
    let mut git = std::process::Command::new("git");
    git.args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", nowhere.join("global"))
        .env("XDG_CONFIG_HOME", &nowhere)
        .env("GIT_CEILING_DIRECTORIES", dir)
        .env("GIT_TRACE2_EVENT", sink.target())
        .env("GIT_TRACE2_EVENT_BRIEF", "false")
        .env_remove("GIT_TRACE2_PARENT_SID")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    git
}

/// [`blocking`], awaited.
fn traced(sink: &TraceSink, dir: &Path, args: &[&str]) -> tokio::process::Command {
    let mut git = tokio::process::Command::from(blocking(sink, dir, args));
    git.kill_on_drop(true);
    git
}

/// How many processes the text has the last event of — each one read to
/// its end.
fn ended(text: &str) -> usize {
    text.lines()
        .filter(|line| line.contains("\"event\":\"atexit\""))
        .count()
}

#[tokio::test]
async fn every_git_that_ended_is_read_to_its_end() {
    let dir = crate::temp::dir();
    let sink = TraceSink::open().expect("a receiver");
    for _ in 0..3 {
        let status = traced(&sink, dir.path(), &["version"])
            .status()
            .await
            .expect("spawn git");
        assert!(status.success());
    }
    let text = bounded("the record", sink.finish()).await;
    assert_eq!(ended(&text), 3, "{text}");
}

/// Gits that came and went before the receiver looked: their record waits
/// on an instance no acceptor has seen (Windows) or in the socket's queue,
/// and is read all the same. Two, as a git opens the pipe twice on Windows
/// and two take every instance kept waiting.
#[tokio::test(flavor = "current_thread")]
async fn gits_the_receiver_has_not_seen_are_read_all_the_same() {
    let dir = crate::temp::dir();
    let sink = TraceSink::open().expect("a receiver");
    for _ in 0..2 {
        let status = blocking(&sink, dir.path(), &["version"])
            .status()
            .expect("spawn git");
        assert!(status.success());
    }
    let text = bounded("the record", sink.finish()).await;
    assert_eq!(ended(&text), 2, "{text}");
}

/// A git still running when the command's own has ended is not waited for,
/// and runs on with nowhere to write: it answers, and ends well.
#[tokio::test]
async fn a_git_that_outlives_the_command_is_not_waited_for_and_runs_on() {
    let dir = crate::temp::dir();
    let sink = TraceSink::open().expect("a receiver");
    let status = traced(&sink, dir.path(), &["init", "-q", "repo"])
        .status()
        .await
        .expect("spawn git");
    assert!(status.success());
    let repo = dir.path().join("repo");
    let mut survivor = traced(&sink, &repo, &["cat-file", "--batch-check"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn git");
    let mut ask = survivor.stdin.take().expect("its input");
    let mut answers = BufReader::new(survivor.stdout.take().expect("its output")).lines();
    // An answer: it is running, and has said its start long before.
    ask.write_all(b"HEAD\n").await.expect("ask");
    let answer = bounded("an answer", answers.next_line()).await;
    assert_eq!(answer.expect("read").as_deref(), Some("HEAD missing"));

    let text = bounded("the record", sink.finish()).await;
    assert_eq!(ended(&text), 1, "only `git init` has ended: {text}");

    ask.write_all(b"HEAD\n").await.expect("ask again");
    let answer = bounded("an answer", answers.next_line()).await;
    assert_eq!(answer.expect("read").as_deref(), Some("HEAD missing"));
    drop(ask);
    let status = bounded("its end", survivor.wait()).await.expect("wait");
    assert!(status.success(), "{status}");
}

/// Nothing is left where the socket stood.
#[cfg(unix)]
#[tokio::test]
async fn the_socket_goes_with_the_receiver() {
    let sink = TraceSink::open().expect("a receiver");
    let target = sink.target().to_string_lossy().into_owned();
    let path = std::path::PathBuf::from(
        target
            .strip_prefix("af_unix:stream:")
            .expect("a stream socket"),
    );
    assert!(path.exists());
    bounded("the record", sink.finish()).await;
    assert!(!path.exists(), "{}", path.display());

    let dropped = TraceSink::open().expect("another");
    let target = dropped.target().to_string_lossy().into_owned();
    drop(dropped);
    let path = target
        .strip_prefix("af_unix:stream:")
        .expect("a stream socket");
    assert!(!Path::new(path).exists(), "{path}");
}

/// A receiver dropped unfinished — its command never ran — closes every
/// instance it kept waiting: the name is gone. Nothing says when its tasks
/// have seen the drop, so the name is asked after until it is.
#[cfg(windows)]
#[tokio::test]
async fn the_pipe_goes_with_a_receiver_dropped_unfinished() {
    use tokio::net::windows::named_pipe::ClientOptions;
    const ERROR_FILE_NOT_FOUND: i32 = 2;

    let sink = TraceSink::open().expect("a receiver");
    let name = sink.target().to_os_string();
    drop(sink);
    bounded("the pipe gone", async {
        while ClientOptions::new()
            .read(false)
            .open(&name)
            .map_or_else(|e| e.raw_os_error(), |_| None)
            != Some(ERROR_FILE_NOT_FOUND)
        {
            // waits(paced): each ask is the answer itself and ends the loop; the sleep only spaces the asks
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await;
}
