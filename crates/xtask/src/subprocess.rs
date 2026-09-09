//! Running another program and reading what it said — the two shapes
//! every verb here uses, and the git one in particular.
//!
//! Not in main.rs, for the reason `tree` gives: a helper on the crate
//! root ties every module to every other in the gate's dependency graph.

/// Runs a command to completion, capturing output; errors carry context.
pub(crate) fn run_captured(
    cmd: &mut std::process::Command,
) -> Result<std::process::Output, String> {
    let display = format!("{cmd:?}");
    cmd.output()
        .map_err(|e| format!("failed to spawn {display}: {e}"))
}

/// Runs git in `dir`, answering its trimmed stdout with backslashes
/// forward, and None when it fails at all — callers treat a repository
/// that cannot answer as one they are not judging.
pub(crate) fn git_query(dir: &str, arguments: &[&str]) -> Option<String> {
    if dir.is_empty() {
        return None;
    }
    let mut command = std::process::Command::new("git");
    command.arg("-C").arg(dir).args(arguments);
    let output = run_captured(&mut command).ok()?;
    output.status.success().then(|| {
        String::from_utf8_lossy(&output.stdout)
            .trim()
            .replace('\\', "/")
    })
}

/// The repository `dir` belongs to, shared by all of its worktrees, so
/// that a merge run from a worktree is recognised as the same repository
/// as the session that runs it, and a hold taken beside it is seen from
/// every seat.
pub(crate) fn common_git_dir(dir: &str) -> Option<String> {
    git_query(
        dir,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
}

/// Whether the process `pid` names still exists. A probe that could not
/// answer says "alive": every caller asks this to decide whether somebody
/// else's claim may be broken, and breaking a live one costs more than
/// leaving a dead one standing.
#[cfg(windows)]
pub(crate) fn process_exists(pid: u32) -> bool {
    // Through the same three-answer probe as everything else here: a
    // question that could not be asked reads as alive, never as gone
    // ([`listed_by_tasklist`]).
    !matches!(behind(pid), Behind::Nobody)
}

/// The same, where a signal-less kill is the question.
#[cfg(not(windows))]
pub(crate) fn process_exists(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|status| status.success())
        .unwrap_or(true)
}

/// What is behind a pid right now, in the three answers a probe can
/// honestly give. A pid is a name the machine hands out again the moment
/// its process is gone, so a claim a killed process left behind would
/// otherwise stand for as long as whatever inherited the number — a git,
/// a browser tab — and hold what it claimed until that stranger exits.
/// The image name tells the two apart.
enum Behind {
    /// The program at that number, as this probe spells it.
    Named(String),
    /// The probe answered, and nobody is at that number.
    Nobody,
    /// The probe could not be asked at all.
    Unanswerable,
}

/// Who holds `pid`, asked of `tasklist`.
#[cfg(windows)]
fn behind(pid: u32) -> Behind {
    let Ok(out) = std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
        .output()
    else {
        return Behind::Unanswerable;
    };
    listed_by_tasklist(
        out.status.success(),
        &String::from_utf8_lossy(&out.stdout),
        &String::from_utf8_lossy(&out.stderr),
        pid,
    )
}

/// What `tasklist` said, as one of the three answers.
///
/// **A probe that could not ask is not a process that is gone.** The
/// tool answers a pid nobody has with a line saying so and an exit of
/// zero, so an empty result on its own means "nobody" — but a tool that
/// was refused, or is not there, or is restricted by policy, also
/// returns nothing that matches, and reading *that* as nobody hands out
/// a running process's claim. Every claim here is broken on the strength
/// of this answer, so the two are told apart: a non-zero exit, or
/// anything said on the error stream, is a question that did not get
/// asked.
#[cfg(windows)]
fn listed_by_tasklist(ok: bool, stdout: &str, stderr: &str, pid: u32) -> Behind {
    if !ok || !stderr.trim().is_empty() {
        return Behind::Unanswerable;
    }
    // `"xtask.exe","12345","Console","1","75,836 K"`: the image and the
    // pid are the first two fields, ahead of the one holding a comma.
    stdout
        .lines()
        .find_map(|line| {
            let mut fields = line.split(',').map(|field| field.trim().trim_matches('"'));
            let image = fields.next().unwrap_or_default().to_string();
            (fields.next().unwrap_or_default() == pid.to_string()).then_some(image)
        })
        .map_or(Behind::Nobody, Behind::Named)
}

