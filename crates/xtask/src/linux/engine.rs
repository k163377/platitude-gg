//! The container engine a host drives: WSL's own `wslc` on Windows (WSL
//! 3.0 and later, no Docker Desktop), docker anywhere else. A Linux host
//! needs neither: the command runs where it stands (`super::here`).
//!
//! **One set of words for both.** The two CLIs take the same verbs and
//! flags for everything this crate asks of them, but for three things,
//! each spelled once here:
//!
//! * **Listings** — wslc takes no Go template, and both print
//!   `--format json` as one object a line under the same keys, so every
//!   listing asks for that and reads the fields by hand ([`field`]).
//! * **`--init`** — wslc has none. The image carries the same init docker
//!   would have put in ([`INIT`]).
//! * **The build cache** — docker's `builder` verb is not wslc's; the
//!   engine inside wslc's VM still has it ([`trim_build_cache`]).
//!
//! **What an engine says is in the machine's language.** wslc answers a
//! missing object in Japanese on a Japanese Windows, so nothing here reads
//! a refusal's words to learn what state it left: a removal that failed is
//! followed by asking whether the object is still there ([`presence`]) —
//! there, gone, or not known because the engine could not be asked.
//!
//! **An engine that gives no answer is not an answer.** Everything short
//! that is asked of it goes through [`put`]: to its end or to a ceiling,
//! and a command that exited non-zero, could not be started or ran out
//! of time comes back as that — with what was asked, when, for how long
//! and what it said — never as "the object is not there". What the
//! caller does next is its own (`super::built` builds nothing on it).
//! Nothing here ends the engine's VM or its session, which every
//! checkout on the machine shares, and nothing is put twice: the CLI
//! ending says nothing about what the engine had already taken on.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::subprocess::{Answer, Stdin, bounded_apart};

/// What this host's engine is called, in what this crate says.
pub(crate) const NAME: &str = if cfg!(windows) { "wslc" } else { "docker" };

/// What the engine needs to be there, said where a command cannot start
/// it.
pub(crate) const NEEDS: &str = if cfg!(windows) {
    "WSL 3.0 or later (`wsl --update`), whose wslc.exe is the whole Linux side of a Windows \
     workstation — no Docker Desktop"
} else {
    "docker on PATH"
};

/// The init the image puts in front of a long-lived container's own
/// process: tini, which is what docker's `--init` starts. Something has
/// to reap what a verb leaves orphaned, and a shell waiting on its own
/// `sleep` is not a promise to.
pub(crate) const INIT: [&str; 2] = ["tini", "--"];

/// The engine's command line, with nothing on it yet.
pub(crate) fn command() -> Command {
    Command::new(program())
}

/// Where the engine's CLI is. On Windows, WSL's own install directory
/// first: a shell started before WSL 3 was installed carries a PATH
/// without it, and every harness shell is one.
fn program() -> PathBuf {
    if cfg!(windows)
        && let Some(files) = std::env::var_os("ProgramFiles")
    {
        let installed = PathBuf::from(files).join("WSL").join("wslc.exe");
        if installed.is_file() {
            return installed;
        }
    }
    PathBuf::from(NAME)
}

/// A command run in the engine's VM itself rather than in a container —
/// where docker's own CLI is, behind wslc. Off Windows, the engine's CLI
/// answers on the host and `program` is run there.
///
/// **Stdin is an empty pipe, never the null device.** wslc relays stdin
/// to the process in its VM, and fails the whole run as an invalid handle
/// when that is `NUL` (or a console it was not given) — a pipe or a file
/// it relays. `output` closes the pipe at once; a sampler holds it for as
/// long as it runs. (`run` and `exec` take `NUL` as they should.)
pub(crate) fn in_its_vm(program: &str, args: &[&str]) -> Command {
    let mut cmd = if cfg!(windows) {
        let mut cmd = command();
        cmd.args(["system", "session", "run", program]);
        cmd
    } else {
        Command::new(program)
    };
    cmd.args(args).stdin(Stdio::piped());
    cmd
}

