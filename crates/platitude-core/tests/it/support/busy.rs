//! Running a file the suite has just written, on a kernel that refuses to
//! execute one anybody still holds open for writing.

/// Runs `command`, retrying while the kernel answers that somebody still
/// holds the image open for writing (`ETXTBSY`).
///
/// The window belongs to another process: a thread that forks git while
/// this end is writing the file hands the child the same open file
/// description until the child's `execve` (`CLOEXEC`), and `rename` keeps
/// the inode, so nothing here can time it. Every attempt is the real run;
/// the first answer that is not "busy" is the answer, and a busy one at
/// the end of the budget reaches the caller as the failure it is.
///
/// `on_busy` is called once, the first time an attempt is refused, for
/// the tests that hold a handle open on purpose and have to know the loop
/// reached them before they let it go.
#[cfg(unix)]
pub fn run_once_it_is_not_busy(
    command: &mut std::process::Command,
    on_busy: impl FnOnce(),
) -> std::io::Result<std::process::Output> {
    // The ceiling is the suite's backstop (`QUIET_BUDGET`), not a short
    // retry budget a loaded machine can outlast (core.md「待ちの上限は失敗検出の backstop」).
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
