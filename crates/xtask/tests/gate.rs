//! The gate on throwaway repositories: selection follows the reach of a
//! diff, stamps follow the inputs, main refuses what is not stamped, and
//! land rebases → gates → fast-forwards. Every test builds its own
//! repository under the OS temp directory with a git configuration of
//! its own, so they run in parallel and read nobody's identity or hooks.
//!
//! Steps are faked (`PG_GATE_FAKE_LOG`): what is under test is which
//! steps a change owes and when they are asked again, not whether cargo
//! passes. The repository is a miniature of this workspace's shape —
//! enough for the dependency graph to have edges to follow.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const EXE: &str = env!("CARGO_BIN_EXE_xtask");
static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// The three steps every gate runs regardless of the diff.
const ALWAYS: [&str; 3] = ["structure", "waits", "fmt"];

struct Sandbox {
    root: PathBuf,
    gitconfig: PathBuf,
    fake_log: PathBuf,
    /// The primary checkout, on main.
    repo: PathBuf,
    /// A worktree on `worktree-a`, the seat under test.
    seat: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!("pg-gate-{name}-{}-{n}", std::process::id()));
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

    fn env(&self, command: &mut Command) {
        command
            .env("GIT_CONFIG_GLOBAL", &self.gitconfig)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("LC_ALL", "C")
            .env("PG_GATE_FAKE_LOG", &self.fake_log)
            .env_remove("PG_GATE_FAKE_FAIL")
            .env_remove("PG_GATE_SKIP");
    }

    fn git(&self, dir: &Path, args: &[&str], extra: &[(&str, &str)]) -> Result<String, String> {
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

    fn git_ok(&self, dir: &Path, args: &[&str]) -> String {
        self.git(dir, args, &[])
            .unwrap_or_else(|e| panic!("git {} in {}: {e}", args.join(" "), dir.display()))
    }

    /// Runs `cargo xtask gate …` against `dir`; answers (success, output).
    fn gate(&self, dir: &Path, args: &[&str], extra: &[(&str, &str)]) -> (bool, String) {
        let mut command = Command::new(EXE);
        command.arg("gate").args(args).arg("--dir").arg(dir);
        self.env(&mut command);
        for (key, value) in extra {
            command.env(key, value);
        }
        let output = command.output().expect("spawn xtask");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        (output.status.success(), text)
    }

    fn gate_ok(&self, dir: &Path, args: &[&str]) -> String {
        let (ok, text) = self.gate(dir, args, &[]);
        assert!(ok, "gate {} failed:\n{text}", args.join(" "));
        text
    }

    /// `cargo xtask land --branch <b>` run from the primary; answers
    /// (success, output).
    fn land(&self, branch: &str) -> (bool, String) {
        let mut command = Command::new(EXE);
        command
            .args(["land", branch, "--dir"])
            .arg(&self.repo)
            .current_dir(&self.repo);
        self.env(&mut command);
        let output = command.output().expect("spawn xtask");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        (output.status.success(), text)
    }

    /// The steps run since the last look, and the look empties the log.
    fn ran(&self) -> BTreeSet<String> {
        let text = std::fs::read_to_string(&self.fake_log).unwrap_or_default();
        let _ = std::fs::remove_file(&self.fake_log);
        text.lines().map(str::to_string).collect()
    }

    fn write(&self, dir: &Path, relative: &str, text: &str) {
        let path = dir.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("parent");
        }
        std::fs::write(&path, text).expect("write");
    }

    fn commit_all(&self, dir: &Path, message: &str, extra: &[(&str, &str)]) -> String {
        self.git_ok(dir, &["add", "-A"]);
        self.git(dir, &["commit", "-q", "-m", message], extra)
            .unwrap_or_else(|e| panic!("commit {message:?}: {e}"));
        self.git_ok(dir, &["rev-parse", "HEAD"])
    }

    fn head(&self, dir: &Path) -> String {
        self.git_ok(dir, &["rev-parse", "HEAD"])
    }

    fn main_sha(&self) -> String {
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
                "crates/platitude-app/src/ui/Main.qml",
                "Item {\n    StashPane {}\n}\n",
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
        self.commit_all(&self.repo, "seed", &[("PG_GATE_SKIP", "1")]);
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
/// very binary's rather than `cargo run`'s, so the script's own
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
            "exec cargo run --quiet -p xtask -- gate verdict",
            &format!("exec \"{exe}\" gate verdict"),
        );
    assert!(
        script.contains(&format!("exec \"{exe}\"")),
        "the checked-in hook no longer spells the line the tests replace"
    );
    script
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

