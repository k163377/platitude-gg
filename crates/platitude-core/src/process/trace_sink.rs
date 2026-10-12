//! Where a traced command's gits write their trace (`GitCommand::traced`):
//! a receiver of the command's own, read while they run — a named pipe on
//! Windows, an AF_UNIX stream socket elsewhere (trace2/tr2_dst.c, the
//! minimum to 2.55). Nothing of the record goes to disk: a file of it that
//! a git outliving the command (the fsmonitor daemon one of them started)
//! held open would stay behind on Windows and go on growing.
//!
//! Every git under the command opens a connection of its own — a hook's
//! and a replay's too, since the variable is inherited. git waits for the
//! gits it runs, so once the command's own git is reaped the record of
//! every git of the command is in; a git still running then outlives the
//! command and is not waited for. The receiver closes, and that git's next
//! write fails and ends its tracing (trace2 ignores `SIGPIPE` around the
//! write).

use std::ffi::{OsStr, OsString};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use tokio_util::sync::{CancellationToken, DropGuard};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// One connection's bytes, kept where the reader can be dropped without
/// losing what it read.
type Received = Arc<Mutex<Vec<u8>>>;

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A name of this process's own: its pid and count.
fn own_name() -> String {
    format!(
        "platitude-gg-trace-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

/// The receiver of one traced command's record. Dropped without
/// [`Self::finish`], it stops reading and closes.
pub(super) struct TraceSink {
    inner: imp::Sink,
    target: OsString,
    _closes: DropGuard,
}

impl TraceSink {
    /// A receiver of its own, or `None` where none can be made — the
    /// command then runs untraced, its refusals named by git's words alone.
    pub(super) fn open() -> Option<Self> {
        let closes = CancellationToken::new();
        match imp::Sink::open(closes.clone()) {
            Ok((inner, target)) => Some(Self {
                inner,
                target,
                _closes: closes.drop_guard(),
            }),
            Err(error) => {
                tracing::debug!(%error, "no trace receiver: the command runs untraced");
                None
            }
        }
    }

    /// What `GIT_TRACE2_EVENT` names.
    pub(super) fn target(&self) -> &OsStr {
        &self.target
    }

    /// Every connection's text, once the command's own git is reaped: each
    /// git that has ended read to its end; of one still running, what has
    /// come in by now, if any.
    pub(super) async fn finish(self) -> String {
        let bytes = self.inner.finish().await;
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

/// The connections' bytes one after another, each ending its last line.
fn joined(conns: impl IntoIterator<Item = Received>) -> Vec<u8> {
    let mut text = Vec::new();
    for received in conns {
        let received = lock(&received);
        if !received.is_empty() {
            text.extend_from_slice(&received);
            text.push(b'\n');
        }
    }
    text
}

#[cfg(windows)]
mod imp {
    use std::ffi::OsString;
    use std::io;
    use std::sync::{Arc, Mutex};

    use tokio::io::AsyncReadExt;
    use tokio::net::windows::named_pipe::{
        ClientOptions, NamedPipeServer, PipeMode, ServerOptions,
    };
    use tokio::task::JoinHandle;
    use tokio_util::sync::CancellationToken;

    use super::{Received, joined, lock, own_name};

    /// Instances kept waiting: a git opens the pipe twice, a look at the
    /// path and then the record (measured on 2.55), and the next git can
    /// follow at once — one that finds none waiting goes untraced.
    const WAITING: usize = 4;

    /// Every instance has a client (winerror.h).
    const ERROR_PIPE_BUSY: i32 = 231;

    pub(super) struct Sink {
        name: String,
        /// Ends the waiting instances.
        stop: CancellationToken,
        /// Ends every reader: the receiver dropped unfinished.
        closes: CancellationToken,
        acceptors: Vec<JoinHandle<Option<NamedPipeServer>>>,
        conns: Arc<Mutex<Vec<Conn>>>,
    }

    struct Conn {
        /// The git on the other end, where Windows tells it.
        pid: Option<u32>,
        received: Received,
        reader: JoinHandle<()>,
    }

    fn instance(name: &str, first: bool) -> io::Result<NamedPipeServer> {
        ServerOptions::new()
            // The first one fails where the name is someone else's already.
            .first_pipe_instance(first)
            .access_outbound(false)
            .reject_remote_clients(true)
            .pipe_mode(PipeMode::Byte)
            .create(name)
    }

    impl Sink {
        pub(super) fn open(closes: CancellationToken) -> io::Result<(Self, OsString)> {
            let name = format!(r"\\.\pipe\{}", own_name());
            let servers = (0..WAITING)
                .map(|i| instance(&name, i == 0))
                .collect::<io::Result<Vec<_>>>()?;
            // The receiver dropped unfinished stops them too.
            let stop = closes.child_token();
            let conns: Arc<Mutex<Vec<Conn>>> = Arc::default();
            let acceptors = servers
                .into_iter()
                .map(|server| {
                    tokio::spawn(accept(
                        name.clone(),
                        server,
                        stop.clone(),
                        closes.clone(),
                        Arc::clone(&conns),
                    ))
                })
                .collect();
            let target = OsString::from(&name);
            Ok((
                Self {
                    name,
                    stop,
                    closes,
                    acceptors,
                    conns,
                },
                target,
            ))
        }

        pub(super) async fn finish(self) -> Vec<u8> {
            self.stop.cancel();
            let mut waiting = Vec::new();
            for acceptor in self.acceptors {
                if let Ok(Some(server)) = acceptor.await {
                    waiting.push(server);
                }
            }
            // An instance still waiting has a git on it the acceptor has not
            // seen yet, or none: once each free one is taken here — until
            // they are all ours, or the pipe answers that every instance has
            // a client — every one left is a git's, and connects.
            let mut ours = Vec::new();
            let mut all_taken = true;
            while ours.len() < waiting.len() {
                match ClientOptions::new().read(false).open(&self.name) {
                    Ok(client) => ours.push(client),
                    Err(error) => {
                        all_taken = error.raw_os_error() == Some(ERROR_PIPE_BUSY);
                        break;
                    }
                }
            }
            if all_taken {
                for server in waiting {
                    let _ = server.connect().await;
                    take(server, &self.closes, &self.conns);
                }
            }
            let conns = std::mem::take(&mut *lock(&self.conns));
            let mut received = Vec::with_capacity(conns.len());
            for conn in conns {
                if conn.pid.is_some_and(win32::has_ended) {
                    // Gone, so its end of the pipe is closed: read to the
                    // end. (A child it spawned unrestricted, compat/mingw.c,
                    // would hold it open, and the command's output pipes
                    // with it, which the run waits on first.)
                    let _ = conn.reader.await;
                } else {
                    // Outlives the command (or is this process, taking a
                    // free instance): not waited for.
                    conn.reader.abort();
                    let _ = conn.reader.await;
                }
                received.push(conn.received);
            }
            drop(ours);
            joined(received)
        }
    }

    /// Waits on one instance, hands each git that connects to a reader and
    /// waits on a fresh one; at the stop, hands back the one it waits on.
    async fn accept(
        name: String,
        mut server: NamedPipeServer,
        stop: CancellationToken,
        closes: CancellationToken,
        conns: Arc<Mutex<Vec<Conn>>>,
    ) -> Option<NamedPipeServer> {
        loop {
            tokio::select! {
                biased;
                () = stop.cancelled() => return Some(server),
                // A git that wrote and left before the connect is read all
                // the same: what it sent is in the pipe.
                _ = server.connect() => {}
            }
            let next = (!stop.is_cancelled())
                .then(|| instance(&name, false).ok())
                .flatten();
            take(server, &closes, &conns);
            server = next?;
        }
    }

    fn take(server: NamedPipeServer, closes: &CancellationToken, conns: &Mutex<Vec<Conn>>) {
        let pid = win32::client_pid(&server);
        let received = Received::default();
        let reader = tokio::spawn(read(server, Arc::clone(&received), closes.clone()));
        lock(conns).push(Conn {
            pid,
            received,
            reader,
        });
    }

    async fn read(mut server: NamedPipeServer, received: Received, closes: CancellationToken) {
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let read = tokio::select! {
                biased;
                () = closes.cancelled() => return,
                read = server.read(&mut buf) => read,
            };
            match read {
                // The other end closed (a broken pipe is that too).
                Ok(0) | Err(_) => return,
                Ok(n) => lock(&received).extend_from_slice(&buf[..n]),
            }
        }
    }

    #[expect(
        unsafe_code,
        reason = "who is on the other end of a pipe, and whether it is still running, have no \
                  safe binding, and three signatures do not earn a dependency (family.rs says \
                  the same of job objects)"
    )]
    mod win32 {
        use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle};

        use tokio::net::windows::named_pipe::NamedPipeServer;

        const SYNCHRONIZE: u32 = 0x0010_0000;
        const WAIT_OBJECT_0: u32 = 0;
        const ERROR_INVALID_PARAMETER: i32 = 87;

        // SAFETY: the signatures are transcribed from the Win32 headers
        // (winbase.h, processthreadsapi.h, synchapi.h); the arguments are
        // handles, integers and one integer out.
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetNamedPipeClientProcessId(pipe: RawHandle, pid: *mut u32) -> i32;
            fn OpenProcess(access: u32, inherit: i32, pid: u32) -> RawHandle;
            fn WaitForSingleObject(handle: RawHandle, millis: u32) -> u32;
        }

        /// The process on the other end of `server`.
        pub(super) fn client_pid(server: &NamedPipeServer) -> Option<u32> {
            let mut pid = 0u32;
            // SAFETY: the pipe handle is live, borrowed from `server`; the
            // answer goes to a local.
            let told = unsafe { GetNamedPipeClientProcessId(server.as_raw_handle(), &mut pid) };
            (told != 0).then_some(pid)
        }

        /// Whether process `pid` has ended. One that cannot be opened for
        /// another reason than being gone counts as running.
        pub(super) fn has_ended(pid: u32) -> bool {
            // SAFETY: integer arguments; a null answer is checked.
            let raw = unsafe { OpenProcess(SYNCHRONIZE, 0, pid) };
            if raw.is_null() {
                return std::io::Error::last_os_error().raw_os_error()
                    == Some(ERROR_INVALID_PARAMETER);
            }
            // SAFETY: a handle just opened, owned by nothing else.
            let process = unsafe { OwnedHandle::from_raw_handle(raw) };
            // SAFETY: a live handle opened with SYNCHRONIZE.
            unsafe { WaitForSingleObject(process.as_raw_handle(), 0) == WAIT_OBJECT_0 }
        }
    }
}

