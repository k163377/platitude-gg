//! `cargo xtask relied-messages master` — `master`'s git, built where the run
//! stands, and every witness the table names run under it. A test there
//! has real git walk the path to its words, so a change no line of the
//! sources shows (another `die()` reached first) turns it red.
//!
//! Built without what no witness reaches — HTTPS, OpenSSL (git hashes with
//! its own code), translations, Perl, Python, Tcl/Tk, and the Rust half
//! `master` builds unless told not to — so a C compiler, make and zlib's
//! headers are all it takes (the container's `git-build` stage, CI's
//! weekly job).

use std::path::Path;
use std::process::Command;

use super::table;

const UPSTREAM: &str = "https://github.com/git/git";

/// What the build leaves out (git's Makefile).
const LEFT_OUT: [&str; 8] = [
    "NO_CURL=1",
    "NO_EXPAT=1",
    "NO_GETTEXT=1",
    "NO_OPENSSL=1",
    "NO_PERL=1",
    "NO_PYTHON=1",
    "NO_RUST=1",
    "NO_TCLTK=1",
];

pub(super) fn run(root: &Path) -> Result<(), String> {
    if !cfg!(target_os = "linux") {
        return Err(
            "git's own build wants Linux here: `cargo xtask linux relied-messages master`"
                .to_string(),
        );
    }
    let forms = table::read(root)?;
    let mut witnesses: Vec<&str> = forms
        .iter()
        .flat_map(|form| form.witnesses.iter().map(String::as_str))
        .collect();
    witnesses.sort_unstable();
    witnesses.dedup();

    let bin = build()?;
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = vec![bin.clone()];
    paths.extend(std::env::split_paths(&path));
    let path = std::env::join_paths(paths).map_err(|e| e.to_string())?;
    let mut version = Command::new(bin.join("git"));
    version.arg("--version");
    println!("{}", succeed(version, "git --version")?.trim());

    println!("running {} witnesses under it…", witnesses.len());
    let mut test = Command::new("cargo");
    test.current_dir(root)
        .env("PATH", &path)
        .args([
            "test",
            "--locked",
            "-p",
            "platitude-core",
            "--test",
            "it",
            "--",
        ])
        .args(["--include-ignored", "--exact"])
        .args(&witnesses);
    let tested = crate::subprocess::run_captured(&mut test)?;
    let said = String::from_utf8_lossy(&tested.stdout);
    print!("{said}");
    eprint!("{}", String::from_utf8_lossy(&tested.stderr));
    if !tested.status.success() {
        return Err(
            "a witness failed under git's master: git changed what it says, or the way to it, \
             for a form the table names — see the test above"
                .to_string(),
        );
    }
    // `--exact` selects nothing for a name the binary does not hold, and
    // passes: a run is green only where every witness ran and passed.
    let unrun = unrun(&witnesses, &said);
    if unrun.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "witnesses that did not run under git's master — no test of that full name: {}",
            unrun.join(", ")
        ))
    }
}

/// The witnesses a test run's report does not name as passed.
fn unrun<'a>(witnesses: &[&'a str], said: &str) -> Vec<&'a str> {
    let passed: Vec<&str> = said
        .lines()
        .filter_map(|line| line.strip_prefix("test ")?.strip_suffix(" ... ok"))
        .collect();
    witnesses
        .iter()
        .copied()
        .filter(|witness| !passed.contains(witness))
        .collect()
}

/// git's master, cloned one commit deep and installed into a folder of
/// the run's own: the folder its `git` is in.
fn build() -> Result<std::path::PathBuf, String> {
    let place = crate::temp::root().join("relied-messages-master");
    if place.exists() {
        std::fs::remove_dir_all(&place).map_err(|e| format!("{}: {e}", place.display()))?;
    }
    std::fs::create_dir_all(&place).map_err(|e| format!("{}: {e}", place.display()))?;
    let source = place.join("src");
    let installed = place.join("git");

    println!("cloning git's master…");
    let mut clone = Command::new("git");
    clone
        .args([
            "clone",
            "--quiet",
            "--depth=1",
            "--branch",
            "master",
            UPSTREAM,
        ])
        .arg(&source);
    succeed(clone, "git clone")?;

    println!("building it…");
    let jobs = std::thread::available_parallelism().map_or(2, usize::from);
    let log = place.join("build.log");
    let file = std::fs::File::create(&log).map_err(|e| format!("{}: {e}", log.display()))?;
    let mut make = Command::new("make");
    make.arg("-C")
        .arg(&source)
        .arg(format!("-j{jobs}"))
        .arg(format!("prefix={}", installed.display()))
        .args(LEFT_OUT)
        .arg("install")
        .stdout(file.try_clone().map_err(|e| e.to_string())?)
        .stderr(file);
    let built = make.status().map_err(|e| format!("make: {e}"))?;
    if !built.success() {
        // A parallel build ends in every job's `Error 1`; what broke is
        // what the compiler said before them, past make's own lines and
        // the quiet build's `    CC x.o`.
        let said = std::fs::read_to_string(&log).unwrap_or_default();
        let first: Vec<&str> = said
            .lines()
            .filter(|line| {
                let quiet = line.split_whitespace().next().is_some_and(|word| {
                    word.len() <= 6 && word.chars().all(|c| c.is_ascii_uppercase())
                });
                !line.starts_with("make") && !quiet
            })
            .take(40)
            .collect();
        return Err(format!(
            "git's master did not build ({}):\n{}",
            log.display(),
            first.join("\n")
        ));
    }
    Ok(installed.join("bin"))
}

fn succeed(mut command: Command, what: &str) -> Result<String, String> {
    let output = crate::subprocess::run_captured(&mut command)?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(format!(
            "{what} — {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::unrun;

    /// A name the binary does not hold is selected by nothing and passes
    /// the run all the same — here, the full name with its inner module
    /// left out; a witness the run reports ignored did not pass either.
    #[test]
    fn a_witness_the_run_does_not_report_passed_is_unrun() {
        let said = "\nrunning 2 tests\n\
                    test rebase::periodic::a_dirty_tree_stops ... ok\n\
                    test carried_apply::a_change_here ... ignored\n\n\
                    test result: ok. 1 passed; 0 failed; 1 ignored\n";
        assert_eq!(
            unrun(
                &[
                    "rebase::periodic::a_dirty_tree_stops",
                    "rebase::a_dirty_tree_stops",
                    "carried_apply::a_change_here",
                ],
                said
            ),
            ["rebase::a_dirty_tree_stops", "carried_apply::a_change_here"]
        );
    }
}
