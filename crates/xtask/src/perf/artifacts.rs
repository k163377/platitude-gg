//! Immutable run evidence — failed runs keep their logs too — and the
//! `settings.toml` / `state.toml` every run writes: fetching while timed,
//! the rows the window holds and the screen it lands on are settings, and
//! a run that did not write them measures whatever the platform chose.
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use super::Options;
use super::display::Screen;

/// The app's own default window, said out loud: the height sets how many
/// rows a graph builds, and the overhang sets the scroll bench's speed.
const WINDOW_WIDTH: i32 = 1440;
const WINDOW_HEIGHT: i32 = 900;

pub(super) fn open_run(
    directory: &Path,
    opts: &Options,
    screen: Option<&Screen>,
) -> Result<(PathBuf, std::fs::File, std::fs::File), String> {
    let config = directory.join("config");
    std::fs::create_dir(&config).map_err(|e| e.to_string())?;
    std::fs::write(config.join("settings.toml"), settings_file(opts)).map_err(|e| e.to_string())?;
    std::fs::write(config.join("state.toml"), state_file(opts, screen))
        .map_err(|e| e.to_string())?;
    let log = std::fs::File::create(directory.join("app.log")).map_err(|e| e.to_string())?;
    let samples = std::fs::File::create(directory.join("memory.csv")).map_err(|e| e.to_string())?;
    Ok((config, log, samples))
}

/// No fetch while timed, then whatever `--setting` named — written after,
/// so a run that asks for the interval back gets it.
fn settings_file(opts: &Options) -> String {
    let mut text = String::from("version = 1\n\n[defaults]\nauto_fetch_minutes = 0\n");
    for (key, value) in &opts.settings {
        text.push_str(&format!("{key} = {value}\n"));
    }
    text
}

/// The window's place and size, and — for a build with no harness — the
/// repository to open: a shipped build has no command line to take one.
fn state_file(opts: &Options, screen: Option<&Screen>) -> String {
    let mut text = String::from("version = 1\n\n[window]\n");
    if let Some(screen) = screen {
        // Centred on the chosen screen: the same place every run.
        let x = screen.x + (screen.width - WINDOW_WIDTH).max(0) / 2;
        let y = screen.y + (screen.height - WINDOW_HEIGHT).max(0) / 2;
        text.push_str(&format!("x = {x}\ny = {y}\n"));
    }
    text.push_str(&format!(
        "width = {WINDOW_WIDTH}\nheight = {WINDOW_HEIGHT}\nmaximized = false\n"
    ));
    if !opts.harness && opts.open {
        text.push_str(&format!(
            "\n[tabs]\nactive = 0\npaths = [{}]\n",
            toml_string(&opts.repo.display().to_string())
        ));
    }
    text
}

/// A path as a TOML basic string: a literal (`'…'`) cannot hold the
/// single quote a path may contain, and a `state.toml` that does not
/// parse measures an empty window.
fn toml_string(path: &str) -> String {
    let mut quoted = String::with_capacity(path.len() + 2);
    quoted.push('"');
    for ch in path.chars() {
        match ch {
            '\\' => quoted.push('/'),
            '"' => quoted.push_str("\\\""),
            _ => quoted.push(ch),
        }
    }
    quoted.push('"');
    quoted
}

