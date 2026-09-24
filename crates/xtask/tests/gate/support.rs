//! The throwaway repository every test gates in: a miniature of the
//! workspace, its seat, and the runner run against it.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::wait::{Budget, LOOK_AGAIN, Wait};

pub const EXE: &str = env!("CARGO_BIN_EXE_xtask");
static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Where the suites' trees stand under the system temp — `xtask`'s own
/// `yard::BASE`, spelled again because an integration binary is a crate
/// of its own and cannot read the runner's modules. **The two have to
/// say the same thing**: the name is what the day-old sweep looks under
/// (`verify::ownership::RUN_BASES`), and a root outside it is one
/// nothing ever takes away. A sandbox whose removal is refused — git
/// still holding the tree, the process killed before the `Drop` — is
/// what this is for; the ordinary road removes its own.
const YARD: &str = "pgg-tests";

/// The steps every gate runs regardless of the diff.
pub const ALWAYS: [&str; 5] = ["structure", "waits", "docs", "verbs", "fmt"];

/// Runs a command, retrying while the kernel answers that somebody still
/// holds its image open for writing (`ETXTBSY`).
///
/// A test publishing a runner into a seat's build slot has that file open
/// for writing, and `i_writecount` is counted per open file description:
/// a neighbour test forking git or xtask mid-copy hands its child a
/// reference to the same one. `CLOEXEC` closes it, but not before the
/// child's `execve`, and until then the file cannot be executed at all.
/// The copying thread never sees its own window — `spawn` returns once
/// the child's `execve` closes the error pipe — but with a thread per
/// core the suite is forking constantly and every neighbour sees it.
///
/// Hence a retry on the error: every
/// attempt is the real run, and the first answer that is not "busy" is
/// the answer. The window belongs to another process's scheduling, so the
/// retries run under the suite's budget, and an image still busy at
/// the end of it reaches the caller as the failure it is — a defect.
/// (`run_published_helper` in
/// platitude-core's suite carries the same loop over the todo helper.)
pub fn output_past_a_busy_image(
    command: &mut Command,
    on_busy: impl FnOnce(),
) -> std::io::Result<std::process::Output> {
    let mut wait = Wait::new("the runner's image", Budget::SUITE, LOOK_AGAIN);
    let mut on_busy = Some(on_busy);
    loop {
        let answer = command.output();
        let busy = match &answer {
            Err(error) if error.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                error.to_string()
            }
            _ => return answer,
        };
        if let Some(notify) = on_busy.take() {
            notify();
        }
        wait.saw(busy);
        if let Err(expired) = wait.look_again("the image to come free") {
            return Err(std::io::Error::new(
                std::io::ErrorKind::ExecutableFileBusy,
                expired.to_string(),
            ));
        }
    }
}

pub struct Sandbox {
    pub root: PathBuf,
    gitconfig: PathBuf,
    fake_log: PathBuf,
    /// The primary checkout, on main.
    pub repo: PathBuf,
    /// A worktree on `worktree-a`, the seat under test.
    pub seat: PathBuf,
}

