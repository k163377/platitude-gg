//! The gate's own container: the one the Linux side's verbs go into,
//! by `docker exec`, instead of a container apiece.
//!
//! A full gate's Linux side is five hundred verbs, and each one was a
//! container of its own: created, given a network namespace and a
//! cgroup, run, torn down. The verbs start from one prepared copy
//! already (`runner`); this puts them into one container as well,
//! owned by the gate that prepared the copy and taken down when that
//! gate's side is over. Everything that is *not* one of the copy's
//! verbs — the cargo steps, `bare`, `offline` — keeps a container of
//! its own, because those are different images, different mounts, or
//! a different network.
//!
//! **What is shared and what is not.** Every verb of a side has the
//! same mounts anyway (the checkout, the build volume, the registry,
//! the demo volume), so the container carries those once. What a verb
//! has of its own is its environment and its `/out`: the exec hands
//! each verb its own marks and its own leaf under a mount of the whole
//! `pgg-linux` directory, so two verbs never see one settings store
//! (`keepsakes::Landing::Leaf`). `/tmp` is one for the whole container
//! — as it already was across containers for the demo volume — and
//! what stands in it is claimed by liveness (`verify::ownership`), so
//! a leaf of one run is never handed to another. `HOME` is the image's
//! and is not written: git reads `GIT_CONFIG_GLOBAL`, the app reads
//! `PGG_CONFIG_DIR`, both under the verb's own `/out` leaf.
//!
//! **A process is addressed by its mark, never by a pid.** Every
//! process a verb starts inherits `PGG_STEP=<mark>` from the exec, so
//! a stop is a walk over `/proc` for that mark: it needs no pid file,
//! no process group and no notion of who started what, and it cannot
//! reach another verb's processes. The verb's own wrapper takes its
//! leftovers the same way as it ends, from a shell started without the
//! mark, so that the walk is not in the walk ([`MARKED_FN`]).
//!
//! **The container's life has a ceiling of its own** ([`IDLE_SCRIPT`]).
//! The gate takes it down when its side is over
//! ([`remove_container`]), and the next gate in this tree takes down
//! whatever an earlier one left ([`take_down_earlier_gates`]); but a
//! gate can be killed from outside with nothing of its own left
//! running, so the container's own process leaves when no verb has
//! been in for a while and none is running, and says so in the gate's
//! log directory. A verb's mark counts as running for the step ceiling
//! and no longer, so a verb that never ends cannot hold the container
//! up forever either.
//!
//! **Every docker command out here is bounded**
//! (`subprocess::bounded_both_streams`): a start, a listing, a stop and
//! a removal each have a ceiling and a file for what docker said on
//! either stream, and what did not happen is said in the gate's own
//! words — never a container quietly left, never a `docker run` in
//! place of an exec that failed.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use super::runner::{note_of, spelled};
use crate::subprocess::{Answer, bounded_both_streams};

/// The mark every process of a verb carries inside the gate's container
/// — the address a stop is sent to.
pub(crate) const STEP_MARK: &str = "PGG_STEP";

/// The labels the gate's container carries: whose tree, which gate
/// (its pid, as the tree's note spells it), which run.
const TREE_LABEL: &str = "pgg.tree";
const GATE_LABEL: &str = "pgg.gate";
const RUN_LABEL: &str = "pgg.run";

/// Where the gate's log directory is mounted, so the container's own
/// process can say why it left where a reader looks for the gate's
/// logs. Not under `/work`, which the build volume covers.
const LOGS_MOUNT: &str = "/pgg-gate-logs";

/// The container's own log in that directory, and the file the host
/// side of a start writes docker's answer to.
const CONTAINER_LOG: &str = "linux-container.log";

/// Where the container keeps the marks of the verbs in it (`steps/`)
/// and the time a verb was last in (`live`). Inside the container, so
/// nothing of it crosses a mount. Every script takes it as an
/// argument, so a test can run them somewhere it may write.
const STATE_DIR: &str = "/run/pgg-gate";

/// How long the container waits with no verb in it before it leaves
/// on its own. The verbs of a side follow each other by seconds; a
/// gate that is gone has none coming.
const IDLE: Duration = Duration::from_secs(5 * 60);

/// How long a docker command about the container may take out here.
/// A start is a create and an attach against a daemon that may be
/// wedged; a stop is an exec that walks `/proc`; a removal is a kill
/// and a teardown. All three are seconds when they work at all.
const START_CEILING: Duration = Duration::from_secs(120);
const STOP_CEILING: Duration = Duration::from_secs(30);
const REMOVE_CEILING: Duration = Duration::from_secs(60);