/// The same, asking `ps` for the state and the command name.
///
/// **A zombie is nobody.** It has already exited and holds no processor,
/// no memory and no lock; what keeps its number in the table is that
/// whoever started it has not waited on it — and where the parent was
/// killed, nobody ever will unless the system's first process reaps
/// (inside a container that process is the command the container was
/// started with, and cargo reaps nothing it did not start). Read as
/// alive, such a number would hold a machine claim for as long as the
/// container lived.
#[cfg(not(windows))]
fn behind(pid: u32) -> Behind {
    let Ok(out) = std::process::Command::new("ps")
        .args(["-o", "stat=,comm=", "-p", &pid.to_string()])
        .output()
    else {
        return Behind::Unanswerable;
    };
    // A pid nobody has answers with nothing and a non-zero exit.
    let answer = String::from_utf8_lossy(&out.stdout);
    let answer = answer.trim();
    if !out.status.success() || answer.is_empty() {
        return Behind::Nobody;
    }
    let mut fields = answer.split_whitespace();
    let state = fields.next().unwrap_or_default();
    let name = fields.next().unwrap_or_default();
    if state.starts_with('Z') || name.is_empty() {
        return Behind::Nobody;
    }
    Behind::Named(name.to_string())
}

/// Whether the process `pid` names still exists **and is a program
/// `is_wanted` accepts** — the shape every claim that records its writer
/// is asked with. Answers "alive" when it could not ask, as
/// [`process_exists`] does: every caller asks this to decide whether
/// somebody else's claim may be broken.
fn image_matches(pid: u32, is_wanted: impl Fn(&str) -> bool) -> bool {
    match behind(pid) {
        Behind::Named(image) => is_wanted(&image),
        Behind::Nobody => false,
        Behind::Unanswerable => true,
    }
}

/// The program behind `pid`, for a claim to record beside the number it
/// is claiming from. None when nothing could be read, which leaves the
/// claim naming a bare number — the shape claims had before this, and
/// the one [`process_exists`] is the whole answer for.
///
/// The recording and the reading go through the same probe on purpose:
/// what a claim keeps is not the writer's idea of its own name but the
/// string this machine will hand back at that number, so the two are
/// comparable however the program was installed or spelled.
pub(crate) fn image_of(pid: u32) -> Option<String> {
    match behind(pid) {
        Behind::Named(image) => Some(image),
        Behind::Nobody | Behind::Unanswerable => None,
    }
}

/// Whether anything is still *running* at `pid` — for a claim that has
/// no name to compare against and only wants to know whether the work
/// is still on the machine (`budget::Pool::leftover`). Stricter than
/// [`process_exists`] in the one way that matters there: a process that
/// has exited and not been waited on holds nothing, and [`behind`] reads
/// it as nobody. Answers "alive" when it could not ask, as every probe
/// here does.
pub(crate) fn running_at(pid: u32) -> bool {
    image_matches(pid, |_| true)
}

/// Whether `pid` still names this task runner — for the claims nothing
/// but the runner ever writes (a verify-ui run's hold on a repository, a
/// shot directory, a config directory). The runner is the one program
/// this may name outright: it is asking after itself.
pub(crate) fn task_runner_exists(pid: u32) -> bool {
    image_matches(pid, is_task_runner)
}

/// Whether the program a claim recorded is still the one at `pid` — for
/// the claims written from somebody else's process, whose name this has
/// no business knowing (`seats::SEAT_CLAIM`, out of a session's
/// `CLAUDE_PID`). The claim carries the answer; this only compares.
pub(crate) fn image_still_at(pid: u32, recorded: &str) -> bool {
    image_matches(pid, |image| same_image(image, recorded))
}

/// When the process at a pid began, in the three answers a probe can
/// honestly give. A number and the program behind it still name two
/// processes where the successor is installed as the same program, which
/// is every claim on this machine: a Claude session's number goes to the
/// next `claude.exe`, and the image name reads as a match. The instant
/// the process began is what tells those apart.
enum Born {
    /// When the process at that number began, as this probe spells it.
    At(String),
    /// The probe answered, and nobody is at that number.
    Nobody,
    /// The probe could not be asked, or was not allowed to say.
    Unanswerable,
}

