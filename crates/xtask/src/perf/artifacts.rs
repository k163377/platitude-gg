//! Immutable run evidence. Failed runs keep their logs too.
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use super::Options;

pub(super) fn open_run(
    directory: &Path,
) -> Result<(PathBuf, std::fs::File, std::fs::File), String> {
    let config = directory.join("config");
    std::fs::create_dir(&config).map_err(|e| e.to_string())?;
    std::fs::write(
        config.join("settings.toml"),
        "version = 1\n\n[defaults]\nauto_fetch_minutes = 0\n",
    )
    .map_err(|e| e.to_string())?;
    let log = std::fs::File::create(directory.join("app.log")).map_err(|e| e.to_string())?;
    let samples = std::fs::File::create(directory.join("memory.csv")).map_err(|e| e.to_string())?;
    Ok((config, log, samples))
}

pub(super) fn prepare(root: &Path, exe: &Path, opts: &Options) -> Result<PathBuf, String> {
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
        "protocol=2\nos={}\narch={}\nexe={}\nrepo={}\nselection={}\noid={}\nfile={}\ndiff={}\nscroll={}\nopen={}\nsettle_ms={}\nbreakdown={}\nruns={}\n",
        std::env::consts::OS, std::env::consts::ARCH, exe.display(), opts.repo.display(),
        opts.selection, opts.oid, opts.file, opts.diff, opts.scroll, opts.open, opts.settle_ms,
        opts.breakdown, opts.runs).map_err(|e| e.to_string())?;
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