/// The gate's container for the run `name`, in this tree. Named for
/// the tree (as its volumes are) and the run, so two seats' gates and
/// two runs of one seat never meet at a name.
pub(crate) fn container_of(root: &Path, name: &str) -> String {
    format!(
        "{}-gate-{}-{name}",
        super::IMAGE,
        super::checkout_name(root)
    )
}

/// Starts the container the copy's verbs go into, and says so.
///
/// `--init`, so that what a verb leaves orphaned is reaped: the
/// container's own process is a shell that waits on nothing.
/// `--detach`, so no process out here holds it — its life is the
/// gate's removal, or its own idle ceiling ([`IDLE_SCRIPT`]).
pub(super) fn start_container(root: &Path, name: &str, gate: u32, tag: &str) -> Result<(), String> {
    let container = container_of(root, name);
    let out = crate::keepsakes::keepsake_base()?;
    let logs = root.join("target").join("gate-logs");
    std::fs::create_dir_all(&logs).map_err(|e| format!("{}: {e}", logs.display()))?;
    let mut cmd = Command::new("docker");
    cmd.args(["run", "--detach", "--rm", "--init", "--name", &container])
        .arg("--label")
        .arg(format!("{TREE_LABEL}={}", super::checkout_name(root)))
        .arg("--label")
        .arg(format!("{GATE_LABEL}={gate}"))
        .arg("--label")
        .arg(format!("{RUN_LABEL}={name}"));
    // The marks every process in there is under (`super::carried`), on
    // the container as on every exec into it: a road that carries one
    // and forgets the other counts the machine twice.
    super::marked(&mut cmd);
    cmd.arg("--volume")
        .arg(format!("{}:{}", super::mount_path(root), super::WORK))
        .arg("--volume")
        .arg(format!(
            "{}:{}",
            super::volume(root, "target"),
            super::TARGET_MOUNT
        ))
        .arg("--volume")
        .arg(format!(
            "{}-registry:{}",
            super::IMAGE,
            super::REGISTRY_MOUNT
        ))
        .arg("--volume")
        .arg(format!(
            "{}:{}",
            super::volume(root, "demo"),
            super::DEMO_MOUNT
        ))
        // The whole directory the verbs' leaves are claimed under, so a
        // verb's own leaf is there by the time its exec names it.
        .arg("--volume")
        .arg(format!("{}:{}", super::mount_path(&out), super::OUT_MOUNT))
        .arg("--volume")
        .arg(format!("{}:{LOGS_MOUNT}", super::mount_path(&logs)))
        .arg("--workdir")
        .arg(super::WORK)
        .arg("--env")
        .arg(format!("{}=1", super::IN_CONTAINER))
        .arg(tag)
        .args(["sh", "-c", IDLE_SCRIPT, "pgg-gate"])
        .arg(IDLE.as_secs().to_string())
        .arg((crate::check::STEP_CEILING.as_secs() / 60).to_string())
        .arg(format!("{LOGS_MOUNT}/{CONTAINER_LOG}"))
        .arg(STATE_DIR);
    let said = logs.join("linux-container.start.txt");
    // waits(measured): the start's cost, said on its line and judged by nothing
    let at = std::time::Instant::now();
    match bounded_both_streams("the gate's container", cmd, START_CEILING, &said) {
        Answer::Ended { status, stdout } if status.success() => {
            println!(
                "the gate's container: {container} ({}) up in {:.1}s",
                stdout.trim().chars().take(12).collect::<String>(),
                at.elapsed().as_secs_f32()
            );
            Ok(())
        }
        Answer::Ended { status, stdout } => Err(format!(
            "the gate's container {container} did not start (docker exited {:?}): {}",
            status.code(),
            stdout.trim()
        )),
        Answer::OutOfTime { pid, after, ended } => Err(format!(
            "the gate's container {container} did not start within {:.0}s — docker (pid {pid}) \
             was {}",
            after.as_secs_f32(),
            match ended {
                Ok(()) => "ended".to_string(),
                Err(why) => format!("not ended: {why}"),
            }
        )),
        Answer::Unstarted(why) => Err(format!("the gate's container {container}: {why}")),
    }
}

/// The pid the tree's gate note names now, or why it cannot be read.
/// Read beside each decision, never once at the top, for the reason
/// the copy's emptying (`runner::SCRIPT`) reads it late: the moment a
/// preparation started is the moment not to be trusted.
fn live_gate(root: &Path) -> Result<u32, String> {
    let note = note_of(root);
    std::fs::read_to_string(&note)
        .ok()
        .and_then(|text| crate::still::Note::parse(&text))
        .map(|note| note.pid())
        .ok_or_else(|| format!("no pid to spare in {}", note.display()))
}