/// When `pid` began, asked of PowerShell: `tasklist` carries no clock and
/// `wmic` is on its way off the system (`reap::snapshot`). A number
/// nobody holds says so; a process this user may not be told about
/// throws, and that is not the same answer.
#[cfg(windows)]
fn born(pid: u32) -> Born {
    let script = format!(
        "$p=Get-Process -Id {pid} -ErrorAction SilentlyContinue;\
         if($null -eq $p){{'none'}}else{{try{{$p.StartTime.Ticks}}catch{{''}}}}"
    );
    let mut command = std::process::Command::new("powershell");
    command.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
    let Ok(out) = run_captured(&mut command) else {
        return Born::Unanswerable;
    };
    match String::from_utf8_lossy(&out.stdout).trim() {
        "none" => Born::Nobody,
        ticks if !ticks.is_empty() && ticks.bytes().all(|b| b.is_ascii_digit()) => {
            Born::At(ticks.to_string())
        }
        _ => Born::Unanswerable,
    }
}

/// The same, out of `/proc` where the machine keeps one and `ps` where it
/// does not. A missing `/proc` entry is an answer only where `/proc` is
/// the register of processes — on macOS there is none to be missing from,
/// and reading its absence as a corpse would call every claim litter.
#[cfg(not(windows))]
fn born(pid: u32) -> Born {
    match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => started_field(&stat).map_or(Born::Unanswerable, Born::At),
        Err(_) if std::path::Path::new("/proc/self/stat").exists() => Born::Nobody,
        Err(_) => born_from_ps(pid),
    }
}

/// Field 22 of `/proc/<pid>/stat`, the clock ticks since boot at which the
/// process began. The command name is field 2 and may hold spaces and
/// parentheses of its own, so the fields are counted from the last `)`.
#[cfg(not(windows))]
fn started_field(stat: &str) -> Option<String> {
    let (_, after_command) = stat.rsplit_once(')')?;
    after_command.split_whitespace().nth(19).map(str::to_string)
}

/// When `pid` began as `ps` renders it, for the machines with no `/proc`.
/// The spacing is column padding, so it is collapsed: what a claim keeps
/// has to compare equal to what a later read gets back.
///
/// A `ps` that has no `lstart` to give fails the same way a pid nobody
/// holds does — nothing on stdout, and a non-zero exit — and reading that
/// as a corpse would call every live claim on such a machine litter. What
/// separates them is that a refused format says so on stderr.
#[cfg(not(windows))]
fn born_from_ps(pid: u32) -> Born {
    let Ok(out) = std::process::Command::new("ps")
        .args(["-o", "lstart=", "-p", &pid.to_string()])
        .output()
    else {
        return Born::Unanswerable;
    };
    if !out.stderr.is_empty() {
        return Born::Unanswerable;
    }
    let rendered = String::from_utf8_lossy(&out.stdout);
    let collapsed = rendered.split_whitespace().collect::<Vec<_>>().join(" ");
    match (out.status.success(), collapsed.is_empty()) {
        (true, false) => Born::At(collapsed),
        _ => Born::Nobody,
    }
}

/// When the process behind `pid` began, for a claim to record beside the
/// number and the program. None when nothing could be read, which leaves
/// the claim naming what it named before — the shape claims had until
/// this, and the one [`image_still_at`] is the whole answer for.
pub(crate) fn born_of(pid: u32) -> Option<String> {
    match born(pid) {
        Born::At(when) => Some(when),
        Born::Nobody | Born::Unanswerable => None,
    }
}

/// Whether the process a claim was written from is still the one at
/// `pid`: `Some(false)` for a stranger handed the number after it, one
/// installed as the same program included, and None where the machine
/// would not say — which leaves the caller the older, weaker question
/// rather than an answer this did not have.
pub(crate) fn born_still_at(pid: u32, recorded: &str) -> Option<bool> {
    if recorded.is_empty() {
        return None;
    }
    match born(pid) {
        Born::At(when) => Some(when == recorded),
        Born::Nobody => Some(false),
        Born::Unanswerable => None,
    }
}

/// Whether two spellings name one program. The tolerance is a probe's
/// own drift and a launcher's — a path where a bare name was expected,
/// Windows' indifference to case, and the suffix the loader adds to a
/// program a caller spelled without one ([`stem`]) — and not a guess at
/// what any particular program is called.
fn same_image(image: &str, recorded: &str) -> bool {
    !recorded.is_empty() && stem(image).eq_ignore_ascii_case(stem(recorded))
}