fn without_always(ran: &BTreeSet<String>) -> BTreeSet<String> {
    ran.iter()
        .filter(|id| !ALWAYS.contains(&id.as_str()))
        .cloned()
        .collect()
}

#[test]
fn a_docs_only_change_owes_the_always_steps_and_nothing_else() {
    let sb = Sandbox::new("docs");
    sb.write(&sb.seat, "internal-docs/notes.md", "# notes\n\nmore\n");
    sb.commit_all(&sb.seat, "docs: more", &[]);
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(text.contains("no step reads these"), "{text}");
    let text = sb.gate_ok(&sb.seat, &[]);
    assert!(text.contains("gate: PASS"), "{text}");
    assert_eq!(sb.ran(), set(&ALWAYS));
}

#[test]
fn a_core_change_owes_what_reads_it_and_nothing_beside_it() {
    let sb = Sandbox::new("core");
    sb.write(&sb.seat, "crates/platitude-core/src/stash.rs", "pub fn stash() { let _ = 1; }\n#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    fn t() { stash() }\n}\n");
    sb.commit_all(&sb.seat, "feat(core): stash", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let ran = without_always(&sb.ran());
    for owed in [
        "clippy platitude-core",
        "clippy-linux platitude-core",
        "clippy platitude-app",
        "test platitude-core 1",
        "test platitude-core 1 linux",
        "test platitude-app 1",
        "test it 1",
        "test it 1 linux",
        "shipped",
        "verify stash --preset basic",
        "verify-linux stash --preset basic",
        "bare",
    ] {
        assert!(ran.contains(owed), "{owed} not run; ran: {ran:?}");
    }
    for spared in ["clippy xtask", "test xtask 1"] {
        assert!(!ran.contains(spared), "{spared} ran; ran: {ran:?}");
    }
    // And the filters are the modules the reach holds, not the crate.
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(
        text.contains("test platitude-core 1") && text.contains("test it 1"),
        "{text}"
    );
}

#[test]
fn a_leaf_core_change_stays_narrow() {
    let sb = Sandbox::new("leaf");
    sb.write(
        &sb.seat,
        "crates/platitude-core/src/refs.rs",
        "pub fn refs() { let _ = 2; }\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(core): refs", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let ran = without_always(&sb.ran());
    assert_eq!(
        ran,
        set(&[
            "clippy platitude-core",
            "clippy-linux platitude-core",
            "test platitude-core 1",
            "test platitude-core 1 linux",
            "test it 1",
            "test it 1 linux",
            "bare",
        ]),
        "{ran:?}"
    );
}

#[test]
fn a_qml_change_owes_the_verbs_whose_census_names_it_and_no_rust_test() {
    let sb = Sandbox::new("qml");
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/StashPane.qml",
        "Item {\n    width: 1\n    property var model: StashModel\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(app-ui): pane", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let ran = without_always(&sb.ran());
    assert_eq!(
        ran,
        set(&[
            "qmltest",
            "qmltest-linux",
            "shipped",
            "verify stash --preset basic",
            "verify-linux stash --preset basic",
            "bare"
        ]),
        "{ran:?}"
    );
}

#[test]
fn a_qtest_file_owes_the_qml_runner_and_nothing_the_app_is_built_for() {
    let sb = Sandbox::new("qmltest");
    sb.write(
        &sb.seat,
        "crates/platitude-app/tests/qml/tst_probe.qml",
        "import QtTest\nItem {\n    TestCase { name: \"Probe\" }\n}\n",
    );
    sb.commit_all(&sb.seat, "test(app-ui): probe", &[]);
    sb.gate_ok(&sb.seat, &[]);
    // It stands in a runner of its own, so no census owes it a verb and
    // nothing here is built: not shipped, not a verb, not bare.
    let ran = without_always(&sb.ran());
    assert_eq!(ran, set(&["qmltest", "qmltest-linux"]), "{ran:?}");

    // The qmldir declares the singletons the tests resolve through, so it
    // is as much of the module as the components are. The file above goes
    // again, so what stands against main is this and nothing else.
    std::fs::remove_file(sb.seat.join("crates/platitude-app/tests/qml/tst_probe.qml")).expect("rm");
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/qmldir",
        "module platitude.ui\nsingleton Theme 1.0 Theme.qml\nMain 1.0 Main.qml\nX 1.0 X.qml\n",
    );
    sb.commit_all(&sb.seat, "chore(app-ui): declare X", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let ran = without_always(&sb.ran());
    assert_eq!(ran, set(&["qmltest", "qmltest-linux"]), "{ran:?}");

    // Their README is not something the runner reads, so it is a document
    // like any other and nothing at all is owed for it.
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/qmldir",
        "module platitude.ui\nsingleton Theme 1.0 Theme.qml\nMain 1.0 Main.qml\n",
    );
    sb.write(
        &sb.seat,
        "crates/platitude-app/tests/qml/README.md",
        "# how they are run\n",
    );
    sb.commit_all(&sb.seat, "docs: how they are run", &[]);
    let text = sb.gate_ok(&sb.seat, &[]);
    assert!(text.contains("no step reads these"), "{text}");
    let ran = without_always(&sb.ran());
    assert!(ran.is_empty(), "{ran:?}");

    // The staging is what a run resolves through, so a change to the
    // runner is a change nothing else here would exercise.
    sb.write(
        &sb.seat,
        "crates/xtask/src/qmltest.rs",
        "pub fn run() { let _ = 1; }\n",
    );
    sb.commit_all(&sb.seat, "fix(xtask): stage it differently", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let ran = sb.ran();
    assert!(ran.contains("qmltest"), "{ran:?}");
    assert!(ran.contains("qmltest-linux"), "{ran:?}");
}

#[test]
fn a_component_no_verb_shows_stops_the_gate_by_name() {
    let sb = Sandbox::new("uncovered");
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/Extra.qml",
        "Item {}\n",
    );
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/Main.qml",
        "Item {\n    StashPane {}\n    Extra {}\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(app-ui): extra", &[]);
    let (ok, text) = sb.gate(&sb.seat, &[], &[]);
    assert!(!ok, "{text}");
    assert!(
        text.contains("no verb shows") && text.contains("ui/Extra.qml"),
        "{text}"
    );
    assert!(
        !text.contains("ui/Main.qml\n") || text.contains("Main"),
        "{text}"
    );
    // A singleton stands in no item tree, so it is never owed a census.
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/Theme.qml",
        "pragma Singleton\nQtObject { property int x: 1 }\n",
    );
    sb.write(
        &sb.seat,
        "crates/platitude-app/src/ui/Main.qml",
        "Item {\n    StashPane {}\n}\n",
    );
    std::fs::remove_file(sb.seat.join("crates/platitude-app/src/ui/Extra.qml")).expect("rm");
    sb.commit_all(&sb.seat, "feat(app-ui): theme", &[]);
    sb.gate_ok(&sb.seat, &[]);
}

