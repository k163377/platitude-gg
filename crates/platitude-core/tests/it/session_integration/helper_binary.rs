//! Publishing the todo-editor helper beside the running test binary.

// Everything below is unix-only: Windows neither reports `ETXTBSY` nor
// refuses to replace an entry whose file is being executed.
#[cfg(unix)]
use crate::support::session::publish_helper;

/// Runs a helper this suite published, waiting out a neighbouring fork's
/// write handle on it (`support::busy::run_once_it_is_not_busy`).
#[cfg(unix)]
fn run_published_helper(
    path: &std::path::Path,
    on_busy: impl FnOnce(),
) -> std::io::Result<std::process::Output> {
    crate::support::busy::run_once_it_is_not_busy(&mut std::process::Command::new(path), on_busy)
}

/// The install must replace the helper's directory entry: on Linux a file
/// being executed is unwritable (`ETXTBSY`), and one open for writing is
/// unexecutable. A new inode under the same name settles both.
#[test]
#[cfg(unix)]
fn the_helper_is_replaced_rather_than_written_over() {
    use std::os::unix::fs::MetadataExt;

    let built = std::path::PathBuf::from(env!("CARGO_BIN_EXE_pgg-todo-editor"));
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

    // `fs::copy` carries the mode, so what landed is executable.
    let out = run_published_helper(&again, || {}).expect("run the installed helper");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("usage: pgg-todo-editor"),
        "the installed file is not the helper: {out:?}"
    );
}

/// Publishing by `rename` settles nothing about a write handle already
/// open on the published inode — under load, a forked git holding the
/// copy's fd until its own `execve`. Held open here on purpose: one attempt
/// is refused, and the runner that keeps asking succeeds once it goes.
///
/// Linux only: POSIX says `execve` *may* refuse a file open for writing,
/// and Linux promises it does.
#[test]
#[cfg(target_os = "linux")]
fn a_helper_held_open_for_writing_is_run_once_the_handle_goes() {
    let built = std::path::PathBuf::from(env!("CARGO_BIN_EXE_pgg-todo-editor"));
    let dir = tempfile::tempdir().expect("tempdir");
    let published = publish_helper(&built, dir.path()).expect("install");

    // Opened: the file stays the helper throughout.
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
        String::from_utf8_lossy(&out.stderr).contains("usage: pgg-todo-editor"),
        "the installed file is not the helper: {out:?}"
    );
}