#[cfg(unix)]
mod imp {
    use std::ffi::OsString;
    use std::io::{self, Read};
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    use tokio::io::AsyncReadExt;
    use tokio::net::{UnixListener, UnixStream};
    use tokio::task::JoinHandle;
    use tokio_util::sync::CancellationToken;

    use super::{Received, joined, lock, own_name};

    /// `sun_path` holds 104 bytes on macOS (108 on Linux), its NUL among
    /// them; git refuses a path that does not fit (tr2_dst.c).
    const SUN_PATH: usize = 104;

    pub(super) struct Sink {
        /// The socket's entry, removed with the receiver.
        _entry: Entry,
        stop: CancellationToken,
        acceptor: JoinHandle<Option<UnixListener>>,
        conns: Arc<Mutex<Vec<Conn>>>,
    }

    struct Conn {
        received: Received,
        /// Hands the stream back at the stop, still open, to be drained.
        reader: JoinHandle<Option<UnixStream>>,
    }

    /// The socket's name on disk — an entry with nothing in it, gone when
    /// dropped.
    struct Entry(PathBuf);

    impl Drop for Entry {
        fn drop(&mut self) {
            if let Err(error) = std::fs::remove_file(&self.0)
                && error.kind() != io::ErrorKind::NotFound
            {
                tracing::debug!(path = %self.0.display(), %error, "trace socket not removed");
            }
        }
    }