/// Takes down the containers earlier gates of this tree left behind —
/// a gate killed from outside before its removal, an idle ceiling not
/// yet reached — sparing the one whose gate the tree's note names now.
///
/// **Bounded by what it can name**: only containers labelled for this
/// tree, and only those whose gate is not the live one. A note that
/// yields no pid stops this, as it stops the copy's emptying, because
/// an emptying that cannot say what to spare must spare everything.
pub(super) fn take_down_earlier_gates(root: &Path) -> Result<(), String> {
    let logs = root.join("target").join("gate-logs");
    let said = logs.join("linux-container.list.txt");
    let mut list = Command::new("docker");
    list.args([
        "ps",
        "--all",
        "--filter",
        &format!("label={TREE_LABEL}={}", super::checkout_name(root)),
        "--format",
        &format!("{{{{.Names}}}}\t{{{{.Label \"{GATE_LABEL}\"}}}}\t{{{{.Status}}}}"),
    ]);
    let listed =
        match bounded_both_streams("listing this tree's containers", list, STOP_CEILING, &said) {
            Answer::Ended { status, stdout } if status.success() => stdout,
            Answer::Ended { status, stdout } => {
                return Err(format!(
                    "docker ps exited {:?} listing this tree's containers: {}",
                    status.code(),
                    stdout.trim()
                ));
            }
            Answer::OutOfTime { after, .. } => {
                return Err(format!(
                    "docker ps did not list this tree's containers within {:.0}s",
                    after.as_secs_f32()
                ));
            }
            Answer::Unstarted(why) => return Err(why),
        };
    for line in listed.lines() {
        let mut fields = line.split('\t');
        let (Some(name), Some(gate), status) = (fields.next(), fields.next(), fields.next()) else {
            continue;
        };
        let live = live_gate(root)?;
        if gate == live.to_string() {
            println!("left {name} alone: the live gate's ({live})");
            continue;
        }
        let line = remove_container(root, name);
        println!(
            "an earlier gate's container ({}): {line}",
            status.unwrap_or("status unknown")
        );
    }
    Ok(())
}

/// Removes the gate's container, and answers a line saying what
/// became of it — never an error: a removal that did not happen is
/// said, with what to do about it, and the gate's verdict is about
/// its steps.
pub(crate) fn remove_container(root: &Path, name: &str) -> String {
    // Under the container's name, so a sweep of several keeps every
    // answer docker gave.
    let said = root
        .join("target")
        .join("gate-logs")
        .join(format!("linux-remove-{name}.txt"));
    let mut rm = Command::new("docker");
    rm.args(["rm", "--force", name]);
    // waits(measured): the removal's cost, said on its line and judged by nothing
    let at = std::time::Instant::now();
    match bounded_both_streams("removing the gate's container", rm, REMOVE_CEILING, &said) {
        Answer::Ended { status, .. } if status.success() => {
            format!("{name} removed in {:.1}s", at.elapsed().as_secs_f32())
        }
        // Not there: it left on its own idle ceiling (`--rm` takes it
        // with it, and one leaving at this very moment answers "already
        // in progress"), or it was never started because the
        // preparation stopped before it.
        Answer::Ended { stdout, .. }
            if stdout.contains("No such container") || stdout.contains("already in progress") =>
        {
            format!("{name} is not there (gone on its own, or never started)")
        }
        Answer::Ended { status, stdout } => format!(
            "{name} was not removed (docker exited {:?}: {}) — the next gate in this tree \
             takes it down, and `docker rm --force {name}` does now",
            status.code(),
            stdout.trim()
        ),
        Answer::OutOfTime { after, .. } => format!(
            "{name} was not removed within {:.0}s — it leaves on its own once no verb has \
             been in it for {}s, and the next gate in this tree takes it down",
            after.as_secs_f32(),
            IDLE.as_secs()
        ),
        Answer::Unstarted(why) => format!("{name} was not removed: {why}"),
    }
}

