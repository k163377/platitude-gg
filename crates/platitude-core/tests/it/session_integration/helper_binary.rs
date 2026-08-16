//! Publishing the todo-editor helper beside the running test binary.

// Everything below is unix-only: Windows neither reports `ETXTBSY` nor
// refuses to replace an entry whose file is being executed.
#[cfg(unix)]
use crate::support::session::publish_helper;

/// Runs a helper this suite published, retrying while the kernel answers
/// that somebody still holds the file open for writing (`ETXTBSY`).
///
/// `rename` keeps the inode, so what gets published is the very file
/// `fs::copy` had open for writing a moment earlier — and that write
/// reference can outlive the copy. `i_writecount` is counted per open
/// file description, and a thread that forks git mid-copy hands the child
/// a reference to the same one; `CLOEXEC` closes it, but not before the
/// child's `execve`. Until then the file cannot be executed at all. The
/// forking thread never sees its own window — `spawn` returns when the
/// child's `execve` closes the error pipe, so it is already past — but
/// with a thread per core the suite is forking git constantly and every
/// neighbour sees it. It belongs to another process's scheduling, and
/// this one cannot time it (実測: a child made to sleep 300ms between
/// fork and `execve` refuses a neighbour's exec for exactly that long).
///
/// Hence a retry on the error, not a wait for the window: every attempt
/// is the real run, and the first answer that is not "busy" is the answer
/// — a busy one at the end of the budget included, which reaches the
/// caller as the failure it is. (cargo and rustup carry the same loop for
/// the same reason, around the binaries they have just written.)
#[cfg(unix)]
fn run_published_helper(
    path: &std::path::Path,
    on_busy: impl FnOnce(),
) -> std::io::Result<std::process::Output> {
    // A second in all (40 × 25ms), which spans a fork→exec on a loaded
    // machine many times over, and is paid only while it really is busy.
    let mut retries = 40;
    let mut on_busy = Some(on_busy);
    loop {
        let answer = std::process::Command::new(path).output();
        let busy = matches!(
            &answer,
            Err(error) if error.kind() == std::io::ErrorKind::ExecutableFileBusy
        );
        if !busy || retries == 0 {
            return answer;
        }
        if let Some(notify) = on_busy.take() {
            notify();
        }
        retries -= 1;
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

/// The install must replace the helper's directory entry, never write
/// through it: a replay running the old one has it open for execution, and
/// on Linux that makes it unwritable (`ETXTBSY`) in one direction and
/// unexecutable in the other. A new inode under the same name settles both
/// — whoever is mid-exec keeps the file they started, and the next replay
/// gets the fresh one.
#[test]
#[cfg(unix)]
fn the_helper_is_replaced_rather_than_written_over() {
    use std::os::unix::fs::MetadataExt;

    let built = std::path::PathBuf::from(env!("CARGO_BIN_EXE_pg-todo-editor"));
    let dir = tempfile::tempdir().expect("tempdir");
    let first = publish_helper(&built, dir.path()).expect("install");
    let before = std::fs::metadata(&first).expect("stat").ino();

    let again = publish_helper(&built, dir.path()).expect("install over the first");
    assert_eq!(again, first, "the same name both times");
    assert_ne!(
        std::fs::metadata(&again).expect("stat").ino(),
        before,
        "the second install wrote through the live path"
    );

    // And what landed is still the helper: `fs::copy` carries the mode, so
    // the file it publishes is one git can execute.
    let out = run_published_helper(&again, || {}).expect("run the installed helper");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("usage: pg-todo-editor"),
        "the installed file is not the helper: {out:?}"
    );
}

/// The other half of that: publishing by `rename` settles who wins a race
/// between two copies, and settles nothing about a write handle already
/// open on the inode it publishes. Under load the suite hits that as a
/// forked git holding the copy's fd until its own `execve` — a window of
/// somebody else's making, too short to catch on purpose. Held open here
/// on purpose instead, since what the runner has to survive is the error,
/// not the fork: one attempt is refused outright, and the run that keeps
/// asking gets its answer as soon as the handle goes.
///
/// Linux rather than every unix, because POSIX only says `execve` *may*
/// refuse a file open for writing — this asserts that it does, which is
/// a promise Linux makes and the container is the machine that keeps it.
#[test]
#[cfg(target_os = "linux")]
fn a_helper_held_open_for_writing_is_run_once_the_handle_goes() {
    let built = std::path::PathBuf::from(env!("CARGO_BIN_EXE_pg-todo-editor"));
    let dir = tempfile::tempdir().expect("tempdir");
    let published = publish_helper(&built, dir.path()).expect("install");

    // Opened, not truncated: the file stays the helper throughout.
    let handle = std::fs::OpenOptions::new()
        .write(true)
        .open(&published)
        .expect("hold the published helper open for writing");
    let refused = std::process::Command::new(&published)
        .output()
        .expect_err("a file open for writing is not executable on linux");
    assert_eq!(refused.kind(), std::io::ErrorKind::ExecutableFileBusy);

    let (busy, saw_busy) = std::sync::mpsc::channel();
    let helper = published.clone();
    let runner = std::thread::spawn(move || {
        run_published_helper(&helper, move || busy.send(()).expect("report ETXTBSY"))
    });
    saw_busy.recv().expect("the first execution was refused");
    drop(handle);
    let out = runner
        .join()
        .expect("the helper runner")
        .expect("run the installed helper");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("usage: pg-todo-editor"),
        "the installed file is not the helper: {out:?}"
    );
}