    /// The person's own folder for runtime sockets: `$XDG_RUNTIME_DIR`
    /// where it is set, the temporary folder (the person's own on macOS)
    /// otherwise — both short enough for a socket's path.
    fn place() -> PathBuf {
        std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .filter(|dir| dir.is_absolute() && dir.is_dir())
            .unwrap_or_else(std::env::temp_dir)
    }

    impl Sink {
        pub(super) fn open(closes: CancellationToken) -> io::Result<(Self, OsString)> {
            let path = place().join(own_name());
            if path.as_os_str().len() >= SUN_PATH {
                return Err(io::Error::other(format!(
                    "{} is too long for a socket",
                    path.display()
                )));
            }
            // The name is this process's own, so one already there was left
            // by a process gone before this one took its pid.
            remove_left(&path)?;
            let listener = std::os::unix::net::UnixListener::bind(&path)?;
            let entry = Entry(path.clone());
            listener.set_nonblocking(true)?;
            let listener = UnixListener::from_std(listener)?;
            // The receiver dropped unfinished stops as a finish does.
            let stop = closes;
            let conns: Arc<Mutex<Vec<Conn>>> = Arc::default();
            let acceptor = tokio::spawn(accept(listener, stop.clone(), Arc::clone(&conns)));
            let mut target = OsString::from("af_unix:stream:");
            target.push(&path);
            Ok((
                Self {
                    _entry: entry,
                    stop,
                    acceptor,
                    conns,
                },
                target,
            ))
        }

