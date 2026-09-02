//! Immutable run evidence, and the two files that decide what the run is.
//!
//! Failed runs keep their logs too.
//!
//! `settings.toml` and `state.toml` are written here rather than left to
//! the app's defaults because three of the things that move a measurement
//! are settings: whether it fetches while being timed, how many rows the
//! window holds, and which screen the window lands on. A run that did not
//! write them is measuring whatever the platform felt like.
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use super::Options;
use super::display::Screen;

/// The window every run is measured in. The app's own default, said out
/// loud: the number of rows a graph builds is set by the height, and the
/// scroll bench's speed is set by how far the content overhangs it.
const WINDOW_WIDTH: i32 = 1440;
const WINDOW_HEIGHT: i32 = 900;

pub(super) fn open_run(
    directory: &Path,
    opts: &Options,
    screen: Option<&Screen>,
) -> Result<(PathBuf, std::fs::File, std::fs::File), String> {
    let config = directory.join("config");
    std::fs::create_dir(&config).map_err(|e| e.to_string())?;
    std::fs::write(
        config.join("settings.toml"),
        "version = 1\n\n[defaults]\nauto_fetch_minutes = 0\n",
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(config.join("state.toml"), state_file(opts, screen))
        .map_err(|e| e.to_string())?;
    let log = std::fs::File::create(directory.join("app.log")).map_err(|e| e.to_string())?;
    let samples = std::fs::File::create(directory.join("memory.csv")).map_err(|e| e.to_string())?;
    Ok((config, log, samples))
}

/// The window's place and size, and — for a build with no harness in it —
/// the repository to open, which is the only way to ask one for a tab
/// (`platitude-app` §features; there is no command line).
fn state_file(opts: &Options, screen: Option<&Screen>) -> String {
    let mut text = String::from("version = 1\n\n[window]\n");
    if let Some(screen) = screen {
        // Centred on the chosen screen, which is a function of that
        // screen's own bounds and so the same place every run.
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

/// A path as a TOML basic string. Not a literal (`'…'`): those cannot
/// hold a quote of their own at all, and a single quote is a legal
/// character in a path on every platform this runs on — a repository
/// under `C:/it's mine/` would otherwise write a `state.toml` that does
/// not parse, and the run would measure an empty window instead of
/// saying so.
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

pub(super) fn prepare(
    root: &Path,
    exe: &Path,
    opts: &Options,
    screen: Option<&Screen>,
    corpus: &str,
    modes: &str,
) -> Result<PathBuf, String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let directory = opts
        .output
        .clone()
        .unwrap_or_else(|| root.join("target/perf").join(stamp.to_string()));
    if let Some(parent) = directory.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    // Never overwrite another run, even when an explicit output path is reused.
    std::fs::create_dir(&directory)
        .map_err(|e| format!("cannot reserve {}: {e}", directory.display()))?;
    let mut manifest =
        std::fs::File::create(directory.join("manifest.txt")).map_err(|e| e.to_string())?;
    writeln!(manifest,
        "protocol=4\nos={}\narch={}\nexe={}\nfeatures={}\nrepo={}\nselection={}\noid={}\nfile={}\ndiff={}\nscroll={}\nopen={}\nsettle_ms={}\nbreakdown={}\nruns={}\n",
        std::env::consts::OS, std::env::consts::ARCH, exe.display(), opts.features(),
        opts.repo.display(), opts.selection, opts.oid, opts.file, opts.diff, opts.scroll,
        opts.open, opts.settle_ms, opts.breakdown, opts.runs).map_err(|e| e.to_string())?;
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
    capture(&directory, "source-head.txt", root, &["rev-parse", "HEAD"])?;
    capture(
        &directory,
        "source-status.txt",
        root,
        &["status", "--short"],
    )?;
    capture(
        &directory,
        "source.patch",
        root,
        &["diff", "HEAD", "--", "crates"],
    )?;
    let output = Command::new("git")
        .current_dir(root)
        .arg("hash-object")
        .arg(exe)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("could not fingerprint the measured executable".into());
    }
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
    Ok(directory)
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
    /// repository, and a build with one is told over `PG_AUTO_OPEN`.
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

    /// A quote in a path is legal and a TOML literal string cannot hold
    /// one, so the file has to be written as a basic string.
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