/// Whether an image name is this runner's, however it is spelled: cargo's
/// `xtask.exe`, a test binary's `xtask-<hash>`, a landing's
/// `xtask-inflight-<pid>` (`land::step_out_of_the_build_slot`).
fn is_task_runner(image: &str) -> bool {
    basename(image)
        .get(..5)
        .is_some_and(|head| head.eq_ignore_ascii_case("xtask"))
}

/// An image name with whatever path a probe put in front of it taken off.
fn basename(image: &str) -> &str {
    image.rsplit(['/', '\\']).next().unwrap_or(image)
}

/// The same, with the executable suffix off as well — because the two
/// sides of a comparison are not always written by the same hand. A
/// claim that records what a *launcher* spelled has `cargo` or `docker`
/// where the machine hands back the image it loaded, `cargo.exe`
/// (`budget::Pool::leftover`). A name that is nothing but a suffix is
/// left whole: two dotfiles are not one program.
fn stem(image: &str) -> &str {
    let base = basename(image);
    match base.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem,
        _ => base,
    }
}

/// A program a caller spelled itself, written in the vocabulary a probe
/// on this machine answers in — for a claim that has to record a name
/// without paying a process for it ([`image_of`] costs one, and the
/// budget records a name on every step of a gate).
///
/// **Linux keeps only the first fifteen characters of a program's name**
/// (`TASK_COMM_LEN` less its terminator), so a gate's runner copy is
/// `xtask-runner-12345` to the launcher and `xtask-runner-1` to `ps` —
/// and a claim that recorded the whole of it would read as a stranger's
/// on every step. Nothing else here truncates: Windows hands the image
/// name back whole and macOS hands back a path, and [`same_image`] takes
/// the path and the suffix off either side.
pub(crate) fn as_probed(program: &str) -> String {
    let stem = stem(program);
    if !cfg!(target_os = "linux") {
        return stem.to_string();
    }
    const COMM: usize = 15;
    stem.char_indices()
        .nth(COMM)
        .map_or(stem, |(at, _)| &stem[..at])
        .to_string()
}

/// A pid no process on this machine can carry — for the tests that need a
/// claim whose process is gone. Spawning a child and reaping it hands its
/// number back to the kernel, which gives it out again; under a suite
/// forking git on every thread the number is somebody else's before the
/// assertion runs, and the test meets a live stranger where it looked for
/// a corpse. Windows hands out multiples of four only, and Linux stops at
/// `pid_max`, which is 2^22 at the most: odd, and above both, so nobody's.
#[cfg(test)]
pub(crate) const NO_SUCH_PID: u32 = 0x7FFF_FFFD;

#[cfg(test)]
mod tests {
    use super::{
        NO_SUCH_PID, born_of, born_still_at, image_of, image_still_at, is_task_runner,
        process_exists, same_image, task_runner_exists,
    };

    /// A probe that was refused is not a process that is gone. Every
    /// claim on this machine is broken on the strength of this answer —
    /// a seat's, a run's, and the room a unit is holding on the
    /// machine's budget — so a `tasklist` that could not answer must
    /// come back as the third answer rather than as "nobody".
    #[test]
    #[cfg(windows)]
    fn a_refused_listing_is_not_a_process_that_is_gone() {
        use super::{Behind, listed_by_tasklist};
        let named = |seen: Behind| matches!(seen, Behind::Named(_));
        assert!(
            named(listed_by_tasklist(
                true,
                "\"xtask.exe\",\"1234\",\"Console\",\"1\",\"75,836 K\"\n",
                "",
                1234
            )),
            "the ordinary answer"
        );
        assert!(
            matches!(
                listed_by_tasklist(
                    true,
                    "INFO: No tasks are running which match the specified criteria.\n",
                    "",
                    1234
                ),
                Behind::Nobody
            ),
            "a pid nobody has is answered, and the answer is nobody"
        );
        for (ok, stdout, stderr) in [
            (false, "", "ERROR: Access is denied.\n"),
            (false, "", ""),
            (true, "", "ERROR: The RPC server is unavailable.\n"),
        ] {
            assert!(
                matches!(
                    listed_by_tasklist(ok, stdout, stderr, 1234),
                    Behind::Unanswerable
                ),
                "a refused listing read as a process that is gone ({ok}, {stderr:?})"
            );
        }
    }

