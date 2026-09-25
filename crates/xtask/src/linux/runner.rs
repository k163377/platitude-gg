//! `cargo xtask linux runner <name> --gate <pid>` — the Linux side's own
//! copy of the task runner, and the lines that start from it.
//!
//! **What it is for.** Every verb of a gate's Linux side is spelled
//! `cargo xtask <verb>` in there ([`super::command_line`]), and a full
//! gate is five hundred of them, each a cargo resolving /work's manifests
//! and lock across the mount before the verb runs — the host's
//! `gate::runner` one boundary further in. `linux test`, `clippy`, `bare`
//! and `offline` are cargo's work and stay cargo's.
//!
//! **Whose directory, and whose lock.** `/work/target` is this
//! checkout's own docker volume (`super::volume`). The build takes that
//! volume's cargo lock and gives it back when its container ends, before
//! any step of the side starts.
//!
//! **Who may take a copy away.** A copy under `gate-runner` is what a
//! gate's verbs start from, and neither thing that reaches for it can be
//! answered by asking who is running: a preparation can be typed beside
//! a live gate (the note is a file anybody reads), and one can outlive
//! the gate that sent it — **killing a launcher does not kill what it
//! started**. An emptying run from a place of its own only moves this:
//! whatever runs it can be held up too. So **the emptying is bounded by
//! what it can name** ([`SCRIPT`]).
//!
//! **Everything is one step** — the one container (or `sh`) the gate
//! starts through `check::run_step`, under its ceiling, log and tree
//! kill. A docker command on the side would be a road out of that
//! ceiling (`budget::watched` has none) and a second container to orphan.
//!
//! **Installed by rename**: a copy written in place is a binary somebody
//! may be executing (ETXTBSY, or a half-written one). The name carries
//! the run that asked for it, so an older copy is never the one a line
//! names.
//!
//! **A failure here stops the side** (`gate::sides::run_sides`); there
//! is no road back to `cargo xtask`.

use std::path::Path;

use super::{TARGET_MOUNT, XTASK_VERBS};

/// The directory's name under the build directory, wherever that stands.
const UNDER_TARGET: &str = "gate-runner";

/// What a run's copy may be called: the name is written into a shell
/// script and a file name. Asked where a copy is prepared ([`prepare`])
/// and where one is named (`super::run`'s `--runner`), because the
/// guard's script spells the path unquoted (`super::watched_from_inside`).
pub(super) fn spelled(name: &str) -> Result<(), String> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err(format!(
            "a runner name is letters, digits, dot, dash and underscore; got {name:?}"
        ));
    }
    Ok(())
}

/// Where the copies stand as the run sees them: the build volume in the
/// container, or the tree's own build directory on a Linux host
/// (`super::here`). A string, not a path: on Windows it names a place in
/// the container, which `PathBuf::join` would spell with a backslash.
fn dir(root: &Path) -> String {
    format!("{}/{UNDER_TARGET}", target(root))
}

/// The build directory the run writes into, in the same two spellings.
fn target(root: &Path) -> String {
    if cfg!(target_os = "linux") {
        format!("{}/target", root.display())
    } else {
        TARGET_MOUNT.to_string()
    }
}

/// The copy `name` asks for, which is what a line starts from.
pub(crate) fn at(root: &Path, name: &str) -> String {
    format!("{}/xtask-{name}", dir(root))
}

/// Whether a `linux <command…>` line is one of the task runner's own
/// verbs — the only lines a prepared copy starts. The verb must lead: a
/// line with cargo's own options ahead (`super::subcommand_at`) is cargo's.
pub(crate) fn a_runner_verb(rest: &[String]) -> bool {
    rest.first()
        .is_some_and(|verb| XTASK_VERBS.contains(&verb.as_str()))
}