/// The evidence directory and the measured exe's blob id. The evidence
/// lands in `root`, the tree the command runs in; what was measured is
/// `built`, the rig's tree under `--at`.
pub(super) fn prepare(
    root: &Path,
    built: &super::rig::Built,
    opts: &Options,
    screen: Option<&Screen>,
    corpus: &str,
    modes: &str,
) -> Result<(PathBuf, String), String> {
    let exe = built.exe.as_path();
    let source = built.tree.as_path();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let directory = opts.output.clone().unwrap_or_else(|| {
        root.join("target/perf")
            .join(super::experiment::platform())
            .join(stamp.to_string())
    });
    if let Some(parent) = directory.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    // A fresh directory each run, even when an explicit output path is reused.
    std::fs::create_dir(&directory)
        .map_err(|e| format!("cannot reserve {}: {e}", directory.display()))?;
    let mut manifest =
        std::fs::File::create(directory.join("manifest.txt")).map_err(|e| e.to_string())?;
    writeln!(manifest, "cache={}\ncold_prepare={}\ncycles={}\ncompletion={}\ndiff_scroll={}\nbaseline_platform={}\n",
        opts.cache, opts.cold_prepare, opts.cycles, opts.completion, opts.diff_scroll, super::experiment::platform())
        .map_err(|e| e.to_string())?;
    std::fs::write(
        directory.join("cases.tsv"),
        super::cases::encode(&opts.cases),
    )
    .map_err(|e| e.to_string())?;
    writeln!(manifest,
        "protocol=8\nos={}\narch={}\nexe={}\nfeatures={}\nrepo={}\nselection={}\noid={}\nfile={}\ndiff={}\nscroll={}\nopen={}\nsettle_ms={}\nbreakdown={}\nattribute={}\ncalibrate={}\nruns={}\nsoftware={}\n",
        std::env::consts::OS, std::env::consts::ARCH, exe.display(), opts.features(),
        opts.repo.display(), opts.selection, opts.oid, opts.file, opts.diff, opts.scroll,
        opts.open, opts.settle_ms, opts.breakdown, opts.attribute, opts.calibrate, opts.runs, opts.software).map_err(|e| e.to_string())?;
    // Under `--at` the commit is exactly the one asked for; otherwise
    // source.patch below says what the tree held beyond it.
    writeln!(
        manifest,
        "tree={}\ncommit={}\nat={}\n",
        source.display(),
        built.commit,
        if opts.at.is_empty() { "-" } else { &opts.at }
    )
    .map_err(|e| e.to_string())?;
    writeln!(
        manifest,
        "screen={}\nscreen_hz={}\nwindow={WINDOW_WIDTH}x{WINDOW_HEIGHT}\ncorpus={corpus}\n",
        screen.map_or("platform's choice", |s| s.name.as_str()),
        screen.map_or(0, |s| s.hz),
    )
    .map_err(|e| e.to_string())?;
    writeln!(manifest, "trace_frames={}\nframe_boundary=GUI-delivered-frameSwapped\ndisplay_modes=display-chosen.txt,display-before.txt,display-after.txt\nwindow_metadata=app.log perf_display\nhost_conditions=memory.csv\n", opts.trace_frames)
        .map_err(|e| e.to_string())?;
    std::fs::write(directory.join("display-chosen.txt"), modes).map_err(|e| e.to_string())?;
    capture(&directory, "git-version.txt", root, &["--version"])?;
    // The commit the exe is of, from the build: a shelf hit switches
    // the rig nowhere, so its HEAD may be some other measurement's
    // commit by now.
    std::fs::write(
        directory.join("source-head.txt"),
        format!("{}\n", built.commit),
    )
    .map_err(|e| e.to_string())?;
    if opts.at.is_empty() {
        capture(
            &directory,
            "source-status.txt",
            source,
            &["status", "--short"],
        )?;
        capture(
            &directory,
            "source.patch",
            source,
            &["diff", "HEAD", "--", "crates"],
        )?;
    } else {
        // A rig build is of a clean tree at that commit (`rig::switch`):
        // nothing beyond the commit to record.
        std::fs::write(directory.join("source-status.txt"), "").map_err(|e| e.to_string())?;
        std::fs::write(directory.join("source.patch"), "").map_err(|e| e.to_string())?;
    }
    let output = Command::new("git")
        .current_dir(root)
        .arg("hash-object")
        .arg(exe)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("could not fingerprint the measured executable".into());
    }
    let exe_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
    std::fs::write(directory.join("exe-git-blob-hash.txt"), output.stdout)
        .map_err(|e| e.to_string())?;
    if opts.open {
        capture(
            &directory,
            "repo-head.txt",
            &opts.repo,
            &["rev-parse", "--verify", "--quiet", "HEAD"],
        )?;
        capture(&directory, "repo-refs.txt", &opts.repo, &["show-ref"])?;
        capture(
            &directory,
            "repo-status.txt",
            &opts.repo,
            &["status", "--porcelain=v2", "--untracked-files=no"],
        )?;
    }
    Ok((directory, exe_hash))
}

fn capture(directory: &Path, name: &str, repo: &Path, args: &[&str]) -> Result<(), String> {
    let output = Command::new("git")
        .current_dir(repo)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        if ["repo-head.txt", "repo-refs.txt"].contains(&name) && output.status.code() == Some(1) {
            return std::fs::write(directory.join(name), "unborn\n").map_err(|e| e.to_string());
        }
        return Err(format!(
            "could not capture {name}: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    std::fs::write(directory.join(name), output.stdout).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::{Screen, state_file};
    use crate::perf::options::parse;

    fn options(words: &[&str]) -> crate::perf::Options {
        parse(&words.iter().map(|w| (*w).to_string()).collect::<Vec<_>>()).expect("valid options")
    }

    const SCREEN: Screen = Screen {
        name: String::new(),
        primary: true,
        x: 1920,
        y: -120,
        width: 2560,
        height: 1440,
        hz: 120,
    };

    #[test]
    fn the_window_is_centred_on_the_screen_it_was_given() {
        let text = state_file(&options(&["--repo", "C:/r"]), Some(&SCREEN));
        // 1920 + (2560-1440)/2, -120 + (1440-900)/2
        assert!(text.contains("x = 2480\n"), "{text}");
        assert!(text.contains("y = 150\n"), "{text}");
        assert!(
            text.contains("width = 1440\nheight = 900\nmaximized = false\n"),
            "{text}"
        );
    }

    #[test]
    fn without_a_screen_only_the_size_is_pinned() {
        let text = state_file(&options(&["--repo", "C:/r"]), None);
        assert!(!text.contains("x = "), "{text}");
        assert!(text.contains("width = 1440"), "{text}");
    }

    /// A build with no harness in it has no other way to be handed a
    /// repository, and a build with one is told over `PGG_AUTO_OPEN`.
    #[test]
    fn only_a_harnessless_run_is_given_its_tab_in_the_file() {
        let driven = state_file(&options(&["--repo", "C:\\r\\kotlin"]), None);
        assert!(!driven.contains("[tabs]"), "{driven}");
        let shipped = state_file(&options(&["--repo", "C:\\r\\kotlin", "--shipped"]), None);
        assert!(
            shipped.contains("[tabs]\nactive = 0\npaths = [\"C:/r/kotlin\"]\n"),
            "{shipped}"
        );
    }

    #[test]
    fn a_path_with_a_quote_in_it_still_writes_a_file_that_parses() {
        let text = state_file(
            &options(&["--repo", "C:\\it's mine\\kotlin", "--shipped"]),
            None,
        );
        assert!(text.contains("paths = [\"C:/it's mine/kotlin\"]"), "{text}");
        assert_eq!(super::toml_string("a\"b\\c"), "\"a\\\"b/c\"");
    }
}