/// Cuts the build cache back to `ceiling` bytes of what no image shares
/// (`super::TrimTheCache` says why), and answers what docker said it
/// freed — its own last line, the only place it says so.
///
/// The ceiling has two spellings: wslc's VM carries docker 25, whose
/// buildx calls it `--keep-storage`; the docker a host installs is past
/// the rename to `--max-used-space`.
///
/// `said` is the file docker's words are written to. A trim past
/// [`WORKING_CEILING`] is left to the engine: whether it is still at it
/// in there is not known out here, and it is not asked again.
pub(crate) fn trim_build_cache(ceiling: u64, said: &Path) -> Result<String, String> {
    let ceiling = ceiling.to_string();
    let flag = if cfg!(windows) {
        "--keep-storage"
    } else {
        "--max-used-space"
    };
    let prune = in_its_vm("docker", &["builder", "prune", "--force", flag, &ceiling]);
    put(prune, WORKING_CEILING, said, Stdin::Closed)
        .freed()
        .map_err(NoAnswer::told)
}

/// How putting one command to the engine ended.
#[derive(Debug, PartialEq)]
enum Put {
    /// It exited zero: what it wrote.
    Answered(String),
    /// It exited non-zero: the account of that, and its stderr alone for
    /// a caller that goes on to ask what state the refusal left.
    Refused { account: String, said: String },
    /// It was still standing at its ceiling: the account. The engine was
    /// asked and nothing is known.
    Silent(String),
    /// It could not be started, or asked after: the account.
    Unstarted(String),
}

/// The account of a command that left no answer to read, by what it is
/// an account of.
#[derive(Debug, PartialEq)]
enum NoAnswer {
    /// The engine itself is in a state: it stood silent, or refused what
    /// an engine in order does not.
    OfTheEngine(String),
    /// The command's own: it never reached the engine, or was refused
    /// what an engine in order may refuse.
    OfTheCommand(String),
}

impl NoAnswer {
    /// The account as a caller is told it. One of the engine carries
    /// where the witness of that moment is kept ([`super::witness`]):
    /// the one time the engine's processes are worth a look before
    /// anything is done about them.
    fn told(self) -> String {
        match self {
            NoAnswer::OfTheCommand(account) => account,
            NoAnswer::OfTheEngine(account) => {
                let kept = super::witness::beside(&account);
                format!("{account}{kept}")
            }
        }
    }
}

impl Put {
    /// Read as a listing: its rows. A listing is not something an engine
    /// in order refuses, so a refusal is of the engine as a silence is.
    fn rows(self) -> Result<String, NoAnswer> {
        match self {
            Put::Answered(rows) => Ok(rows),
            Put::Refused { account, .. } | Put::Silent(account) => {
                Err(NoAnswer::OfTheEngine(account))
            }
            Put::Unstarted(account) => Err(NoAnswer::OfTheCommand(account)),
        }
    }

    /// Read as a removal: done (`None`), or refused in the words given —
    /// an engine in order refuses to remove what is in use, and
    /// [`presence`] is what says the state that left.
    fn removal(self) -> Result<Option<String>, NoAnswer> {
        match self {
            Put::Answered(_) => Ok(None),
            Put::Refused { said, .. } => Ok(Some(said)),
            Put::Silent(account) => Err(NoAnswer::OfTheEngine(account)),
            Put::Unstarted(account) => Err(NoAnswer::OfTheCommand(account)),
        }
    }

    /// Read as the build cache's trim: what docker said it freed, off
    /// its own last line.
    fn freed(self) -> Result<String, NoAnswer> {
        match self {
            Put::Answered(wrote) => Ok(wrote
                .lines()
                .rev()
                .find_map(|line| line.trim().strip_prefix("Total:"))
                .map_or("nothing", str::trim)
                .to_string()),
            Put::Silent(account) => Err(NoAnswer::OfTheEngine(account)),
            Put::Refused { account, .. } | Put::Unstarted(account) => {
                Err(NoAnswer::OfTheCommand(account))
            }
        }
    }
}