/// Starts `command` — a prepared copy and its verb — inside the gate's
/// container `container`, every process of it carrying `mark`.
///
/// The line is the one [`super::in_container`] would run in a
/// container of its own, with the container's creation taken out:
/// the same marks, the same `/work`, the copy's guard, and the verb's
/// pictures out through `/out` — as a leaf of it, since the mount is
/// the whole directory ([`crate::keepsakes::Landing::Leaf`]).
pub(crate) fn exec_in(container: &str, command: &[String], mark: &str) -> Result<(), String> {
    let mut inside = command.to_vec();
    let keepsake = crate::keepsakes::bridge(
        &mut inside,
        crate::keepsakes::Landing::Leaf(super::OUT_MOUNT),
    )?;
    let mut cmd = exec_line(container, &inside, mark)?;
    // Through the budget's runner, as a container of the verb's own is:
    // the process in there goes on when this launcher is killed, and
    // what then reaches it is its mark (`stop_step`).
    let status =
        crate::budget::watched(&mut cmd).map_err(|e| format!("failed to run docker: {e}"))?;
    if status.success() {
        crate::keepsakes::onto_the_board(keepsake.as_ref().map(|out| out.dir.as_path()), command);
        return Ok(());
    }
    Err(match status.code() {
        Some(code) => format!("the run in {container} exited {code}"),
        None => format!("the run in {container} was killed"),
    })
}

/// The `docker exec` for `inside` — the copy and its arguments, the
/// verb's `/out` leaf already on it — in `container`, marked `mark`.
/// Pure but for the terminal check, so a test can read the line.
pub(super) fn exec_line(container: &str, inside: &[String], mark: &str) -> Result<Command, String> {
    let Some(copy) = inside.first() else {
        return Err("nothing to run".into());
    };
    let mut cmd = Command::new("docker");
    cmd.arg("exec");
    if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        cmd.arg("--interactive").arg("--tty");
    }
    super::marked(&mut cmd);
    cmd.arg("--env")
        .arg(format!("{}=1", super::IN_CONTAINER))
        .arg("--env")
        .arg(format!("{STEP_MARK}={mark}"))
        .arg("--workdir")
        .arg(super::WORK)
        .arg(container)
        .args(["sh", "-c", &exec_script(copy, mark, STATE_DIR), "pgg-step"])
        // The leavings walk, as the wrapper's first argument: text it
        // hands to a shell of its own once the verb is over
        // ([`exec_script`]).
        .arg(leftovers_script())
        .args(inside);
    Ok(cmd)
}

/// Ends every process in `container` carrying `mark`, and answers what
/// that came to. For a step ended at its ceiling out here: the host
/// side of it is reaped by the runner, and this is the other side.
///
/// `Ok` is a walk that ran and left nothing carrying the mark; `Err`
/// is the same line for a walk that left something, could not run, or
/// did not come back — so a typed stop can exit as it found things,
/// while a step's failure stays what it was whichever came back.
pub(crate) fn stop_step(container: &str, mark: &str, logs: &Path) -> Result<String, String> {
    if spelled(mark).is_err() {
        return Err(format!("{mark:?} is not a mark this addresses"));
    }
    // The stop's whole account is kept here; the line answered is its
    // last, which counts what went and what would not.
    let said = logs.join(format!("linux-stop-{mark}.txt"));
    let account = format!("(the stop's account: {})", said.display());
    let mut stop = Command::new("docker");
    stop.args([
        "exec",
        container,
        "sh",
        "-c",
        &stop_script(),
        "pgg-stop",
        mark,
        STATE_DIR,
    ]);
    match bounded_both_streams(
        "stopping the step in the container",
        stop,
        STOP_CEILING,
        &said,
    ) {
        Answer::Ended { status, stdout } if status.success() => {
            let last = stdout
                .lines()
                .last()
                .unwrap_or("the stop said nothing")
                .to_string();
            // `… took N process(es), M left` — M is what would not go.
            let left = last
                .rsplit(' ')
                .nth(1)
                .and_then(|count| count.parse::<u32>().ok());
            let line = format!("{last} {account}");
            match left {
                Some(0) => Ok(line),
                _ => Err(line),
            }
        }
        Answer::Ended { status, stdout } => Err(format!(
            "the stop exited {:?}: {} {account}",
            status.code(),
            stdout.trim().replace('\n', " / ")
        )),
        Answer::OutOfTime { after, .. } => Err(format!(
            "the stop did not come back within {:.0}s — what carries {mark} in {container} may \
             still be running {account}",
            after.as_secs_f32()
        )),
        Answer::Unstarted(why) => Err(format!("{why} {account}")),
    }
}

