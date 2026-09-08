//! Running a file the suite has just written, on a kernel that refuses to
//! execute one anybody still holds open for writing.

/// Runs `command`, retrying while the kernel answers that somebody still
/// holds the image open for writing (`ETXTBSY`).
///
/// **The window belongs to another process.** `i_writecount` is counted
/// per open file description, and a thread that forks git while this end
/// is writing a file hands the child a reference to the same one;
/// `CLOEXEC` closes it, but not before the child's `execve`. Until then
/// the file cannot be executed at all — and `rename` does not settle it
/// either, since it keeps the inode the write had open. The forking
/// thread never sees its own window (`spawn` returns when the child's
/// `execve` closes the error pipe, so it is already past), but with a
/// thread per core the suite is forking git constantly and every
/// neighbour sees it. Nothing here can time it: it is another process's
/// scheduling (measured — a child made to sleep 300ms between fork and
/// `execve` refuses a neighbour's exec for exactly that long).
///
/// Hence a retry on the error, not a wait for the window: every attempt
/// is the real run, and the first answer that is not "busy" is the answer
/// — a busy one at the end of the budget included, which reaches the
/// caller as the failure it is. (cargo and rustup carry the same loop for
/// the same reason, around the binaries they have just written.)
///
/// `on_busy` is called once, the first time an attempt is refused, for
/// the tests that hold a handle open on purpose and have to know the loop
/// reached them before they let it go.
#[cfg(unix)]
pub fn run_once_it_is_not_busy(
    command: &mut std::process::Command,
    on_busy: impl FnOnce(),
) -> std::io::Result<std::process::Output> {
    // Paid only while it really is busy. The ceiling is the suite's
    // failure-detection backstop (`QUIET_BUDGET`), not a guess at the
    // window: the window belongs to another process's scheduling, and a
    // fixed second of retries is a wall-clock verdict a loaded machine
    // can outlast (.claude/rules/core.md: a ceiling is for detecting
    // failure, never for deciding it).
    // waits(ceiling): the suite's quiet budget, spent only on a kernel that keeps calling the image busy
    let started = std::time::Instant::now();
    let mut on_busy = Some(on_busy);
    loop {
        let answer = command.output();
        let busy = matches!(
            &answer,
            Err(error) if error.kind() == std::io::ErrorKind::ExecutableFileBusy
        );
        if !busy || started.elapsed() >= super::wait::QUIET_BUDGET {
            return answer;
        }
        if let Some(notify) = on_busy.take() {
            notify();
        }
        // waits(paced): every attempt is the real run and its answer ends the loop; the sleep only spaces the attempts
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}