impl Sandbox {
    pub fn new(name: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir()
            .join(YARD)
            .join(format!("gate-{name}-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&root).expect("temp root");
        let gitconfig = root.join("gitconfig");
        std::fs::write(
            &gitconfig,
            "[user]\n\tname = Gate Test\n\temail = gate@example.com\n\
             [init]\n\tdefaultBranch = main\n[commit]\n\tgpgsign = false\n\
             [core]\n\tautocrlf = false\n",
        )
        .expect("gitconfig");
        let sandbox = Self {
            repo: root.join("repo"),
            seat: root.join("seat-a"),
            fake_log: root.join("ran.txt"),
            root,
            gitconfig,
        };
        sandbox.seed();
        sandbox
    }

    pub fn env(&self, command: &mut Command) {
        command
            .env("GIT_CONFIG_GLOBAL", &self.gitconfig)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("LC_ALL", "C")
            .env("PGG_GATE_FAKE_LOG", &self.fake_log)
            .env_remove("PGG_GATE_FAKE_FAIL")
            .env_remove("PGG_GATE_FAKE_FAIL_AFTER")
            .env_remove("PGG_GATE_FAKE_HOLD")
            .env_remove("PGG_GATE_SKIP")
            // This suite runs as a step of a gate, which marks every
            // child of a step as running under its ticket. That mark
            // would reach the runners started here — and a budget that
            // hands out passes is not the one under test. Spelled out,
            // as CLAUDECODE is: this crate cannot use the runner's
            // modules, and the runner's own suite checks the two
            // spellings agree (`budget::tests`).
            .env_remove("PGG_BUDGET_HELD")
            // The gate answers for a session's git and nobody else's, so
            // the sandbox's git is a session's — whether or not the run
            // that started these tests was one (CI's is not, a session's
            // own `cargo test` is).
            .env("CLAUDECODE", "1");
    }

    pub fn git(&self, dir: &Path, args: &[&str], extra: &[(&str, &str)]) -> Result<String, String> {
        let mut command = Command::new("git");
        command.arg("-C").arg(dir).args(args);
        self.env(&mut command);
        for (key, value) in extra {
            command.env(key, value);
        }
        let output = command.output().expect("spawn git");
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if output.status.success() {
            Ok(stdout)
        } else {
            Err(format!("{stdout}\n{stderr}"))
        }
    }

    pub fn git_ok(&self, dir: &Path, args: &[&str]) -> String {
        self.git(dir, args, &[])
            .unwrap_or_else(|e| panic!("git {} in {}: {e}", args.join(" "), dir.display()))
    }

    /// Runs `cargo xtask gate …` against `dir`; answers (success, output).
    pub fn gate(&self, dir: &Path, args: &[&str], extra: &[(&str, &str)]) -> (bool, String) {
        let mut command = Command::new(EXE);
        command.arg("gate").args(args).arg("--dir").arg(dir);
        self.env(&mut command);
        for (key, value) in extra {
            command.env(key, value);
        }
        let output = output_past_a_busy_image(&mut command, || {}).expect("spawn xtask");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        (output.status.success(), text)
    }

    pub fn gate_ok(&self, dir: &Path, args: &[&str]) -> String {
        let (ok, text) = self.gate(dir, args, &[]);
        assert!(ok, "gate {} failed:\n{text}", args.join(" "));
        text
    }

    /// `cargo xtask land --branch <b>` run from the primary; answers
    /// (success, output).
    pub fn land(&self, branch: &str) -> (bool, String) {
        self.land_from(Path::new(EXE), branch)
    }

    /// The same landing under a session's own mark, for the claim on the
    /// landed seat: whose it is decides whether it is handed back. The
    /// marks are set explicitly — the run that started these tests may
    /// be a session itself, and its id would otherwise be the answer.
    pub fn land_as(&self, branch: &str, session: &str) -> (bool, String) {
        self.landing(Path::new(EXE), branch, Some(session))
    }

    /// The same landing run from a given binary — a copy in a tree's own
    /// build slot, for the landing that has to write that slot.
    pub fn land_from(&self, exe: &Path, branch: &str) -> (bool, String) {
        self.landing(exe, branch, None)
    }

    fn landing(&self, exe: &Path, branch: &str, session: Option<&str>) -> (bool, String) {
        let mut command = Command::new(exe);
        command
            .args(["land", branch, "--dir"])
            .arg(&self.repo)
            .current_dir(&self.repo);
        self.env(&mut command);
        if let Some(session) = session {
            command.env("CLAUDE_CODE_SESSION_ID", session);
            command.env_remove("CLAUDE_PID");
        }
        let output = output_past_a_busy_image(&mut command, || {}).expect("spawn xtask");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        (output.status.success(), text)
    }

    /// The steps run since the last look, and the look empties the log.
    pub fn ran(&self) -> BTreeSet<String> {
        let text = std::fs::read_to_string(&self.fake_log).unwrap_or_default();
        let _ = std::fs::remove_file(&self.fake_log);
        text.lines().map(str::to_string).collect()
    }

    /// The ledger the newest gate run left in this tree, as `(side, id,
    /// outcome)` — the table a reader does arithmetic over
    /// (`gate::record::ledger`).
    ///
    /// **Newest by the clock, not by the name.** A run is filed under
    /// `<epoch>-<pid>`, and two gates of one test land in the same
    /// second, so the pid decides — as text, where `10236` sorts before
    /// `9999`. Sorted by name, which run a test reads then turns on the
    /// pids the machine happened to hand out (observed: the second gate
    /// of `a_run_that_ran_nothing_still_files_a_row_for_every_step` read
    /// the first gate's table and saw its verbs as `ran`).
    pub fn ledger(&self, dir: &Path) -> Vec<(String, String, String)> {
        let runs = dir.join("target").join("gate-runs");
        let mut names: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(&runs)
            .unwrap_or_else(|e| panic!("{}: {e}", runs.display()))
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.to_string_lossy().ends_with(".units.tsv"))
            .map(|path| {
                let wrote = path
                    .metadata()
                    .and_then(|meta| meta.modified())
                    .unwrap_or(std::time::UNIX_EPOCH);
                (wrote, path)
            })
            .collect();
        names.sort();
        let (_, newest) = names.last().expect("a ledger beside the record");
        std::fs::read_to_string(newest)
            .expect("the ledger")
            .lines()
            .skip(1)
            .map(|line| {
                let mut fields = line.split('\t');
                (
                    fields.next().unwrap_or_default().to_string(),
                    fields.next().unwrap_or_default().to_string(),
                    fields.next().unwrap_or_default().to_string(),
                )
            })
            .collect()
    }

    /// The steps that were told to reuse a release
    /// (`--no-build`), since the last look. Kept beside the fake log
    /// in its own file, because every other test reads that as a set
    /// of step ids.
    pub fn told_not_to_build(&self) -> BTreeSet<String> {
        let path = PathBuf::from(format!("{}.no-build", self.fake_log.display()));
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let _ = std::fs::remove_file(&path);
        text.lines().map(str::to_string).collect()
    }

    /// What a hook prints for `payload`, through the binary the harness
    /// runs. Empty is a hook's way of saying it has no objection.
    pub fn hook(&self, event: &str, payload: &str) -> String {
        let file = self.root.join(format!(
            "{event}-{}.json",
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::write(&file, payload).expect("payload");
        let mut command = Command::new(EXE);
        command
            .args(["hook", event])
            .stdin(std::process::Stdio::from(
                std::fs::File::open(&file).expect("payload open"),
            ));
        command.current_dir(&self.repo);
        self.env(&mut command);
        // The payload's id is the session's; the environment's, if the
        // run that started this suite is a session, is not.
        command.env_remove("CLAUDE_CODE_SESSION_ID");
        command.env_remove("CLAUDE_PID");
        let output = output_past_a_busy_image(&mut command, || {}).expect("spawn xtask");
        assert!(
            output.status.success(),
            "hook {event} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    pub fn write(&self, dir: &Path, relative: &str, text: &str) {
        let path = dir.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("parent");
        }
        std::fs::write(&path, text).expect("write");
    }

    /// The core's leaf module `refs` changed to a body nobody else has,
    /// with a unit test of its own: the smallest change that owes the
    /// core's unit tests and nothing beside them. A distinct `n` per
    /// test keeps two tests' commits from being one object.
    pub fn write_refs(&self, dir: &Path, n: u32) {
        self.write(
            dir,
            "crates/platitude-core/src/refs.rs",
            &format!(
                "pub fn refs() {{ let _ = {n}; }}\n#[cfg(test)]\nmod tests {{\n    #[test]\n    \
                 fn t() {{}}\n}}\n"
            ),
        );
    }

    pub fn commit_all(&self, dir: &Path, message: &str, extra: &[(&str, &str)]) -> String {
        self.git_ok(dir, &["add", "-A"]);
        self.git(dir, &["commit", "-q", "-m", message], extra)
            .unwrap_or_else(|e| panic!("commit {message:?}: {e}"));
        self.git_ok(dir, &["rev-parse", "HEAD"])
    }

    pub fn head(&self, dir: &Path) -> String {
        self.git_ok(dir, &["rev-parse", "HEAD"])
    }

    pub fn main_sha(&self) -> String {
        self.git_ok(&self.repo, &["rev-parse", "main"])
    }

    /// A miniature of the workspace: two core modules with tests, an
    /// integration binary over them, an app model naming one of them,
    /// two product components and a harness driver, an xtask module,
    /// and a census that knows one verb. One commit on main, the hook
    /// installed, a seat worktree.
    fn seed(&self) {
        std::fs::create_dir_all(&self.repo).expect("repo dir");
        self.git_ok(&self.repo, &["init", "-q", "-b", "main"]);
        for (path, text) in [
            ("Cargo.toml", "[workspace]\n"),
            ("Cargo.lock", "# lock\n"),
            // As the workspace ignores it: a landing steps out of the
            // build slot under here, and what it leaves there is nobody's
            // uncommitted change.
            (".gitignore", "/target\n"),
            ("deny.toml", "[bans]\n"),
            ("rust-toolchain.toml", "[toolchain]\nchannel = \"stable\"\n"),
            ("ci/linux/Dockerfile", "FROM ubuntu\n"),
            (
                "crates/platitude-core/src/lib.rs",
                "pub mod refs;\npub mod stash;\n",
            ),
            (
                "crates/platitude-core/src/stash.rs",
                "pub fn stash() {}\n#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    fn t() { stash() }\n}\n",
            ),
            (
                "crates/platitude-core/src/refs.rs",
                "pub fn refs() {}\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n",
            ),
            (
                "crates/platitude-core/tests/it/main.rs",
                "mod support;\nmod refs_integration;\nmod stash_integration;\n",
            ),
            (
                "crates/platitude-core/tests/it/support/mod.rs",
                "pub fn repo() {}\n",
            ),
            (
                "crates/platitude-core/tests/it/stash_integration.rs",
                "use crate::support;\nuse platitude_core::stash;\n#[test]\nfn s() { support::repo(); stash::stash() }\n",
            ),
            (
                "crates/platitude-core/tests/it/refs_integration.rs",
                "use crate::support;\nuse platitude_core::refs;\n#[test]\nfn r() { support::repo(); refs::refs() }\n",
            ),
            (
                "crates/platitude-app/src/main.rs",
                "mod models;\nfn main() {}\n",
            ),
            (
                "crates/platitude-app/src/models.rs",
                "use platitude_core::stash;\npub struct StashModel;\n#[qobject]\nimpl StashModel {}\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn m() {}\n}\n",
            ),
            (
                // Reading the singleton, as the product's components all
                // do: it is what puts a singleton in the reach of
                // anything, since no run can ever name one.
                "crates/platitude-app/src/ui/Main.qml",
                "Item {\n    spacing: Theme.gap\n    StashPane {}\n}\n",
            ),
            (
                "crates/platitude-app/src/ui/StashPane.qml",
                "Item {\n    property var model: StashModel\n}\n",
            ),
            (
                "crates/platitude-app/src/ui/Theme.qml",
                "pragma Singleton\nQtObject {}\n",
            ),
            (
                "crates/platitude-app/src/ui/qmldir",
                "module platitude.ui\nsingleton Theme 1.0 Theme.qml\nMain 1.0 Main.qml\n",
            ),
            ("crates/platitude-app/src/auto/Driver.qml", "Item {}\n"),
            (
                "crates/xtask/src/main.rs",
                "mod qmltest;\nmod seats;\nfn main() {}\n",
            ),
            (
                "crates/xtask/src/seats.rs",
                "#[cfg(test)]\nmod tests {\n    #[test]\n    fn x() {}\n}\n",
            ),
            ("crates/xtask/src/qmltest.rs", "pub fn run() {}\n"),
            (
                "crates/xtask/verb-census.txt",
                "# census\nstash --preset basic\tDriver Main StashPane\n",
            ),
            // The census's one verb is one the container repeats before a
            // merge, so every test sees both sides' verb blocks; a line
            // the table does not name is the host's alone there.
            (
                "crates/xtask/verb-tiers.txt",
                "linux\tstash --preset basic\t-\tthe sandbox's line on both sides\n",
            ),
            ("internal-docs/notes.md", "# notes\n"),
        ] {
            self.write(&self.repo, path, text);
        }
        self.write(
            &self.repo,
            ".githooks/reference-transaction",
            &checked_in_hook(),
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                self.repo.join(".githooks/reference-transaction"),
                std::fs::Permissions::from_mode(0o755),
            )
            .expect("chmod");
        }
        self.commit_all(&self.repo, "seed", &[("PGG_GATE_SKIP", "1")]);
        self.install_and_seat();
    }

    /// The hook on the primary checkout and the seat worktree beside it —
    /// the shape every test starts from.
    fn install_and_seat(&self) {
        let (ok, text) = self.gate(&self.repo, &["install"], &[]);
        assert!(ok && text.contains("core.hooksPath ="), "install: {text}");
        let (ok, text) = self.gate(&self.repo, &["install"], &[]);
        assert!(
            ok && text.contains("installed"),
            "a second install changes nothing: {text}"
        );
        let seat = self.seat.display().to_string().replace('\\', "/");
        self.git_ok(
            &self.repo,
            &["worktree", "add", "-q", "-b", "worktree-a", &seat, "main"],
        );
        let _ = self.ran();
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = self.git(
            &self.repo,
            &["worktree", "remove", "--force", "seat-a"],
            &[],
        );
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// The hook as checked in, with one line changed: the verdict is this
/// very binary's, so the script's own
/// structure — the state check, the ref match, the tree it moves to —
/// is what runs in the sandbox.
fn checked_in_hook() -> String {
    let checked_in = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(".githooks")
        .join("reference-transaction");
    let exe = EXE.replace('\\', "/");
    let script = std::fs::read_to_string(&checked_in)
        .unwrap_or_else(|e| panic!("{}: {e}", checked_in.display()))
        .replace(
            "exec cargo run --quiet -p xtask --profile hooks -- gate verdict",
            &format!("exec \"{exe}\" gate verdict"),
        );
    assert!(
        script.contains(&format!("exec \"{exe}\"")),
        "the checked-in hook no longer spells the line the tests replace"
    );
    script
}

pub fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

pub fn without_always(ran: &BTreeSet<String>) -> BTreeSet<String> {
    ran.iter()
        .filter(|id| !ALWAYS.contains(&id.as_str()))
        .cloned()
        .collect()
}