/// A shell function that lists the processes carrying `$1` as their
/// mark — pid and name — this shell aside.
///
/// **The shell that walks must have been started without the mark.**
/// `/proc/<pid>/environ` is the block a process was *started* with:
/// `unset` does not touch it, and a subshell forked for the `|` or the
/// `$( )` carries the parent's block whole. A marked shell that walked
/// would list its own pipeline and kill it (measured: the walk in
/// every green verb took one `sh` with it — its own). So the walker is
/// always an `env -u` exec of a fresh shell ([`exec_script`]), or a
/// `docker exec` that never had the mark ([`stop_step`]).
const MARKED_FN: &str = "marked() {\n\
     \x20 for p in /proc/[0-9]*; do\n\
     \x20 \x20 pid=${p#/proc/}\n\
     \x20 \x20 [ \"$pid\" = \"$$\" ] && continue\n\
     \x20 \x20 if tr '\\0' '\\n' < \"$p/environ\" 2>/dev/null | grep -qxF \"PGG_STEP=$1\"; then\n\
     \x20 \x20 \x20 echo \"$pid $(cat \"$p/comm\" 2>/dev/null)\"\n\
     \x20 \x20 fi\n\
     \x20 done\n\
     }\n";

/// What a verb runs as inside the gate's container: the copy's guard,
/// then the verb, then its leavings. `$1` is the leavings walk
/// ([`leftovers_script`]) and the rest is the copy and its arguments,
/// as [`super::watched_from_inside`] takes them.
///
/// The step's mark file stands for as long as the verb runs, and the
/// live file is touched at both ends: those two are what the
/// container's own process reads to know whether it is still wanted
/// ([`IDLE_SCRIPT`]). The leavings are walked by a shell `exec`ed
/// without the mark in its environment — the one way the walk cannot
/// meet itself ([`MARKED_FN`]) — which carries the verb's own exit
/// code out.
fn exec_script(copy: &str, mark: &str, state: &str) -> String {
    format!(
        "leftovers=$1\nshift\n\
         if [ ! -x \"{copy}\" ]; then\n  echo \"pgg-runner {copy} is not here — this run's \
         task runner was not prepared, and nothing here falls back to cargo\"\n  \
         exit 127\nfi\n\
         mkdir -p \"{state}/steps\"\ntouch \"{state}/steps/{mark}\" \"{state}/live\"\n\
         \"$@\"\ncode=$?\n\
         rm -f \"{state}/steps/{mark}\"\ntouch \"{state}/live\"\n\
         exec env -u {STEP_MARK} sh -c \"$leftovers\" pgg-leftovers {mark} \"$code\" \"$@\"\n"
    )
}

/// The leavings walk a verb's wrapper hands to a fresh shell: `$1` is
/// the mark, `$2` the verb's exit code, the rest the line the verb was.
/// Ends what still carries the mark and names each, prints the line a
/// red verb was started as, and exits with the verb's own code.
fn leftovers_script() -> String {
    format!(
        "mark=$1\ncode=$2\nshift 2\n{MARKED_FN}\
         marked \"$mark\" | while read -r pid name; do\n  \
         kill -9 \"$pid\" 2>/dev/null && echo \"pgg-step took $pid ($name) with it\"\ndone\n\
         if [ \"$code\" -ne 0 ]; then\n  \
         printf 'pgg-probe ran'; for w in \"$@\"; do printf ' [%s]' \"$w\"; done; echo\nfi\n\
         exit \"$code\"\n"
    )
}

/// What a stop runs inside the container: `$1` is the mark, `$2` the
/// state directory ([`STATE_DIR`]). Ends what carries the mark, looks
/// again once the kills have had a second to land, takes the mark's
/// file down so the container does not count the step as running, and
/// ends with one line saying how many went and how many would not.
/// Sent by `docker exec` from a side that never had the mark, so the
/// walk is clean ([`MARKED_FN`]).
fn stop_script() -> String {
    format!(
        "mark=$1\nsteps=$2/steps\ntook=0\n{MARKED_FN}\
         for pid in $(marked \"$mark\" | cut -d' ' -f1); do\n  \
         name=$(cat \"/proc/$pid/comm\" 2>/dev/null)\n  \
         if kill -9 \"$pid\" 2>/dev/null; then echo \"pgg-stop took $pid ($name)\"; \
         took=$((took + 1)); fi\ndone\n\
         [ \"$took\" -gt 0 ] && sleep 1\n\
         left=$(marked \"$mark\" | wc -l)\n\
         marked \"$mark\" | while read -r pid name; do echo \"pgg-stop left $pid ($name)\"; done\n\
         rm -f \"$steps/$mark\"\n\
         echo \"pgg-stop $mark: took $took process(es), $left left\"\n"
    )
}