/// Runs `command` to its end or to `ceiling`, and answers which — with,
/// for anything but an answer, the account a reader needs who was not
/// standing there: the command, when it was put (UTC), how long it stood,
/// how it ended, and what it said on stderr.
///
/// Out of time says only that: the engine gave no answer within the
/// ceiling. Why is not read off a silence.
///
/// `said` takes the command's stdout; its stderr stands beside it while
/// the command runs and is gone by the time this answers.
fn put(command: Command, ceiling: Duration, said: &Path, stdin: Stdin) -> Put {
    let line = spelled(&command);
    let refused = said.with_extension("refused.txt");
    let at = clock(crate::note::now_secs());
    // waits(measured): how long the engine stood, said in the account and judged by nothing
    let since = std::time::Instant::now();
    let answer = bounded_apart(&line, command, ceiling, said, &refused, stdin);
    let stood = since.elapsed().as_secs_f32();
    let on_stderr = std::fs::read(&refused)
        .map(|bytes| String::from_utf8_lossy(&bytes).trim().to_string())
        .unwrap_or_default();
    let _ = std::fs::remove_file(&refused);
    account_of(answer, &line, &at, stood, ceiling, on_stderr)
}

/// [`put`]'s reading of how its command ended: `line` put at `at`, stood
/// `stood` seconds under `ceiling`, and said `on_stderr`. Pure, so a test
/// holds what each account carries.
fn account_of(
    answer: Answer,
    line: &str,
    at: &str,
    stood: f32,
    ceiling: Duration,
    on_stderr: String,
) -> Put {
    match answer {
        Answer::Ended { status, stdout } if status.success() => Put::Answered(stdout),
        Answer::Ended { status, .. } => Put::Refused {
            account: format!(
                "`{line}` exited {:?} after {stood:.1}s (put at {at}) and said: {}",
                status.code(),
                if on_stderr.is_empty() {
                    "nothing on stderr"
                } else {
                    &on_stderr
                }
            ),
            said: on_stderr,
        },
        Answer::OutOfTime { pid, ended, .. } => Put::Silent(format!(
            "`{line}` gave no answer within {:.0}s (put at {at}) — the {NAME} process out here \
             (pid {pid}) was {}; what the engine itself is doing with the command is not known. \
             On stderr by then: {}",
            ceiling.as_secs_f32(),
            match ended {
                Ok(()) => "ended".to_string(),
                Err(why) => format!("not ended: {why}"),
            },
            if on_stderr.is_empty() {
                "nothing"
            } else {
                &on_stderr
            }
        )),
        Answer::Unstarted(why) => Put::Unstarted(format!(
            "{why} (put at {at}). `cargo xtask linux` needs {NEEDS}."
        )),
    }
}

