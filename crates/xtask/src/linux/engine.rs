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

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::subprocess::{Answer, bounded};

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
pub(crate) fn trim_build_cache(ceiling: u64) -> Result<String, String> {
    let ceiling = ceiling.to_string();
    let flag = if cfg!(windows) {
        "--keep-storage"
    } else {
        "--max-used-space"
    };
    let out = in_its_vm("docker", &["builder", "prune", "--force", flag, &ceiling])
        .output()
        .map_err(|e| format!("could not start {NAME}: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let said = String::from_utf8_lossy(&out.stdout);
    Ok(said
        .lines()
        .rev()
        .find_map(|line| line.trim().strip_prefix("Total:"))
        .map_or("nothing", str::trim)
        .to_string())
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

/// How long [`presence`] may take, against a daemon that may be wedged —
/// a listing is a second when it works.
const ASKING_CEILING: Duration = Duration::from_secs(30);

/// Whether `name` is on the engine — asked after a removal that failed,
/// so the answer is the state and not the words of the refusal.
///
/// **Off a listing that succeeded, never off an inspect's status**: an
/// inspect exits non-zero both for an object that is not there and for an
/// engine that could not be reached (docker exits 1 on a refused
/// connection), so only a listing the engine answered can say an object
/// is gone. `said` is the file the listing is written to.
pub(crate) fn presence(kind: Kind, name: &str, said: &Path) -> Presence {
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
    let listed = match bounded("asking after a removal", list, ASKING_CEILING, Some(said)) {
        Answer::Ended { status, stdout } if status.success() => Ok(stdout),
        Answer::Ended { status, .. } => Err(format!("{NAME}'s listing exited {status}")),
        Answer::OutOfTime { after, .. } => Err(format!(
            "{NAME}'s listing did not answer within {:.0}s",
            after.as_secs_f32()
        )),
        Answer::Unstarted(why) => Err(why),
    };
    read_presence(kind, name, listed)
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
    use super::{Kind, Presence, field, label, read_presence};

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