        pub(super) async fn finish(self) -> Vec<u8> {
            self.stop.cancel();
            let mut received = Vec::new();
            // Connections not accepted yet: a git that ended has its whole
            // record queued behind one.
            if let Ok(Some(listener)) = self.acceptor.await
                && let Ok(listener) = listener.into_std()
            {
                while let Ok((stream, _)) = listener.accept() {
                    let into = Received::default();
                    drain(stream, &into);
                    received.push(into);
                }
            }
            let conns = std::mem::take(&mut *lock(&self.conns));
            for conn in conns {
                if let Ok(Some(stream)) = conn.reader.await
                    && let Ok(stream) = stream.into_std()
                {
                    drain(stream, &conn.received);
                }
                received.push(conn.received);
            }
            joined(received)
        }
    }

    fn remove_left(path: &Path) -> io::Result<()> {
        match std::fs::remove_file(path) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        }
    }

    async fn accept(
        listener: UnixListener,
        stop: CancellationToken,
        conns: Arc<Mutex<Vec<Conn>>>,
    ) -> Option<UnixListener> {
        loop {
            let accepted = tokio::select! {
                biased;
                () = stop.cancelled() => return Some(listener),
                accepted = listener.accept() => accepted,
            };
            let Ok((stream, _)) = accepted else {
                // Closed: a later git's connection is refused and it runs
                // untraced, where one queued could wait for room (Linux).
                return None;
            };
            let received = Received::default();
            let reader = tokio::spawn(read(stream, Arc::clone(&received), stop.clone()));
            lock(&conns).push(Conn { received, reader });
        }
    }

    async fn read(
        mut stream: UnixStream,
        received: Received,
        stop: CancellationToken,
    ) -> Option<UnixStream> {
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let read = tokio::select! {
                biased;
                () = stop.cancelled() => return Some(stream),
                read = stream.read(&mut buf) => read,
            };
            match read {
                Ok(0) | Err(_) => return None,
                Ok(n) => lock(&received).extend_from_slice(&buf[..n]),
            }
        }
    }

    /// Reads what is queued on `stream` now, without waiting: all of a git
    /// that has ended, up to its end; what one still running has sent.
    fn drain(mut stream: std::os::unix::net::UnixStream, into: &Received) {
        if stream.set_nonblocking(true).is_err() {
            return;
        }
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            match stream.read(&mut buf) {
                Ok(0) => return,
                Ok(n) => lock(into).extend_from_slice(&buf[..n]),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(_) => return,
            }
        }
    }
}

#[cfg(test)]
#[path = "trace_sink_tests.rs"]
mod tests;