    #[test]
    fn the_runner_is_known_by_its_image_name_however_cargo_spelled_it() {
        for image in [
            "xtask.exe",
            "XTASK.EXE",
            "xtask",
            "xtask-4aadee3d869c602a.exe",
            "xtask-inflight-31336.exe",
            "C:\\x\\target\\debug\\xtask.exe",
            "/x/target/debug/xtask",
        ] {
            assert!(is_task_runner(image), "{image}");
        }
        for image in [
            "git.exe",
            "ping.exe",
            "cargo.exe",
            "",
            "xtas",
            "notxtask.exe",
        ] {
            assert!(!is_task_runner(image), "{image}");
        }
    }

    /// Two recordings of one program are one program, and nothing here
    /// knows what any of them is called — which is the point: whatever a
    /// session is installed as, its claim records that and this compares
    /// it. An empty recording is not a match with anything, so a claim
    /// that recorded nothing cannot be read as naming what it met.
    #[test]
    fn one_program_is_known_by_the_name_the_claim_recorded() {
        for (image, recorded) in [
            ("claude.exe", "claude.exe"),
            ("CLAUDE.EXE", "claude.exe"),
            ("node", "node"),
            ("node.exe", "C:\\Program Files\\nodejs\\node.exe"),
            ("/usr/local/bin/claude", "claude"),
            // What a launcher spelled against what the loader ran: the
            // budget's leftover records the program it started, and the
            // machine hands the image back with the suffix on.
            ("cargo.exe", "cargo"),
            ("docker.exe", "docker"),
        ] {
            assert!(same_image(image, recorded), "{image} vs {recorded}");
        }
        for (image, recorded) in [
            ("node.exe", "claude.exe"),
            ("git.exe", "claude.exe"),
            ("claude.exe", ""),
            ("", ""),
            ("claude.exe", "claude-code.exe"),
            // Nothing but a suffix: two of those are not one program.
            (".bashrc", ".profile"),
        ] {
            assert!(!same_image(image, recorded), "{image} vs {recorded}");
        }
    }

    /// This process is the runner's own test binary, and the pid no
    /// process can have is nobody's — whichever probe is asked. A live
    /// number is not by itself an answer: the runner reads as the runner
    /// and not as whatever else a claim recorded, which is the whole of
    /// what the image name adds.
    #[test]
    fn this_process_is_alive_and_the_pid_nobody_can_have_is_not() {
        let me = std::process::id();
        let mine = image_of(me).expect("this process is behind its own pid");
        assert!(is_task_runner(&mine), "{mine}");
        assert!(process_exists(me));
        assert!(task_runner_exists(me));
        assert!(image_still_at(me, &mine));
        assert!(!image_still_at(me, "claude.exe"));
        assert!(!process_exists(NO_SUCH_PID));
        assert!(!task_runner_exists(NO_SUCH_PID));
        assert!(!image_still_at(NO_SUCH_PID, &mine));
        assert_eq!(image_of(NO_SUCH_PID), None);
    }

    /// This process is the one whose beginning this can ask after, and
    /// the answer is what a claim written here would record. An instant
    /// that is not the one at a live number belongs to the process that
    /// held the number before — the case the image name cannot see, both
    /// sides of it being one program. A claim that recorded nothing is
    /// not answered with a guess: it is left unasked.
    #[test]
    fn a_process_is_told_from_its_successor_by_when_it_began() {
        let me = std::process::id();
        let born = born_of(me).expect("this process began at some point");
        assert!(!born.is_empty());
        assert_eq!(born_still_at(me, &born), Some(true));
        assert_eq!(born_still_at(me, "1"), Some(false));
        assert_eq!(born_still_at(me, ""), None);
        assert_eq!(born_of(NO_SUCH_PID), None);
        assert_eq!(born_still_at(NO_SUCH_PID, &born), Some(false));
    }

    /// The command name sits in parentheses and may hold spaces and
    /// parentheses of its own, so a reader counting fields from the left
    /// lands on the wrong one and dates the process from a page count.
    #[cfg(not(windows))]
    #[test]
    fn reads_when_a_process_began_out_of_its_proc_line() {
        let stat = "42 ((odd) name) S 1 42 42 0 -1 4194560 100 0 0 0 1 2 0 0 20 0 1 0 987654 0 0";
        assert_eq!(super::started_field(stat).as_deref(), Some("987654"));
        assert_eq!(super::started_field("nothing of the sort"), None);
    }
}