/// The command as a reader would type it: the engine's name, not the
/// path this host found it at.
fn spelled(command: &Command) -> String {
    let program = Path::new(command.get_program())
        .file_stem()
        .map_or_else(|| NAME.into(), |stem| stem.to_string_lossy());
    std::iter::once(program)
        .chain(command.get_args().map(|arg| arg.to_string_lossy()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// A time of day off the epoch's seconds, in UTC — what a log beside
/// this one is stamped in.
pub(super) fn clock(secs: u64) -> String {
    format!(
        "{:02}:{:02}:{:02}Z",
        secs / 3600 % 24,
        secs / 60 % 60,
        secs % 60
    )
}

/// What kind of object [`presence`] asks about.
#[derive(Clone, Copy)]
pub(crate) enum Kind {
    Image,
    Volume,
    Container,
}

/// What asking after an object found out.
#[derive(Debug, PartialEq)]
pub(crate) enum Presence {
    /// The engine listed it.
    There,
    /// The engine answered, and what it listed does not hold it.
    Gone,
    /// The engine could not be asked, or did not answer: nothing is known
    /// about the object, and why is said.
    Unknown(String),
}

/// When a question reaches the engine, which is what bounds it.
#[derive(Clone, Copy)]
pub(crate) enum Asked {
    /// The first thing a line says to the engine: it may find the VM
    /// being taken down, and wait for that and for the next one's boot.
    First,
    /// Behind an answer the engine has just given: a listing is the
    /// time of one process.
    Behind,
}

impl Asked {
    fn ceiling(self) -> Duration {
        match self {
            Asked::First => FIRST_CEILING,
            Asked::Behind => ASKING_CEILING,
        }
    }
}

/// [`Asked::Behind`]'s ceiling, against a daemon that may be wedged.
const ASKING_CEILING: Duration = Duration::from_secs(30);

/// [`Asked::First`]'s ceiling. Such a question can arrive while the
/// engine takes its VM down and then has to boot the next; the timeouts
/// the engine sets on the steps of that (WSL 3.0.1: its two daemons'
/// stops, the unmount, the VM's exit, the boot, the daemon's readiness)
/// add up to about three minutes, and this stands above them. A number
/// and no more: the engine also waits on things it sets no timeout on,
/// and nothing here tells a slow teardown from one of those — what is
/// still standing at the ceiling is reported as exactly that.
const FIRST_CEILING: Duration = Duration::from_secs(300);

/// The ceiling of a command that has the engine delete something (a
/// volume, an image, build cache): the VM's disk at work, so not a
/// listing's. One that outlasts it is left alone, never put again.
const WORKING_CEILING: Duration = Duration::from_secs(300);

/// Whether `name` is on the engine: the state, read off a listing, and
/// not the words or the exit code of something else that was asked.
///
/// **Off a listing that succeeded, never off an inspect's status**: an
/// inspect exits non-zero both for an object that is not there and for an
/// engine that could not be reached or would not answer, so only a
/// listing the engine answered can say an object is gone. `said` is the
/// file the listing is written to.
pub(crate) fn presence(kind: Kind, name: &str, said: &Path, asked: Asked) -> Presence {
    let mut list = command();
    match kind {
        Kind::Image => list.args([
            "images",
            "--filter",
            &format!("reference={name}"),
            "--format",
            "json",
        ]),
        Kind::Volume => list.args(["volume", "ls", "--quiet"]),
        Kind::Container => list.args(["ps", "--all", "--format", "json"]),
    };
    read_presence(kind, name, listed(list, asked, said))
}

/// A listing the engine answered, or the account of one it did not
/// ([`put`]). `said` is the file it is written to.
pub(crate) fn listed(list: Command, asked: Asked, said: &Path) -> Result<String, String> {
    put(list, asked.ceiling(), said, Stdin::Null)
        .rows()
        .map_err(NoAnswer::told)
}

/// What became of asking the engine to remove something.
pub(crate) enum Removal {
    Done,
    /// The engine said no, in these words — which say nothing of the
    /// state they left ([`presence`] does).
    Refused(String),
    /// No answer: whether the object is still there is not known, and an
    /// engine in that state is asked for nothing more.
    Unanswered(String),
}

/// Removes the image `tag`. `said` is the file the engine's words are
/// written to.
pub(crate) fn remove_image(tag: &str, said: &Path) -> Removal {
    removed(&["image", "rm", tag], said)
}

/// Removes the volume `name`, as [`remove_image`] does an image.
pub(crate) fn remove_volume(name: &str, said: &Path) -> Removal {
    removed(&["volume", "rm", name], said)
}

fn removed(line: &[&str], said: &Path) -> Removal {
    let mut rm = command();
    rm.args(line);
    match put(rm, WORKING_CEILING, said, Stdin::Null).removal() {
        Ok(None) => Removal::Done,
        Ok(Some(refusal)) => Removal::Refused(refusal),
        Err(none) => Removal::Unanswered(none.told()),
    }
}

/// [`presence`]'s reading of a listing, or of a listing that could not be
/// had. Pure, so a test holds which answers may say gone.
fn read_presence(kind: Kind, name: &str, listed: Result<String, String>) -> Presence {
    let listed = match listed {
        Ok(listed) => listed,
        Err(why) => return Presence::Unknown(why),
    };
    let held = listed.lines().any(|line| match kind {
        Kind::Image => field(line, "Repository")
            .zip(field(line, "Tag"))
            .is_some_and(|(repository, tag)| format!("{repository}:{tag}") == name),
        Kind::Volume => line.trim() == name,
        Kind::Container => {
            field(line, "Names").is_some_and(|names| names.split(',').any(|one| one == name))
        }
    });
    if held {
        Presence::There
    } else {
        Presence::Gone
    }
}

/// The value a `--format json` line holds under `name`, as a string — or
/// None when the key is absent or not a string. Escapes are read, so a
/// value holding a quoted string of its own (wslc's metadata label, in a
/// container's `Labels`) does not end at its first inner quote.
///
/// The first key spelled `name` wins, at any depth: the listings this
/// reads are flat, and `Id` leads an inspect's object.
pub(crate) fn field(line: &str, name: &str) -> Option<String> {
    let key = format!("\"{name}\":");
    let at = line.find(&key)? + key.len();
    let mut chars = line[at..].trim_start().chars();
    if chars.next()? != '"' {
        return None;
    }
    let mut value = String::new();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(value),
            '\\' => match chars.next()? {
                'n' => value.push('\n'),
                't' => value.push('\t'),
                'u' => {
                    let hex: String = chars.by_ref().take(4).collect();
                    value.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                }
                other => value.push(other),
            },
            c => value.push(c),
        }
    }
    None
}

/// One label's value out of a listing's `Labels` — `k=v` pairs joined by
/// commas. A value carrying commas of its own (wslc's metadata) splits
/// into pieces none of which start `key=` unless one is that label.
pub(crate) fn label(labels: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}=");
    labels
        .split(',')
        .find_map(|pair| pair.strip_prefix(&prefix))
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::process::Command;
    use std::time::Duration;

    use super::{Kind, NoAnswer, Presence, Put, clock, field, label, put, read_presence, spelled};
    use crate::subprocess::Stdin;

    /// A shell told what the engine's CLI is to do in its place.
    #[cfg(windows)]
    fn a_cli_that(windows: &str, _unix: &str) -> Command {
        let mut command = Command::new("cmd");
        command.args(["/c", windows]);
        command
    }

    #[cfg(not(windows))]
    fn a_cli_that(_windows: &str, unix: &str) -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", unix]);
        command
    }

    /// One that stands for ten minutes and says nothing — the program
    /// itself, no shell in front: ending a shell leaves what it started.
    #[cfg(windows)]
    fn a_cli_that_gives_no_answer() -> Command {
        let mut command = Command::new("ping");
        command.args(["-n", "600", "127.0.0.1"]);
        command
    }

    #[cfg(not(windows))]
    fn a_cli_that_gives_no_answer() -> Command {
        let mut command = Command::new("sleep");
        command.arg("600");
        command
    }

    /// One that writes `rows` as its listing, after `before` (a shell's
    /// own words, one per system).
    fn a_cli_that_lists(rows: &Path, before: (&str, &str)) -> Command {
        a_cli_that(
            &format!("{}type {}", before.0, rows.display()),
            &format!("{}cat {}", before.1, rows.display()),
        )
    }

    /// What a [`NoAnswer`] is an account of, and the account — the two
    /// things [`NoAnswer::told`] goes by, read without telling it: told,
    /// an account of the engine takes a look at this machine's real one.
    fn of_the_engine(none: NoAnswer) -> (bool, String) {
        match none {
            NoAnswer::OfTheEngine(account) => (true, account),
            NoAnswer::OfTheCommand(account) => (false, account),
        }
    }

    /// The image question put to a stand-in and read by what
    /// [`super::listed`] and [`super::presence`] read it with. Whether
    /// the account was of the engine comes back beside the answer.
    fn image_asked_of(cli: Command, ceiling: Duration, said: &Path) -> (Presence, Option<bool>) {
        let rows = put(cli, ceiling, said, Stdin::Null)
            .rows()
            .map_err(of_the_engine);
        let whose = rows.as_ref().err().map(|(engine, _)| *engine);
        let presence = read_presence(
            Kind::Image,
            "pgg-linux:app-e48be2d7ba8a8979",
            rows.map_err(|(_, account)| account),
        );
        (presence, whose)
    }

    /// The silence is said as a silence — no reason is read into it —
    /// and the process out here is ended and waited for, so the test
    /// coming back at all is the wait being finite.
    #[test]
    fn an_engine_that_gives_no_answer_is_not_an_image_that_is_gone() {
        let yard = crate::yard::Yard::new("engine-silent");
        let said = yard.join("said.txt");

        let (asked, of_the_engine) = image_asked_of(
            a_cli_that_gives_no_answer(),
            Duration::from_millis(300),
            &said,
        );

        let Presence::Unknown(why) = asked else {
            panic!("a silence was read as an answer: {asked:?}");
        };
        assert!(why.contains("gave no answer within"), "{why}");
        assert!(why.contains("was ended;"), "{why}");
        assert!(why.contains("put at "), "{why}");
        assert_eq!(of_the_engine, Some(true), "a silence is the engine's");
        assert!(
            !said.with_extension("refused.txt").exists(),
            "the stderr file outlived the question"
        );
    }

    /// What it had said on stderr before it stood still is in the
    /// account of the silence, beside the command and the time; and a
    /// silence says how long was waited, never why.
    #[test]
    fn what_was_said_before_a_silence_is_carried_with_it() {
        let stood = super::account_of(
            crate::subprocess::Answer::OutOfTime {
                pid: 7,
                after: Duration::from_secs(300),
                ended: Ok(()),
            },
            "wslc images",
            "02:32:27Z",
            300.0,
            Duration::from_secs(300),
            "the session is busy".to_string(),
        );

        let Put::Silent(why) = stood else {
            panic!("a silence was read as an answer: {stood:?}");
        };
        assert!(
            why.contains("`wslc images` gave no answer within 300s"),
            "{why}"
        );
        assert!(why.contains("put at 02:32:27Z"), "{why}");
        assert!(
            why.contains("On stderr by then: the session is busy"),
            "{why}"
        );
    }

    /// A CLI that is not on the machine reached no engine: the account
    /// says so and what is needed, and is nothing to take a witness of.
    #[test]
    fn a_cli_that_is_not_there_is_not_an_engine_in_a_state() {
        let yard = crate::yard::Yard::new("engine-unstarted");

        let (asked, of_the_engine) = image_asked_of(
            Command::new("pgg-no-such-engine-cli"),
            Duration::from_secs(60),
            &yard.join("said.txt"),
        );

        let Presence::Unknown(why) = asked else {
            panic!("a CLI that never started was read as an answer: {asked:?}");
        };
        assert!(why.contains("could not be started"), "{why}");
        assert!(why.contains(super::NEEDS), "{why}");
        assert_eq!(of_the_engine, Some(false));
    }

    /// What wslc answers once its session is going down: at once,
    /// non-zero, the reason on stderr. All of it reaches the reader, and
    /// none of it says the image is not there.
    #[test]
    fn a_refusal_is_carried_whole_and_says_nothing_of_the_image() {
        let yard = crate::yard::Yard::new("engine-refusal");

        let (asked, of_the_engine) = image_asked_of(
            a_cli_that(
                "echo ERROR_INVALID_STATE 1>&2& exit /b 3",
                "echo ERROR_INVALID_STATE >&2; exit 3",
            ),
            Duration::from_secs(60),
            &yard.join("said.txt"),
        );

        let Presence::Unknown(why) = asked else {
            panic!("a refusal was read as an answer: {asked:?}");
        };
        assert!(why.contains("exited Some(3)"), "{why}");
        assert!(why.contains("ERROR_INVALID_STATE"), "{why}");
        assert!(why.contains("put at "), "{why}");
        assert_eq!(
            of_the_engine,
            Some(true),
            "an engine in order refuses no listing"
        );
    }

    /// A removal an engine in order may refuse — the image is in use —
    /// is an answer, in the engine's words; one it stands silent on is
    /// not, and neither is ever read as done.
    #[test]
    fn a_removal_is_done_refused_or_unanswered_and_never_mixed() {
        let yard = crate::yard::Yard::new("engine-removal");
        let said = yard.join("said.txt");
        let within = Duration::from_secs(60);

        assert_eq!(
            put(
                a_cli_that("exit /b 0", "exit 0"),
                within,
                &said,
                Stdin::Null
            )
            .removal(),
            Ok(None)
        );
        assert_eq!(
            put(
                a_cli_that("echo in use 1>&2& exit /b 1", "echo in use >&2; exit 1"),
                within,
                &said,
                Stdin::Null
            )
            .removal(),
            Ok(Some("in use".to_string()))
        );
        let silent = put(
            a_cli_that_gives_no_answer(),
            Duration::from_millis(300),
            &said,
            Stdin::Null,
        )
        .removal()
        .expect_err("a silence was read as a removal");
        assert!(matches!(silent, NoAnswer::OfTheEngine(_)), "{silent:?}");
    }

    /// The one answer that may send a line on to build: the engine
    /// listed, and the tag is not among the rows.
    #[test]
    fn a_listing_that_came_back_without_the_image_says_gone() {
        let yard = crate::yard::Yard::new("engine-absent");
        let rows = yard.join("rows.txt");
        std::fs::write(&rows, IMAGE.replace("app-e48be", "app-00000")).expect("the rows");

        assert_eq!(
            image_asked_of(
                a_cli_that_lists(&rows, ("", "")),
                Duration::from_secs(60),
                &yard.join("said.txt"),
            ),
            (Presence::Gone, None)
        );
    }

    /// An engine that takes its time — a VM to boot — and then lists is
    /// answering: the ceiling is no part of what it said.
    #[test]
    fn an_answer_that_took_its_time_is_still_the_answer() {
        let yard = crate::yard::Yard::new("engine-slow");
        let rows = yard.join("rows.txt");
        std::fs::write(&rows, IMAGE).expect("the rows");

        assert_eq!(
            image_asked_of(
                a_cli_that_lists(&rows, ("ping -n 2 127.0.0.1 >nul& ", "sleep 1; ")),
                Duration::from_secs(60),
                &yard.join("said.txt"),
            ),
            (Presence::There, None)
        );
    }

    /// The trim's road (`super::in_its_vm`'s closed pipe): one put behind
    /// a failure, to an engine that answers that no better, ends at its
    /// ceiling too.
    #[test]
    fn a_command_put_behind_a_failure_ends_at_its_ceiling_as_well() {
        let yard = crate::yard::Yard::new("engine-cleanup");

        let trimmed = put(
            a_cli_that_gives_no_answer(),
            Duration::from_millis(300),
            &yard.join("said.txt"),
            Stdin::Closed,
        )
        .freed()
        .expect_err("a silence was read as a trim");

        let (engine, account) = of_the_engine(trimmed);
        assert!(account.contains("gave no answer within"), "{account}");
        assert!(engine, "a silence is the engine's");
    }

    /// What docker frees is on its last line, and a prune it refuses is
    /// its own failure, not an engine in a state.
    #[test]
    fn a_trim_reads_what_was_freed_off_dockers_last_line() {
        assert_eq!(
            Put::Answered("ID RECLAIMABLE\nabc true\nTotal:  1.2GB\n".into()).freed(),
            Ok("1.2GB".to_string())
        );
        assert_eq!(
            Put::Answered(String::new()).freed(),
            Ok("nothing".to_string())
        );
        let refused = Put::Refused {
            account: "the account".into(),
            said: "no".into(),
        }
        .freed()
        .expect_err("a refusal was read as a trim");
        assert!(matches!(refused, NoAnswer::OfTheCommand(_)), "{refused:?}");
    }

    /// The first question of a line is given longer than one behind an
    /// answer: it may wait out a VM going down and the next one's boot.
    #[test]
    fn the_first_question_is_given_longer_than_one_behind_an_answer() {
        assert!(super::Asked::First.ceiling() > super::Asked::Behind.ceiling());
    }

    #[test]
    fn an_account_names_the_command_and_the_time_of_day() {
        let mut list = super::command();
        list.args(["images", "--format", "json"]);
        assert_eq!(
            spelled(&list),
            format!("{} images --format json", super::NAME)
        );
        assert_eq!(clock(0), "00:00:00Z");
        assert_eq!(clock(86_400 + 3 * 3600 + 25 * 60 + 9), "03:25:09Z");
    }

    /// Lines as both engines print them, cut down.
    const IMAGE: &str = r#"{"Containers":"0","Digest":"<none>","ID":"198209722ed1","Repository":"pgg-linux","Tag":"app-e48be2d7ba8a8979"}"#;
    const CONTAINER: &str = r#"{"Command":"\"sh -c 'sleep 600'\"","ID":"0c0eb3ce5590","Labels":"com.microsoft.wsl.container.metadata={\"V1\":{\"Flags\":1,\"Ports\":[]}},pgg.gate=4242,pgg.tree=probe","Names":"pgg-linux-gate-a-1-2","Status":"Up 3 seconds"}"#;

    #[test]
    fn a_field_is_read_whole_through_its_escapes() {
        assert_eq!(field(IMAGE, "Repository").as_deref(), Some("pgg-linux"));
        assert_eq!(field(IMAGE, "Tag").as_deref(), Some("app-e48be2d7ba8a8979"));
        assert_eq!(field(IMAGE, "Digest").as_deref(), Some("<none>"));
        assert_eq!(
            field(CONTAINER, "Command").as_deref(),
            Some("\"sh -c 'sleep 600'\"")
        );
        assert_eq!(
            field(CONTAINER, "Names").as_deref(),
            Some("pgg-linux-gate-a-1-2"),
            "the label ahead of it, quotes and all, did not end the line early"
        );
        assert_eq!(field(IMAGE, "Absent"), None);
        assert_eq!(field(r#"{"Size":12}"#, "Size"), None, "not a string");
        assert_eq!(field(r#"{"Cut":"half"#, "Cut"), None, "no closing quote");
    }

    /// Only a listing the engine answered says gone; a listing that could
    /// not be had says nothing about the object, whatever it was asked.
    #[test]
    fn gone_is_only_said_off_a_listing_that_came_back() {
        let unreached = || Err("docker's listing exited exit code: 1".to_string());
        for kind in [Kind::Image, Kind::Volume, Kind::Container] {
            assert_eq!(
                read_presence(kind, "x", unreached()),
                Presence::Unknown("docker's listing exited exit code: 1".into())
            );
            assert_eq!(read_presence(kind, "x", Ok(String::new())), Presence::Gone);
        }
        let tag = "pgg-linux:app-e48be2d7ba8a8979";
        assert_eq!(
            read_presence(Kind::Image, tag, Ok(format!("{IMAGE}\n"))),
            Presence::There
        );
        assert_eq!(
            read_presence(
                Kind::Image,
                "pgg-linux:app-0000000000000000",
                Ok(format!("{IMAGE}\n"))
            ),
            Presence::Gone
        );
        let volumes = "pgg-linux-target-ab\npgg-linux-registry\n".to_string();
        assert_eq!(
            read_presence(Kind::Volume, "pgg-linux-target-a", Ok(volumes.clone())),
            Presence::Gone,
            "a name is matched whole, not as the start of another"
        );
        assert_eq!(
            read_presence(Kind::Volume, "pgg-linux-target-ab", Ok(volumes)),
            Presence::There
        );
        assert_eq!(
            read_presence(
                Kind::Container,
                "pgg-linux-gate-a-1-2",
                Ok(format!("{CONTAINER}\n"))
            ),
            Presence::There
        );
        assert_eq!(
            read_presence(
                Kind::Container,
                "pgg-linux-gate-a-1",
                Ok(format!("{CONTAINER}\n"))
            ),
            Presence::Gone
        );
    }

    #[test]
    fn a_label_is_found_among_its_neighbours_commas() {
        let labels = field(CONTAINER, "Labels").expect("labels");
        assert_eq!(label(&labels, "pgg.gate").as_deref(), Some("4242"));
        assert_eq!(label(&labels, "pgg.tree").as_deref(), Some("probe"));
        assert_eq!(label(&labels, "pgg.run"), None);
        assert_eq!(label("", "pgg.gate"), None);
    }
}