/// Whether the live gate of this tree is the one asking. It keeps a
/// stray `linux runner` out of a running gate's volume and cargo lock,
/// and it establishes that the note [`SCRIPT`] reads its pid from is a
/// live gate's: the lock beside it must be held (liveness is the lock,
/// as in `lanes`) and the pid in it must be the one the gate spelled
/// into the step (`gate::runner::linux_runner`).
///
/// It is not what protects the copies — a line can name the live gate,
/// and a preparation can outlive it. [`SCRIPT`]'s own bounds are.
fn owned_by_the_gate(root: &Path, named: u32) -> Result<(), String> {
    let note = note_of(root);
    let held = match std::fs::File::open(note.with_extension("lock")) {
        Ok(lock) => match lock.try_lock() {
            // Nobody holds it: whatever note stands beside it is litter.
            Ok(()) => {
                let _ = lock.unlock();
                false
            }
            Err(std::fs::TryLockError::WouldBlock) => true,
            Err(std::fs::TryLockError::Error(error)) => {
                return Err(format!(
                    "could not probe the gate lock at {}: {error}",
                    note.display()
                ));
            }
        },
        // No lock file: no gate has ever run in this tree.
        Err(_) => false,
    };
    if !held {
        return Err(format!(
            "no gate is running in {} — `{}` is the gate's own step: it builds in this \
             checkout's volume and stands at its one preparation container. Run \
             `cargo xtask gate`.",
            root.display(),
            super::RUNNER.call
        ));
    }
    let there = std::fs::read_to_string(&note)
        .ok()
        .and_then(|text| crate::still::Note::parse(&text))
        .ok_or_else(|| {
            format!(
                "a gate holds {} but its note is not readable — nothing here may build in a \
                 volume it cannot name the holder of",
                root.display()
            )
        })?;
    if there.pid() != named {
        return Err(format!(
            "the gate running in {} is {}, and this was sent for pid {named} — a preparation \
             builds in that gate's volume and stands at its preparation container, so it \
             runs under that gate or not at all",
            root.display(),
            there.line()
        ));
    }
    Ok(())
}

/// Builds the task runner in there and installs it under `name`, for the
/// gate at `gate` — which has to be the one holding this tree
/// ([`owned_by_the_gate`]).
pub(crate) fn prepare(root: &Path, name: &str, gate: u32) -> Result<(), String> {
    spelled(name)?;
    owned_by_the_gate(root, gate)?;
    if cfg!(target_os = "linux") {
        // On a Linux host the note is at its own path.
        let line = line(root, name, &note_of(root).display().to_string(), None);
        return super::here(root, &line);
    }
    // The small image: the task runner holds no Qt (`super::QT_FREE`),
    // and the copy it leaves runs in either stage — the app stage is
    // built from this one (ci/linux/Dockerfile).
    let tag = super::ensure_image(root, "core", false)?;
    let line = line(root, name, NOTE_MOUNT, Some(super::DEMO_MOUNT));
    super::in_container(root, &tag, &line, false, None, Some(&note_of(root)))?;
    // Then the verbs' container, from the image they need — after the
    // copy, so a red build starts nothing (`super::container`).
    let app = super::ensure_image(root, "app", false)?;
    super::container::take_down_earlier_gates(root)?;
    super::container::start_container(root, name, gate, &app)
}

/// The preparation as a line: [`SCRIPT`], and then the things it works
/// on as arguments. Nothing is spelled into the script: on a Linux host
/// these are the checkout's own paths (`target`), and one holding a
/// space would come apart into two operands.
///
/// `demo` is the demo volume to sweep, `None` where the host runs this
/// itself: there `/tmp/pgg-demo` is the host's, under its own sweep
/// (`verify::ownership::sweep_yesterdays_runs`).
fn line(root: &Path, name: &str, note: &str, demo: Option<&str>) -> Vec<String> {
    vec![
        "sh".to_string(),
        "-c".to_string(),
        SCRIPT.to_string(),
        // `$0`, which `sh` names the script by in its own messages.
        "pgg-runner".to_string(),
        dir(root),
        target(root),
        name.to_string(),
        note.to_string(),
        demo.unwrap_or_default().to_string(),
    ]
}

/// Where the tree's gate note stands on the host. A container reads it
/// through a read-only bind mount ([`NOTE_MOUNT`]): `/work/target` is the
/// build volume, which covers the host's `target` in there.
pub(crate) fn note_of(root: &Path) -> std::path::PathBuf {
    root.join("target").join("gate-running")
}