#[test]
fn a_step_is_asked_again_only_when_what_it_reads_changed() {
    let sb = Sandbox::new("cache");
    sb.write(&sb.seat, "crates/platitude-core/src/stash.rs", "pub fn stash() { let _ = 3; }\n#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    fn t() { stash() }\n}\n");
    sb.commit_all(&sb.seat, "feat(core): three", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let first = without_always(&sb.ran());
    assert!(
        first.contains("test platitude-core 1") && first.contains("test platitude-app 1"),
        "{first:?}"
    );

    let text = sb.gate_ok(&sb.seat, &[]);
    assert!(text.contains("cached test platitude-core 1"), "{text}");
    assert_eq!(
        sb.ran(),
        set(&ALWAYS),
        "a second run of one commit runs nothing twice"
    );

    // A refs commit on top: the branch's diff now holds both modules, so
    // the core step is a new one (two filters) and runs; the app's unit
    // tests read stash and its readers, none of which moved, so that
    // step stays green. The verbs and the shipped build read the whole
    // app and core — the core moved, so they run again.
    sb.write(
        &sb.seat,
        "crates/platitude-core/src/refs.rs",
        "pub fn refs() { let _ = 4; }\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(core): four", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let second = without_always(&sb.ran());
    assert!(second.contains("test platitude-core 2"), "{second:?}");
    assert!(
        !second.contains("test platitude-app 1"),
        "the app reads stash, not refs: {second:?}"
    );
    assert!(
        second.contains("shipped") && second.contains("verify stash --preset basic"),
        "{second:?}"
    );
}

#[test]
fn a_host_only_run_stamps_half_and_the_full_run_reuses_it() {
    let sb = Sandbox::new("host-only");
    sb.write(
        &sb.seat,
        "crates/platitude-core/src/refs.rs",
        "pub fn refs() { let _ = 5; }\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(core): five", &[]);
    let text = sb.gate_ok(&sb.seat, &["--host-only"]);
    assert!(text.contains("host-only"), "{text}");
    let daily = sb.ran();
    assert!(
        daily.contains("test platitude-core 1") && !daily.contains("test platitude-core 1 linux"),
        "{daily:?}"
    );
    let refused = sb.git(&sb.repo, &["merge", "--ff-only", "worktree-a"], &[]);
    assert!(
        refused.as_ref().is_err_and(|e| e.contains("host-only")),
        "{refused:?}"
    );
    sb.gate_ok(&sb.seat, &[]);
    let rest = without_always(&sb.ran());
    assert!(
        rest.contains("test platitude-core 1 linux") && !rest.contains("test platitude-core 1"),
        "{rest:?}"
    );
    sb.git_ok(&sb.repo, &["merge", "--ff-only", "worktree-a"]);
}

#[test]
fn main_moves_only_onto_a_gated_commit_whatever_moves_it() {
    let sb = Sandbox::new("hook");
    sb.write(
        &sb.seat,
        "crates/platitude-core/src/refs.rs",
        "pub fn refs() { let _ = 6; }\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n",
    );
    let tip = sb.commit_all(&sb.seat, "feat(core): six", &[]);
    let main_before = sb.main_sha();

    let merge = sb.git(&sb.repo, &["merge", "--ff-only", "worktree-a"], &[]);
    assert!(
        merge.as_ref().is_err_and(|e| e.contains("no gate stamp")),
        "{merge:?}"
    );
    let update = sb.git(&sb.seat, &["update-ref", "refs/heads/main", &tip], &[]);
    assert!(
        update.as_ref().is_err_and(|e| e.contains("no gate stamp")),
        "{update:?}"
    );
    sb.write(&sb.repo, "internal-docs/notes.md", "# notes\n\ndirect\n");
    sb.git_ok(&sb.repo, &["add", "-A"]);
    let direct = sb.git(&sb.repo, &["commit", "-q", "-m", "docs: direct"], &[]);
    assert!(
        direct.as_ref().is_err_and(|e| e.contains("no gate stamp")),
        "{direct:?}"
    );
    assert_eq!(sb.main_sha(), main_before, "main did not move");
    // A reset to where main already stands writes the ref to its own
    // value; git runs the hook for it, and it must pass with no stamp.
    sb.git_ok(&sb.repo, &["reset", "-q", "--hard", "HEAD"]);

    // The user's own way past, which the Claude hook denies to sessions.
    sb.write(&sb.repo, "internal-docs/notes.md", "# notes\n\nskipped\n");
    sb.git_ok(&sb.repo, &["add", "-A"]);
    sb.git(
        &sb.repo,
        &["commit", "-q", "-m", "docs: skipped"],
        &[("PG_GATE_SKIP", "1")],
    )
    .expect("PG_GATE_SKIP lets the user through");
    let rewind = sb.git(&sb.repo, &["reset", "-q", "--hard", &main_before], &[]);
    assert!(
        rewind.as_ref().is_err_and(|e| e.contains("no gate stamp")),
        "{rewind:?}"
    );
    sb.git(
        &sb.repo,
        &["reset", "-q", "--hard", &main_before],
        &[("PG_GATE_SKIP", "1")],
    )
    .expect("PG_GATE_SKIP rewinds");

    sb.gate_ok(&sb.seat, &[]);
    sb.git_ok(&sb.repo, &["merge", "--ff-only", "worktree-a"]);
    assert_eq!(sb.main_sha(), tip);
}

#[test]
fn a_red_step_leaves_main_where_it_was() {
    let sb = Sandbox::new("red");
    sb.write(
        &sb.seat,
        "crates/platitude-core/src/refs.rs",
        "pub fn refs() { let _ = 7; }\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(core): seven", &[]);
    let (ok, text) = sb.gate(
        &sb.seat,
        &[],
        &[("PG_GATE_FAKE_FAIL", "test platitude-core 1")],
    );
    assert!(
        !ok && text.contains("FAIL   test platitude-core 1") && text.contains("nothing stamped"),
        "{text}"
    );
    let merge = sb.git(&sb.repo, &["merge", "--ff-only", "worktree-a"], &[]);
    assert!(merge.is_err(), "a red gate stamped nothing: {merge:?}");
    let _ = sb.ran();
    sb.gate_ok(&sb.seat, &[]);
    let again = without_always(&sb.ran());
    assert!(again.contains("test platitude-core 1"), "{again:?}");
    assert!(
        !again.contains("test platitude-core 1 linux"),
        "the linux side was green and stays so: {again:?}"
    );
}

#[test]
fn land_rebases_then_gates_then_fast_forwards() {
    let sb = Sandbox::new("land");
    sb.write(&sb.repo, "internal-docs/notes.md", "# notes\n\nmoved\n");
    let moved = sb.commit_all(&sb.repo, "docs: main moved", &[("PG_GATE_SKIP", "1")]);
    sb.write(
        &sb.seat,
        "crates/platitude-core/src/refs.rs",
        "pub fn refs() { let _ = 8; }\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n",
    );
    let before = sb.commit_all(&sb.seat, "feat(core): eight", &[]);
    assert!(
        sb.git(
            &sb.repo,
            &["merge-base", "--is-ancestor", "main", "worktree-a"],
            &[]
        )
        .is_err()
    );

    let (ok, text) = sb.land("worktree-a");
    assert!(ok, "{text}");
    assert!(text.contains("rebasing"), "{text}");
    assert!(text.contains("landed worktree-a"), "{text}");
    let after = sb.head(&sb.seat);
    assert_ne!(after, before, "the rebase rewrote the commit");
    assert_eq!(sb.main_sha(), after, "main is the seat's tip");
    assert_eq!(
        sb.git_ok(&sb.seat, &["rev-parse", "HEAD~1"]),
        moved,
        "on top of what main had"
    );
    let ran = without_always(&sb.ran());
    assert!(ran.contains("test platitude-core 1"), "{ran:?}");
    let (ok, text) = sb.land("worktree-a");
    assert!(ok && text.contains("nothing to land"), "{text}");
}

#[test]
fn a_pseudo_run_off_main_is_reused_after_the_rebase() {
    let sb = Sandbox::new("pseudo");
    sb.write(&sb.repo, "internal-docs/notes.md", "# notes\n\nmoved\n");
    sb.commit_all(&sb.repo, "docs: main moved", &[("PG_GATE_SKIP", "1")]);
    sb.write(
        &sb.seat,
        "crates/platitude-core/src/refs.rs",
        "pub fn refs() { let _ = 9; }\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n",
    );
    let tip = sb.commit_all(&sb.seat, "feat(core): nine", &[]);
    let text = sb.gate_ok(&sb.seat, &[]);
    assert!(text.contains("off main"), "{text}");
    let pseudo = without_always(&sb.ran());
    assert!(pseudo.contains("test platitude-core 1"), "{pseudo:?}");
    let update = sb.git(&sb.seat, &["update-ref", "refs/heads/main", &tip], &[]);
    assert!(
        update.as_ref().is_err_and(|e| e.contains("off main")),
        "{update:?}"
    );

    let (ok, text) = sb.land("worktree-a");
    assert!(
        ok && text.contains("rebasing") && text.contains("landed"),
        "{text}"
    );
    assert_eq!(sb.ran(), set(&ALWAYS), "the rebase owed nothing new");
    assert_eq!(sb.main_sha(), sb.head(&sb.seat));
}

#[test]
fn a_rebase_that_touches_a_step_s_inputs_reruns_that_step_only() {
    let sb = Sandbox::new("rebase-inputs");
    // Main's move touches the stash module.
    sb.write(&sb.repo, "crates/platitude-core/src/stash.rs", "pub fn stash() { let _ = 10; }\n#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    fn t() { stash() }\n}\n");
    sb.commit_all(&sb.repo, "feat(core): main moved", &[("PG_GATE_SKIP", "1")]);
    sb.write(
        &sb.seat,
        "crates/platitude-core/src/refs.rs",
        "pub fn refs() { let _ = 11; }\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(core): eleven", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let _ = sb.ran();
    let (ok, text) = sb.land("worktree-a");
    assert!(ok, "{text}");
    let again = without_always(&sb.ran());
    // The refs tests read refs.rs only: same objects, still green. The it
    // binary's inputs include support and both modules' readers — its
    // refs filter reads refs_integration, which reads support: unchanged
    // too. clippy reads the whole crate: main's move changed that.
    assert!(
        !again.contains("test platitude-core 1"),
        "refs' inputs did not move: {again:?}"
    );
    assert!(
        again.contains("clippy platitude-core"),
        "the crate as a whole moved: {again:?}"
    );
}

#[test]
fn land_refuses_a_dirty_seat_and_a_rebase_that_stops_is_walked_back() {
    let sb = Sandbox::new("refusals");
    sb.write(
        &sb.seat,
        "crates/platitude-core/src/refs.rs",
        "pub fn refs() { let _ = 12; }\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n",
    );
    let tip = sb.commit_all(&sb.seat, "feat(core): twelve", &[]);
    sb.write(
        &sb.seat,
        "crates/platitude-core/src/extra.rs",
        "// unsaved\n",
    );
    let (ok, text) = sb.land("worktree-a");
    assert!(!ok && text.contains("uncommitted"), "{text}");
    std::fs::remove_file(sb.seat.join("crates/platitude-core/src/extra.rs")).expect("clean");

    sb.write(
        &sb.repo,
        "crates/platitude-core/src/refs.rs",
        "pub fn refs() { let _ = 13; }\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n",
    );
    let main_before = sb.commit_all(&sb.repo, "feat(core): thirteen", &[("PG_GATE_SKIP", "1")]);
    let (ok, text) = sb.land("worktree-a");
    assert!(!ok && text.contains("walked back"), "{text}");
    assert_eq!(sb.head(&sb.seat), tip, "the seat stands where it did");
    assert!(
        sb.git(
            &sb.seat,
            &["rev-parse", "--verify", "--quiet", "REBASE_HEAD"],
            &[]
        )
        .is_err(),
        "no rebase left in flight"
    );
    assert_eq!(sb.main_sha(), main_before);
}

#[test]
fn an_xtask_change_leaves_the_app_alone_and_runs_on_both_sides() {
    let sb = Sandbox::new("xtask");
    sb.write(
        &sb.seat,
        "crates/xtask/src/seats.rs",
        "pub fn seat() {}\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn x() {}\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(xtask): seats", &[]);
    sb.gate_ok(&sb.seat, &[]);
    let ran = without_always(&sb.ran());
    assert_eq!(
        ran,
        set(&[
            "clippy xtask",
            "clippy-linux xtask",
            "test xtask 1",
            "test xtask 1 linux",
        ]),
        "{ran:?}"
    );
}

#[test]
fn install_leaves_somebody_elses_hooks_directory_alone() {
    let sb = Sandbox::new("foreign-hooks");
    sb.git_ok(&sb.repo, &["config", "core.hooksPath", "/somewhere/else"]);
    let (ok, text) = sb.gate(&sb.repo, &["install"], &[]);
    assert!(!ok && text.contains("not the gate's"), "{text}");
    assert_eq!(
        sb.git_ok(&sb.repo, &["config", "--get", "core.hooksPath"]),
        "/somewhere/else"
    );
}

#[test]
fn a_build_input_change_owes_everything() {
    let sb = Sandbox::new("lockfile");
    sb.write(&sb.seat, "Cargo.lock", "# lock\n# bumped\n");
    sb.commit_all(&sb.seat, "chore: bump", &[]);
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(text.contains("everything (Cargo.lock)"), "{text}");
    sb.gate_ok(&sb.seat, &[]);
    let ran = without_always(&sb.ran());
    for owed in [
        "test platitude-core (all)",
        "test platitude-app (all)",
        "test xtask (all)",
        "test it (all)",
        "clippy platitude-core",
        "clippy xtask",
        "qmltest",
        "shipped",
        "verify stash --preset basic",
        "bare",
        "deny",
    ] {
        assert!(ran.contains(owed), "{owed} not run; ran: {ran:?}");
    }
}

#[test]
fn a_policy_change_owes_cargo_deny_and_nothing_else() {
    let sb = Sandbox::new("deny");
    sb.write(
        &sb.seat,
        "deny.toml",
        "[bans]\nmultiple-versions = \"allow\"\n",
    );
    sb.commit_all(&sb.seat, "chore(deps): a ban", &[]);
    let text = sb.gate_ok(&sb.seat, &["--dry-run"]);
    assert!(!text.contains("no step reads these"), "{text}");
    sb.gate_ok(&sb.seat, &[]);
    assert_eq!(without_always(&sb.ran()), set(&["deny"]));
}

/// The policy is read against the closure, not against the sources: a
/// change to neither leaves the step unselected, however far it reaches.
#[test]
fn a_source_change_owes_no_policy_check() {
    let sb = Sandbox::new("deny-source");
    sb.write(
        &sb.seat,
        "crates/platitude-core/src/refs.rs",
        "pub fn refs() { let _ = 14; }\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n",
    );
    sb.commit_all(&sb.seat, "feat(core): refs", &[]);
    sb.gate_ok(&sb.seat, &[]);
    assert!(!sb.ran().contains("deny"));
}