/// The container's own process: waits, and leaves when no verb has
/// been in for `$1` seconds and none is running — a mark younger than
/// `$2` minutes is a verb running. `$3` is where it says so, `$4` the
/// state directory ([`STATE_DIR`]).
///
/// This is the ceiling on a container whose gate died: the gate's own
/// removal is the ordinary end, the next gate's sweep is the second,
/// and this is the one that needs nobody. A `live` it cannot read is
/// read as now, so a state directory it could not make keeps it up —
/// that is a container to be taken down, not one to leave under a
/// verb.
const IDLE_SCRIPT: &str = "idle=$1\n\
     mark_minutes=$2\n\
     log=$3\n\
     state=$4\n\
     mkdir -p \"$state/steps\"\n\
     touch \"$state/live\"\n\
     say() { echo \"pgg-gate $(date -u +%FT%TZ) $1\"; echo \"pgg-gate $(date -u +%FT%TZ) $1\" >> \"$log\" 2>/dev/null; }\n\
     say \"up: leaves after ${idle}s with no verb in, a verb's mark counts for ${mark_minutes} minutes\"\n\
     while :; do\n\
     \x20 sleep 5\n\
     \x20 now=$(date +%s)\n\
     \x20 last=$(stat -c %Y \"$state/live\" 2>/dev/null || echo \"$now\")\n\
     \x20 running=$(find \"$state/steps\" -type f -mmin -\"$mark_minutes\" 2>/dev/null | wc -l)\n\
     \x20 if [ \"$running\" -eq 0 ] && [ $((now - last)) -ge \"$idle\" ]; then\n\
     \x20 \x20 say \"leaving: no verb for $((now - last))s and none running\"\n\
     \x20 \x20 exit 0\n\
     \x20 fi\n\
     done\n";

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::super::runner::spelled;

    /// The gate's container is named for the tree and the run, as the
    /// tree's volumes are: two seats and two runs of one seat never
    /// meet at a name, and the name is one docker and a shell accept.
    #[test]
    fn the_gates_container_is_named_for_the_tree_and_the_run() {
        let root = Path::new("C:\\Users\\x\\IdeaProjects\\platitude-gg\\.claude\\worktrees\\c");
        assert_eq!(
            super::container_of(root, "1758246913-40244"),
            "pgg-linux-gate-c-1758246913-40244"
        );
        assert_ne!(
            super::container_of(root, "1758-40"),
            super::container_of(Path::new("/w/d"), "1758-40")
        );
        assert!(spelled(&super::container_of(root, "1758-40")).is_ok());
    }

    /// **What runs in the gate's container carries its mark, and the
    /// mark is a name.** A step's mark is written into a file name and
    /// a shell script, so it is held to the same characters as a copy's
    /// name; and the `exec` carries both machine marks, the container
    /// mark and the step's, with no `run` and no `--rm` in it —
    /// nothing is created.
    #[test]
    fn a_verbs_exec_carries_every_mark_and_creates_nothing() {
        let inside = [
            "/work/target/gate-runner/xtask-1758-40",
            "verify-ui",
            "wip",
            "--shot-dir",
            "/out/shots-1",
            "--no-board",
        ]
        .map(String::from);
        let line = super::exec_line("pgg-linux-gate-c-1758-40", &inside, "1758-40-linux-12")
            .expect("a line");
        let args: Vec<String> = line
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(line.get_program(), "docker");
        assert_eq!(args[0], "exec");
        for env in [
            format!("{}=1", crate::still::UNDER),
            format!("{}=1", crate::budget::HELD),
            format!("{}=1", super::super::IN_CONTAINER),
            format!("{}=1758-40-linux-12", super::STEP_MARK),
        ] {
            assert!(
                args.windows(2)
                    .any(|pair| pair[0] == "--env" && pair[1] == env),
                "{env} is missing from {args:?}"
            );
        }
        assert!(
            args.windows(2)
                .any(|pair| pair[0] == "--workdir" && pair[1] == super::super::WORK)
        );
        assert!(
            !args.iter().any(|arg| arg == "run" || arg == "--rm"),
            "an exec creates no container: {args:?}"
        );
        let at = args
            .iter()
            .position(|arg| arg == "pgg-step")
            .expect("the script's $0");
        // The leavings walk rides as the wrapper's first argument, and
        // the copy and its verb follow it.
        assert_eq!(args[at + 1], super::leftovers_script());
        assert_eq!(args[at + 2..], inside[..]);
        assert!(
            args[..at].contains(&"pgg-linux-gate-c-1758-40".to_string()),
            "the container is named ahead of the script: {args:?}"
        );
        assert!(super::exec_line("c", &[], "m").is_err());
    }

    /// **The wrapper marks the step for as long as it runs, and hands
    /// its leavings to a shell that was never marked.** The mark's file
    /// is made before the verb and taken down after; the walk is an
    /// `exec` of a fresh shell with the mark taken out of its
    /// environment ([`MARKED_FN`]), which carries the verb's own exit
    /// code out.
    #[test]
    fn the_wrapper_marks_the_step_and_hands_its_leavings_to_an_unmarked_shell() {
        let script = super::exec_script("/w/xtask-1", "1758-40-linux-12", "/run/pgg-gate");
        let at = |what: &str| {
            script
                .lines()
                .position(|l| l.contains(what))
                .unwrap_or_else(|| panic!("{what} is not in the script:\n{script}"))
        };
        assert!(script.starts_with("leftovers=$1\nshift\n"), "{script}");
        assert!(at("if [ ! -x \"/w/xtask-1\" ]") < at("\"$@\""));
        // The guard leaves before the mark is put down: a copy that is
        // not there is a verb that never ran, and a mark standing for it
        // would count as running for the whole step ceiling.
        assert!(at("exit 127") < at("touch \"/run/pgg-gate/steps/1758-40-linux-12\""));
        assert!(at("touch \"/run/pgg-gate/steps/1758-40-linux-12\"") < at("\"$@\""));
        assert!(at("\"$@\"") < at("rm -f \"/run/pgg-gate/steps/1758-40-linux-12\""));
        assert!(
            script.trim_end().ends_with(
                "exec env -u PGG_STEP sh -c \"$leftovers\" pgg-leftovers 1758-40-linux-12 \
                 \"$code\" \"$@\""
            ),
            "the walk is the last thing, in a shell of its own:\n{script}"
        );
        assert!(
            !script.contains("marked"),
            "the wrapper itself walks nothing — it is the marked shell:\n{script}"
        );

        let walk = super::leftovers_script();
        let at = |what: &str| {
            walk.lines()
                .position(|l| l.contains(what))
                .unwrap_or_else(|| panic!("{what} is not in the walk:\n{walk}"))
        };
        assert!(walk.starts_with("mark=$1\ncode=$2\nshift 2\n"), "{walk}");
        assert!(at("marked() {") < at("marked \"$mark\" |"));
        assert!(walk.contains("[ \"$pid\" = \"$$\" ] && continue"), "{walk}");
        assert!(at("marked \"$mark\" |") < at("pgg-probe ran"));
        assert!(walk.trim_end().ends_with("exit \"$code\""), "{walk}");

        let stop = super::stop_script();
        let at = |what: &str| {
            stop.lines()
                .position(|l| l.contains(what))
                .unwrap_or_else(|| panic!("{what} is not in the stop:\n{stop}"))
        };
        assert!(at("kill -9") < at("sleep 1"));
        assert!(at("sleep 1") < at("left=$(marked"));
        assert!(at("left=$(marked") < at("rm -f \"$steps/$mark\""));
        assert!(stop.contains("steps=$2/steps\n"), "{stop}");
        assert!(
            stop.trim_end()
                .ends_with("took $took process(es), $left left\""),
            "{stop}"
        );

        let idle = super::IDLE_SCRIPT;
        assert!(idle.contains("state=$4\n"), "{idle}");
        assert!(
            idle.contains("[ \"$running\" -eq 0 ] && [ $((now - last)) -ge \"$idle\" ]"),
            "{idle}"
        );
        assert!(idle.contains("-mmin -\"$mark_minutes\""), "{idle}");
    }

    /// A root of this test's own with a state directory in it, and the
    /// mark its processes carry. **The copy's path is spelled into the
    /// wrapper**, so the name carries nothing a shell reads: no thread
    /// id, whose `Debug` form has parentheses in it.
    #[cfg(target_os = "linux")]
    fn a_marked_root(what: &str) -> (std::path::PathBuf, std::path::PathBuf, String) {
        let root = std::env::temp_dir().join(format!("pgg-marked-{what}-{}", std::process::id()));
        let state = root.join("state");
        std::fs::create_dir_all(&state).expect("a state directory");
        (root, state, format!("t{}-{what}-01", std::process::id()))
    }

    /// A `sleep` carrying `mark`, the way a verb's process would.
    #[cfg(target_os = "linux")]
    fn a_sleeper_marked(mark: &str) -> std::process::Child {
        use std::process::{Command, Stdio};
        Command::new("sleep")
            .arg("300")
            .env(super::STEP_MARK, mark)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("a marked sleeper")
    }

    /// Whether the process is still there and not a zombie waiting to
    /// be read.
    #[cfg(target_os = "linux")]
    fn alive(pid: u32) -> bool {
        std::fs::read_to_string(format!("/proc/{pid}/stat"))
            .is_ok_and(|stat| !stat.contains(") Z "))
    }

    /// **The stop ends what carries the mark and nothing else**, and its
    /// last line counts both. Run against the real shell: Linux only,
    /// where `/proc` is what the walk reads — the container runs this
    /// crate's tests every gate.
    #[test]
    #[cfg(target_os = "linux")]
    fn the_stop_takes_what_carries_the_mark_and_spares_the_rest() {
        let (root, state, mark) = a_marked_root("stop");
        let mut marked = a_sleeper_marked(&mark);
        let mut unmarked = a_sleeper_marked(&format!("{mark}-other"));
        let stop = std::process::Command::new("sh")
            .args(["-c", &super::stop_script(), "pgg-stop", &mark])
            .arg(&state)
            .output()
            .expect("sh");
        let said = String::from_utf8_lossy(&stop.stdout);
        assert!(stop.status.success(), "{said}");
        assert!(
            said.contains("took 1 process(es), 0 left"),
            "the stop miscounted:\n{said}"
        );
        let _ = marked.wait();
        assert!(
            alive(unmarked.id()),
            "the stop reached a process carrying another mark"
        );
        let _ = unmarked.kill();
        let _ = unmarked.wait();
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **The wrapper carries the verb's own exit code and takes the
    /// leftover its verb left — and nothing else**: a stub verb that
    /// leaves a marked sleeper behind and exits 3. The sleeper goes,
    /// the mark's file goes, 3 is what comes out, and the walk names
    /// exactly one process. A walk that met itself named its own `sh`
    /// beside the leftover, in every green verb of a gate (measured),
    /// which is what the count is for.
    #[test]
    #[cfg(target_os = "linux")]
    fn the_wrapper_carries_the_verbs_code_and_takes_its_leftover() {
        use std::os::unix::fs::PermissionsExt;

        let (root, state, mark) = a_marked_root("wrap");
        let stub = root.join("verb");
        std::fs::write(
            &stub,
            format!(
                "#!/bin/sh\nsleep 300 &\necho $! > '{}'\nexit 3\n",
                root.join("leftover").display()
            ),
        )
        .expect("a stub verb");
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).expect("+x");
        let script = super::exec_script(
            &stub.display().to_string(),
            &mark,
            &state.display().to_string(),
        );
        let ran = std::process::Command::new("sh")
            .args(["-c", &script, "pgg-step", &super::leftovers_script()])
            .arg(&stub)
            .env(super::STEP_MARK, &mark)
            .output()
            .expect("sh");
        let said = String::from_utf8_lossy(&ran.stdout);
        assert_eq!(
            ran.status.code(),
            Some(3),
            "the verb's own code is what comes out:\n{said}"
        );
        let leftover: u32 = std::fs::read_to_string(root.join("leftover"))
            .expect("the stub said its sleeper's pid")
            .trim()
            .parse()
            .expect("a pid");
        assert!(
            said.contains(&format!("pgg-step took {leftover} (sleep) with it")),
            "the wrapper did not take the leftover:\n{said}"
        );
        assert_eq!(
            said.matches("pgg-step took").count(),
            1,
            "the walk took something that was not the verb's — itself, most likely:\n{said}"
        );
        assert!(!alive(leftover), "the leftover is still running");
        assert!(
            !state.join("steps").join(&mark).exists(),
            "the mark's file stands after the verb"
        );
        assert!(
            said.contains("pgg-probe ran"),
            "a red verb's line is printed:\n{said}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **The container's own process leaves once nothing has been in**:
    /// with one second of idle allowed and nothing running, it leaves
    /// on its first look and says so where it was told to.
    #[test]
    #[cfg(target_os = "linux")]
    fn the_idle_ceiling_lets_the_container_leave_and_say_why() {
        let (root, state, _) = a_marked_root("idle");
        let log = root.join("container.log");
        let mut idle = std::process::Command::new("sh")
            .args(["-c", super::IDLE_SCRIPT, "pgg-gate", "1", "90"])
            .arg(&log)
            .arg(&state)
            .stdout(std::process::Stdio::null())
            .spawn()
            .expect("sh");
        let mut wait = crate::wait::Wait::new(
            "the idle ceiling",
            crate::wait::Budget::whole(std::time::Duration::from_secs(30)),
            crate::wait::LOOK_AGAIN,
        );
        let status = loop {
            if let Some(status) = idle.try_wait().expect("asked after") {
                break status;
            }
            if wait.look_again("the process to leave").is_err() {
                let _ = idle.kill();
                panic!("the idle ceiling never let the process leave");
            }
        };
        assert!(status.success());
        let wrote = std::fs::read_to_string(&log).unwrap_or_default();
        assert!(wrote.contains("up: leaves after 1s"), "{wrote}");
        assert!(wrote.contains("leaving: no verb for"), "{wrote}");
        let _ = std::fs::remove_dir_all(&root);
    }
}