/// Where that mount lands. Not under `/work`, which the volume covers.
pub(crate) const NOTE_MOUNT: &str = "/pgg-gate-running";

/// What the preparation does, in the order it does it. A constant, so
/// the order is a test and not a container.
///
/// **The emptying is safe because of what it can name, not because of
/// who runs it or when** — this gate, something typed beside it, or a
/// container that outlived its gate. Two bounds, each covering where the
/// other gives out:
///
/// * **One reading, taken before anything is removed.** Every removal
///   comes out of `here`, so a copy installed after that reading is out
///   of reach — the bound for a container held up after it read.
/// * **The pid is read from the gate note next to each decision.**
///   Copies are named `xtask-<seconds>-<pid>`, so sparing `*-$live`
///   spares the one a gate is using. A pid handed in at the top would be
///   an answer from the moment the container started, the moment not to
///   be trusted; read in the loop, a container held up *before* its
///   reading spares whichever gate is there now.
///
/// What either leaves behind is a copy nobody names, which the next
/// preparation's reading includes.
///
/// **A reading that did not work is not a licence to delete.** No pid,
/// or one that is not digits, ends the preparation: an empty `$live`
/// makes the pattern `*-`, which spares nothing. The first line is taken
/// inside `sed` and **not through a pipe** — `sed … | head -1` hands
/// back `head`'s status, so `set -e` would not stop on a `sed` that could
/// not open the note.
///
/// **It also takes yesterday's demo repositories with it**, as the host
/// does on its way into a gate (`verify::ownership::sweep_yesterdays_runs`):
/// the volume is one checkout's, and otherwise gains about five hundred
/// run directories a gate and loses none — a file count the VM pays for
/// ([the record](../../../../ci/baseline/wsl-memory-windows-x64.md)).
/// Spared: templates (`.pgg-template-ready` / `-refused`), trees somebody
/// kept (`.pgg-keep`), and anything under thirty minutes old — a
/// `cargo xtask linux verify-ui` run by hand beside the gate, which its
/// lane does not cover (a verb's ceiling is ten minutes; thirty is three).
const SCRIPT: &str = "set -e\n\
     dir=$1\n\
     target=$2\n\
     name=$3\n\
     note=$4\n\
     demo=$5\n\
     if [ -d \"$demo\" ]; then\n\
     \x20 was=$(find \"$demo\" -mindepth 1 -maxdepth 1 -type d | wc -l)\n\
     \x20 find \"$demo\" -mindepth 1 -maxdepth 1 -type d -mmin +30 \
     | while IFS= read -r d; do\n\
     \x20 \x20 if [ -e \"$d/.pgg-template-ready\" ]; then continue; fi\n\
     \x20 \x20 if [ -e \"$d/.pgg-template-refused\" ]; then continue; fi\n\
     \x20 \x20 if [ -e \"$d/.pgg-keep\" ]; then continue; fi\n\
     \x20 \x20 rm -rf -- \"$d\"\n\
     \x20 done\n\
     \x20 now=$(find \"$demo\" -mindepth 1 -maxdepth 1 -type d | wc -l)\n\
     \x20 echo \"the demo volume's run directories: $was -> $now\"\n\
     fi\n\
     mkdir -p \"$dir\"\n\
     here=$(find \"$dir\" -maxdepth 1 -type f)\n\
     printf '%s\\n' \"$here\" | while IFS= read -r f; do\n\
     \x20 [ -n \"$f\" ] || continue\n\
     \x20 live=$(sed -n '/^pid /{s/^pid //p;q;}' \"$note\") || live=''\n\
     \x20 case \"$live\" in\n\
     \x20 \x20 '' | *[!0-9]*)\n\
     \x20 \x20 \x20 echo \"pgg-runner: no pid to spare in $note — \
     nothing here decides what to take away without one\" >&2\n\
     \x20 \x20 \x20 exit 1 ;;\n\
     \x20 esac\n\
     \x20 case \"${f##*/}\" in *-\"$live\") continue ;; esac\n\
     \x20 rm -f -- \"$f\"\n\
     done\n\
     cargo build --locked -p xtask\n\
     test -x \"$target/debug/xtask\"\n\
     cp \"$target/debug/xtask\" \"$dir/.writing-$name\"\n\
     chmod 755 \"$dir/.writing-$name\"\n\
     mv -f \"$dir/.writing-$name\" \"$dir/xtask-$name\"\n\
     echo \"the Linux side's task runner: $dir/xtask-$name\"\n";

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{NOTE_MOUNT, SCRIPT, a_runner_verb, at, line, spelled};

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(String::from).collect()
    }

    /// Spelled the way the container reads paths, whatever the host's
    /// spelling.
    #[test]
    #[cfg(not(target_os = "linux"))]
    fn a_copy_stands_in_the_build_volume_under_a_container_path() {
        let root = Path::new("C:\\Users\\x\\IdeaProjects\\platitude-gg");
        assert_eq!(
            at(root, "1758-42"),
            "/work/target/gate-runner/xtask-1758-42"
        );
        assert!(at(root, "1758-42").starts_with(super::TARGET_MOUNT));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn on_linux_the_copy_stands_in_the_trees_own_build_directory() {
        let root = Path::new("/home/x/platitude-gg");
        assert_eq!(
            at(root, "1758-42"),
            "/home/x/platitude-gg/target/gate-runner/xtask-1758-42"
        );
    }

    #[test]
    fn the_name_of_a_copy_carries_the_run_that_asked_for_it() {
        let root = Path::new("/w");
        assert_ne!(at(root, "1758-42"), at(root, "1758-43"));
    }

    /// The name is written into a shell script: it is a name and not a
    /// line of shell.
    #[test]
    fn a_name_is_letters_digits_and_three_marks() {
        assert!(spelled("1758246913-40244").is_ok());
        assert!(spelled("a.b_c-1").is_ok());
        assert!(spelled("").is_err());
        assert!(spelled("a b").is_err());
        assert!(spelled("a;rm -rf /").is_err());
        assert!(spelled("$(id)").is_err());
        assert!(spelled("../../etc/passwd").is_err());
    }

    /// `set -e`, and the build stands ahead of every line that installs,
    /// so a red build leaves this run nothing to start from (the gate's
    /// failure, not a quieter road). Written beside the name and renamed
    /// onto it, so a copy appears whole or not at all.
    #[test]
    fn a_copy_appears_only_after_a_green_build_and_only_whole() {
        let at = |what: &str| {
            SCRIPT
                .lines()
                .position(|l| l.contains(what))
                .unwrap_or_else(|| panic!("{what} is not in the script:\n{SCRIPT}"))
        };
        assert!(SCRIPT.starts_with("set -e\n"), "{SCRIPT}");
        assert!(at("cargo build --locked -p xtask") < at("cp "));
        assert!(at("test -x") < at("cp "));
        assert!(at("cp ") < at("mv -f"));
        assert!(SCRIPT.contains("cp \"$target/debug/xtask\" \"$dir/.writing-$name\"\n"));
        assert!(SCRIPT.contains("mv -f \"$dir/.writing-$name\" \"$dir/xtask-$name\"\n"));
    }

    /// Read out of the script itself ([`SCRIPT`]): a re-reading emptying,
    /// or one that spares "everything but mine", takes the next gate's copy.
    #[test]
    fn the_emptying_is_bounded_by_one_reading_and_spares_the_live_gate() {
        let at = |what: &str| {
            SCRIPT
                .lines()
                .position(|l| l.contains(what))
                .unwrap_or_else(|| panic!("{what} is not in the script:\n{SCRIPT}"))
        };
        // One reading, taken before anything is removed, and every
        // removal comes out of it.
        assert!(
            SCRIPT.contains("here=$(find \"$dir\" -maxdepth 1 -type f)\n"),
            "{SCRIPT}"
        );
        assert!(at("here=$(find") < at("rm -f"));
        assert!(SCRIPT.contains("printf '%s\\n' \"$here\" |"), "{SCRIPT}");
        // Nothing re-reads the runner directory to decide what to
        // remove. The demo volume's own sweep lists its own directory,
        // which is a different set with a different rule.
        assert_eq!(
            SCRIPT.matches("find \"$dir\"").count(),
            1,
            "a second listing is a second chance to name something newer:\n{SCRIPT}"
        );
        assert!(
            !SCRIPT.contains("-delete"),
            "`find -delete` decides and removes in one pass, at the moment it runs:\n{SCRIPT}"
        );
        // The live gate's copy is spared by the pid its note carries,
        // read inside the loop.
        assert!(
            SCRIPT.contains("case \"${f##*/}\" in *-\"$live\") continue ;; esac\n"),
            "{SCRIPT}"
        );
        assert!(
            at("printf '%s") < at("live=$(sed"),
            "the note is read late:\n{SCRIPT}"
        );
        assert!(at("live=$(sed") < at("case \"${f##*/}\""), "{SCRIPT}");
        assert!(
            !SCRIPT.contains("| head"),
            "a pipe hides the reading's own status:\n{SCRIPT}"
        );
        assert!(
            at("live=$(sed") < at("*[!0-9]*"),
            "what it read is checked before it is used:\n{SCRIPT}"
        );
        assert!(at("*[!0-9]*") < at("case \"${f##*/}\""), "{SCRIPT}");
        // The emptying is over before the long part starts, so a killed
        // launcher leaves it unfinished in the shortest window it can.
        assert!(at("rm -f") < at("cargo build --locked -p xtask"));
    }

    /// A checkout whose path holds a space cannot come apart into two
    /// operands ([`line`]).
    #[test]
    fn the_paths_reach_the_script_as_arguments_and_not_as_text() {
        let root = Path::new("/a place/platitude-gg");
        let line = line(root, "1758-40", "/pgg-gate-running", Some("/tmp/pgg-demo"));
        assert_eq!(line[..3], ["sh", "-c", SCRIPT].map(String::from)[..]);
        assert_eq!(
            line[3..],
            [
                "pgg-runner".to_string(),
                super::dir(root),
                super::target(root),
                "1758-40".to_string(),
                NOTE_MOUNT.to_string(),
                "/tmp/pgg-demo".to_string()
            ][..]
        );
        for held in [super::dir(root), super::target(root)] {
            assert!(
                !SCRIPT.contains(&held),
                "{held} is spelled into the script: {SCRIPT}"
            );
        }
    }

    /// The same thing, run, with a stub `cargo`. Linux only, where these
    /// are the checkout's own paths ([`target`]); the container runs this
    /// crate's tests every gate.
    #[test]
    #[cfg(target_os = "linux")]
    fn a_checkout_whose_path_has_a_space_still_gets_its_copy() {
        use std::os::unix::fs::PermissionsExt;

        // The space is on the checkout's own directory; the yard above
        // it has none ([`crate::yard`]).
        let yard = crate::yard::Yard::new("runner-space");
        let root = yard.join("pgg runner");
        let stub = root.join("bin");
        std::fs::create_dir_all(&stub).expect("a directory with a space in its name");
        let cargo = stub.join("cargo");
        std::fs::write(
            &cargo,
            "#!/bin/sh\nmkdir -p target/debug\nprintf x > target/debug/xtask\n\
             chmod 755 target/debug/xtask\n",
        )
        .expect("a stub cargo");
        std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755)).expect("+x");
        // A dead run's copy, which the emptying takes, and the copy of
        // the gate the note names, which it must not.
        let note = a_note(&root, 40);
        let held = std::path::PathBuf::from(super::dir(&root));
        std::fs::create_dir_all(&held).expect("the runner directory");
        std::fs::write(held.join("xtask-older"), "x").expect("a dead run's copy");
        std::fs::write(held.join("xtask-1750-40"), "x").expect("the noted gate's copy");

        let out = ran(&line(&root, "1758-40", &note, None), &root, &stub);
        assert!(
            out.status.success(),
            "the preparation failed in {}:\n{}{}",
            root.display(),
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            std::path::Path::new(&at(&root, "1758-40")).is_file(),
            "no copy at {}",
            at(&root, "1758-40")
        );
        assert!(
            !held.join("xtask-older").exists(),
            "a dead run's copy was left behind"
        );
        assert!(
            held.join("xtask-1750-40").exists(),
            "the live gate's copy was taken — the emptying has to spare its pid"
        );
    }

    /// Three kinds stay: a template, a tree somebody marked, and one
    /// written in the last half hour. Runs the real [`SCRIPT`], the only
    /// thing that can say whether the shell holds.
    #[test]
    #[cfg(target_os = "linux")]
    fn the_demo_volumes_run_directories_go_and_the_templates_stay() {
        use std::os::unix::fs::PermissionsExt;

        let root = crate::yard::Yard::new("runner-sweep");
        let stub = root.join("bin");
        std::fs::create_dir_all(&stub).expect("a tree");
        let cargo = stub.join("cargo");
        std::fs::write(
            &cargo,
            "#!/bin/sh\nmkdir -p target/debug\nprintf x > target/debug/xtask\n\
             chmod 755 target/debug/xtask\n",
        )
        .expect("a stub cargo");
        std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755)).expect("+x");

        let demo = root.join("demo");
        let made = |leaf: &str| {
            let dir = demo.join(leaf);
            std::fs::create_dir_all(&dir).expect("a demo root");
            std::fs::write(dir.join("a.txt"), "x").expect("something in it");
            dir
        };
        let gone = made("basic-41-1789-0");
        let template = made("basic");
        std::fs::write(template.join(".pgg-template-ready"), "").expect("the marker");
        let marked = made("kept-41-1789-0");
        std::fs::write(marked.join(".pgg-keep"), "").expect("the mark");
        let fresh = demo.join("basic-42-1790-0");
        std::fs::create_dir_all(&fresh).expect("a run standing beside the gate");
        // Two hours back, clear of the thirty-minute floor whatever the
        // clock does — and one child for all three: a spawn holds every
        // open handle, other tests' locks included, until it execs
        // (.claude/rules-refs/core.md「lock は unlock で離す」).
        let status = std::process::Command::new("touch")
            .args(["-d", "-2 hours"])
            .args([&gone, &template, &marked])
            .status()
            .expect("touch");
        assert!(status.success(), "could not age the demo roots");

        let note = a_note(&root, 40);
        let line = line(&root, "1758-40", &note, Some(&demo.display().to_string()));
        let out = ran(&line, &root, &stub);

        assert!(
            out.status.success(),
            "the preparation failed:\n{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(!gone.exists(), "a finished run's demo repository was left");
        assert!(template.exists(), "the template was swept out");
        assert!(marked.exists(), "a tree somebody marked was swept out");
        assert!(fresh.exists(), "a run standing beside the gate was swept");
        assert!(
            String::from_utf8_lossy(&out.stdout)
                .contains("the demo volume's run directories: 4 -> 3"),
            "the sweep did not say what it took:\n{}",
            String::from_utf8_lossy(&out.stdout)
        );
    }

    /// A gate note naming `pid`, at the path the run would read it at,
    /// and that path back.
    #[cfg(target_os = "linux")]
    fn a_note(root: &Path, pid: u32) -> String {
        let note = super::note_of(root);
        std::fs::create_dir_all(note.parent().expect("target")).expect("a tree");
        std::fs::write(&note, format!("pid {pid}\nsince 1\nwhat gate\n")).expect("the note");
        note.display().to_string()
    }

    /// Runs a preparation line with a stub `cargo` on PATH.
    #[cfg(target_os = "linux")]
    fn ran(line: &[String], root: &Path, stub: &Path) -> std::process::Output {
        std::process::Command::new(&line[0])
            .args(&line[1..])
            .current_dir(root)
            .env("PATH", format!("{}:/usr/bin:/bin", stub.display()))
            .output()
            .expect("sh")
    }

    /// Every way the note can fail to yield a pid, a failing `sed`
    /// included, is put to the real [`SCRIPT`]: each leaves the live copy
    /// standing, ends non-zero, says why, and installs nothing.
    #[test]
    #[cfg(target_os = "linux")]
    fn a_note_that_yields_no_pid_takes_nothing_away() {
        use std::os::unix::fs::PermissionsExt;

        // The note's bytes, and whether `sed` itself is made to fail.
        for (what, wrote, break_sed) in [
            ("unreadable", Some("pid 22\nsince 1\nwhat gate\n"), true),
            ("empty", Some(""), false),
            ("no pid line", Some("since 1\nwhat gate\n"), false),
            ("not a number", Some("pid seventeen\nsince 1\n"), false),
            ("pid with a tail", Some("pid 22x\nsince 1\n"), false),
            ("absent", None, false),
        ] {
            let root = crate::yard::Yard::new(&format!("nopid-{}", what.replace(' ', "-")));
            let stub = root.join("bin");
            std::fs::create_dir_all(&stub).expect("a tree");
            let cargo = stub.join("cargo");
            std::fs::write(
                &cargo,
                "#!/bin/sh\nmkdir -p target/debug\nprintf x > target/debug/xtask\n\
                 chmod 755 target/debug/xtask\n",
            )
            .expect("a stub cargo");
            std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755)).expect("+x");
            if break_sed {
                // What a `sed` that cannot open the note does: it says
                // so and ends non-zero.
                let sed = stub.join("sed");
                std::fs::write(&sed, "#!/bin/sh\necho 'sed: cannot read' >&2\nexit 2\n")
                    .expect("a stub sed");
                std::fs::set_permissions(&sed, std::fs::Permissions::from_mode(0o755)).expect("+x");
            }
            let note = super::note_of(&root);
            std::fs::create_dir_all(note.parent().expect("target")).expect("a tree");
            match wrote {
                Some(text) => std::fs::write(&note, text).expect("the note"),
                None => {
                    let _ = std::fs::remove_file(&note);
                }
            }
            let held = std::path::PathBuf::from(super::dir(&root));
            std::fs::create_dir_all(&held).expect("the runner directory");
            let live = held.join("xtask-1750-22");
            std::fs::write(&live, "x").expect("a live gate's copy");

            let line = line(&root, "1758-40", &note.display().to_string(), None);
            let out = ran(&line, &root, &stub);
            let said = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            assert!(
                live.exists(),
                "{what}: the live gate's copy was taken on a reading that yielded no pid\n{said}"
            );
            assert!(
                !out.status.success(),
                "{what}: it ended green, so nothing says the emptying did not happen\n{said}"
            );
            assert!(
                said.contains("no pid to spare"),
                "{what}: it stopped without saying why\n{said}"
            );
            assert!(
                !std::path::Path::new(&at(&root, "1758-40")).exists(),
                "{what}: it installed a copy after failing to read the note"
            );
        }
    }

    /// A preparation that outlived its gate finishes after the next run
    /// installed: its reading predates that copy, so nothing in it
    /// reaches the copy ([`SCRIPT`]). The pause is a stub `find` that
    /// lists, says so, and waits for a file the test writes — the order
    /// is the test's, not a clock's.
    #[test]
    #[cfg(target_os = "linux")]
    fn a_preparation_that_outlived_its_gate_cannot_reach_a_later_copy() {
        use std::os::unix::fs::PermissionsExt;

        let root = crate::yard::Yard::new("runner-stale");
        let stub = root.join("bin");
        std::fs::create_dir_all(&stub).expect("a tree");
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).expect("+x");
        let go = root.join("go");
        let read = root.join("read");
        // The pause is inside the reading, which is where a stale one
        // is held.
        let find = stub.join("find");
        std::fs::write(
            &find,
            format!(
                "#!/bin/sh\n/usr/bin/find \"$@\"\ntouch '{}'\nuntil [ -f '{}' ]; do sleep 0.05; done\n",
                read.display(),
                go.display()
            ),
        )
        .expect("a stub find");
        std::fs::set_permissions(&find, std::fs::Permissions::from_mode(0o755)).expect("+x");
        let cargo = stub.join("cargo");
        std::fs::write(
            &cargo,
            "#!/bin/sh\nmkdir -p target/debug\nprintf x > target/debug/xtask\n\
             chmod 755 target/debug/xtask\n",
        )
        .expect("a stub cargo");
        std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755)).expect("+x");

        // The note names the gate that was live when the stale one was
        // admitted; the run that comes later will move it.
        let note = a_note(&root, 11);
        let held = std::path::PathBuf::from(super::dir(&root));
        std::fs::create_dir_all(&held).expect("the runner directory");
        std::fs::write(held.join("xtask-1700-99"), "x").expect("a dead run's copy");

        let line = line(&root, "1700-11", &note, None);
        let (root_for, stub_for) = (root.to_path_buf(), stub.clone());
        let stale = std::thread::spawn(move || ran(&line, &root_for, &stub_for));
        crate::wait::until(
            "the stale preparation's reading",
            || read.exists(),
            |listed| *listed,
        );

        // Now the run that comes after it: the note moves to its pid and
        // it installs a copy the stale one's listing never saw.
        a_note(&root, 22);
        let later = held.join("xtask-9999-22");
        std::fs::write(&later, "x").expect("the next run's copy");

        std::fs::write(&go, "").expect("let the stale one finish");
        let out = stale.join().expect("the stale preparation");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            later.exists(),
            "the stale preparation reached a copy installed after it read the directory"
        );
        assert!(
            std::path::Path::new(&at(&root, "1700-11")).is_file(),
            "the stale one still installed its own, which is the litter it is allowed"
        );
    }

    /// A tree, with the gate note and lock that `lanes::sole` leaves
    /// there, held or not as the caller asks.
    fn a_tree(what: &str, holding: Option<u32>) -> (crate::yard::Yard, Option<std::fs::File>) {
        let root = crate::yard::Yard::new(&format!("owned-{what}"));
        let note = super::note_of(&root);
        std::fs::create_dir_all(note.parent().expect("target")).expect("a tree");
        let lock = holding.map(|pid| {
            std::fs::write(
                &note,
                crate::still::Note::parse(&format!("pid {pid}\nsince 1\nwhat gate\n"))
                    .expect("a note")
                    .text(),
            )
            .expect("the note");
            let file = std::fs::File::options()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(note.with_extension("lock"))
                .expect("the lock");
            file.lock().expect("held");
            file
        });
        (root, lock)
    }

    /// A lock nobody holds is a gate that is over, whatever note stands
    /// beside it ([`owned_by_the_gate`]).
    #[test]
    fn a_preparation_runs_only_under_the_live_gate_that_named_it() {
        let (held, lock) = a_tree("held", Some(4242));
        assert!(super::owned_by_the_gate(&held, 4242).is_ok());
        let wrong = super::owned_by_the_gate(&held, 9999).expect_err("another pid is refused");
        assert!(
            wrong.contains("4242"),
            "it names the gate that holds it: {wrong}"
        );
        // Unlocked, not just dropped: a forked child elsewhere in the
        // suite may hold the description
        // (.claude/rules-refs/core.md「lock は unlock で離す」).
        if let Some(lock) = &lock {
            lock.unlock().expect("the holder lets go");
        }
        drop(lock);
        // The note is still there and still says 4242; the lock is not
        // held, so the gate is over and nothing here may clear anything.
        let over = super::owned_by_the_gate(&held, 4242).expect_err("a freed lock is no gate");
        assert!(over.contains("no gate is running"), "{over}");

        // A tree no gate ever ran in has no lock file at all.
        let (fresh, _) = a_tree("fresh", None);
        assert!(
            super::owned_by_the_gate(&fresh, 1).is_err(),
            "a tree with no gate refuses"
        );
    }

    /// A line carrying cargo's own options is a cargo line.
    #[test]
    fn only_the_task_runners_own_verbs_start_from_a_copy() {
        assert!(a_runner_verb(&words("verify-ui commit --preset basic")));
        assert!(a_runner_verb(&words("qmltest")));
        assert!(a_runner_verb(&words("demo-repo --preset basic")));
        assert!(!a_runner_verb(&words("test -p platitude-core")));
        assert!(!a_runner_verb(&words("clippy -p xtask --all-targets")));
        assert!(!a_runner_verb(&words("bare --discover")));
        assert!(!a_runner_verb(&words("--offline verify-ui commit")));
        assert!(!a_runner_verb(&[]));
    }
}
