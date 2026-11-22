use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// What starts the sides: run by themselves, as `--ignored` runs them, they
/// do nothing.
const SIDES: &str = "INHERITED_SIDES";

/// This test binary, to run the side `name` alone.
fn side(name: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().expect("this test binary"));
    command
        .args([
            &format!("inherited::tests::{name}"),
            "--exact",
            "--ignored",
            "--nocapture",
        ])
        .env(SIDES, "1");
    command
}

/// Lives exactly as long as its stdin is open.
#[test]
#[ignore = "a process started by starting_side"]
fn holding_side() {
    if std::env::var_os(SIDES).is_none() {
        return;
    }
    let mut rest = Vec::new();
    std::io::stdin()
        .read_to_end(&mut rest)
        .expect("stdin reads to its end");
}

/// Started on a launcher's pipe, as a run under an IDE is: keeps what it
/// was started with from its children, starts [`holding_side`] on its own
/// stdin and on nothing else, and ends without waiting for it.
#[test]
#[ignore = "the started side of a_started_process_holds_none_of_what_its_starter_was_started_with"]
#[expect(
    clippy::zombie_processes,
    reason = "outliving its starter is the holding side's part; it ends when the parent test \
              closes its stdin"
)]
fn starting_side() {
    if std::env::var_os(SIDES).is_none() {
        return;
    }
    super::keep_from_children().expect("the handles are kept from the children");
    side("holding_side")
        .stdin(Stdio::inherit())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the holding side starts");
}

/// The pipe a launcher reads a run's output from ends when the run ends,
/// though a process the run started lives on: an IDE's run ends with the
/// window, whatever the window left running.
#[test]
fn a_started_process_holds_none_of_what_its_starter_was_started_with() {
    let (mut output, run_console) = std::io::pipe().expect("a pipe for the run's output");
    // The command goes at the end of the statement, and this side's copy of
    // the pipe's write end with it.
    let mut starting = side("starting_side")
        .stdin(Stdio::piped())
        .stdout(run_console)
        .stderr(Stdio::null())
        .spawn()
        .expect("the starting side starts");
    let holding = starting.stdin.take().expect("the holding side's stdin");
    let started = starting.wait().expect("the starting side ends");
    assert!(
        started.success(),
        "the starting side kept and started: {started}"
    );
    let (said, heard) = mpsc::channel();
    std::thread::spawn(move || {
        let mut all = Vec::new();
        let read = output.read_to_end(&mut all).map(|_| all);
        let _ = said.send(read);
    });
    // waits(ceiling): with the starting side gone, the end comes at once unless the holding side
    // holds the pipe, and then not before `holding` goes below — the ceiling names that failure
    let read = heard.recv_timeout(Duration::from_secs(900));
    drop(holding);
    let all = read
        .expect("the run's output ended while the holding side lived")
        .expect("the run's output reads");
    assert!(
        String::from_utf8_lossy(&all).contains("starting_side"),
        "the output is the starting side's: {}",
        String::from_utf8_lossy(&all)
    );
}
