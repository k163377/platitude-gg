//! `cargo xtask linux offline` — the CI job that nothing else here runs.
//!
//! ci/offline-test.sh is driven by one caller, the offline-test job of
//! .github/workflows/ci.yml, and CI is not run until the distribution
//! phase (CLAUDE.md). A script whose only caller is deferred is a script
//! that drifts: both of the things this verb was written after — a smoke
//! binary built without the harness feature, and an unset list naming a
//! knob the app never had — were readable in the file and unreachable by
//! any run.
//!
//! What it reproduces is the job, not the runner: the same build, the same
//! collection of test executables, the same script, and the same empty
//! network namespace. CI reaches that namespace with `unshare -n` and a
//! sudo; a container is handed one by asking for no network at all, which
//! needs neither. The one difference is the loopback the script opens
//! with: `unshare -n` leaves it down, and here it is already up (measured:
//! /sys/class/net/lo/flags is 0x9, and the image carries no `ip` at all),
//! so that line is the no-op it is written to be allowed to be.

use std::path::Path;
use std::process::Command;

use super::{
    IMAGE, REGISTRY_MOUNT, TARGET_MOUNT, WORK, ensure_image, in_container, mount_path, volume,
};

/// `cargo xtask linux offline` — build online, run offline.
pub(super) fn offline(root: &Path) -> Result<(), String> {
    let app = ensure_image(root, "app", false)?;

    // With the harness (`crate::tree::HARNESS_FEATURE`), because the script
    // drives the app over the `PGG_*` protocol: a build without it answers
    // none of it, stands until the script's timeout kills it, and reports
    // as an exit 124 that says nothing about why.
    println!("building the app with the harness…");
    let build = [
        "cargo",
        "build",
        "-p",
        "platitude-app",
        "--features",
        crate::tree::HARNESS_FEATURE,
    ]
    .map(String::from);
    in_container(root, &app, &build, false)?;

    // Built here and listed again below, the way the job does it: this run
    // is the one whose compile output a reader watches, and the listing
    // that follows it is then a re-check that prints nothing.
    println!("building the test binaries…");
    let tests = ["cargo", "test", "--workspace", "--no-run"].map(String::from);
    in_container(root, &app, &tests, false)?;

    let binaries = test_binaries(root, &app)?;
    if binaries.is_empty() {
        return Err("cargo reported no test executables to run offline".into());
    }
    println!("collected {} test binaries", binaries.len());

    let smoke = format!("{TARGET_MOUNT}/debug/platitude-gg");
    let mut cmd = docker_run(root, false, &[("PGG_SMOKE_BIN", &smoke)]);
    cmd.arg(&app)
        .args(["bash", "ci/offline-test.sh"])
        .args(&binaries);
    let status =
        crate::budget::watched(&mut cmd).map_err(|e| format!("failed to run docker: {e}"))?;
    if status.code() == Some(124) {
        return Err(
            "the smoke run hit ci/offline-test.sh's timeout: the app was started \
             and never finished on its own. A PGG_SMOKE_BIN built without the \
             harness feature is the usual cause — it reads no knob, so nothing \
             tells it to open a repository or to stop."
                .into(),
        );
    }
    if !status.success() {
        return Err(format!(
            "the offline run exited {} — the test binary that failed, or the \
             smoke that did, names itself above",
            status.code().unwrap_or(-1)
        ));
    }
    println!("PASS: the suite and the offscreen smoke ran with no network at all");
    Ok(())
}

/// The test executables cargo built, as paths inside the container. CI
/// reads the same field of the same stream through jq
/// (.github/workflows/ci.yml); this reads it here, so that what runs
/// offline is chosen the way the job chooses it rather than by guessing at
/// file names under deps/.
fn test_binaries(root: &Path, tag: &str) -> Result<Vec<String>, String> {
    let mut cmd = docker_run(root, true, &[]);
    cmd.arg(tag).args([
        "cargo",
        "test",
        "--workspace",
        "--no-run",
        "--message-format=json",
    ]);
    let out = crate::subprocess::run_captured(&mut cmd)?;
    if !out.status.success() {
        return Err("collecting the test executables failed".into());
    }
    Ok(executables(&String::from_utf8_lossy(&out.stdout)))
}

/// The `executable` of every compiler artifact cargo built to be run as a
/// test. xtask links no serde, so each line — one JSON object — is read by
/// hand, and the field that answers is `profile.test`: `target.test` is set
/// on every lib whether or not this artifact is its test binary.
fn executables(stream: &str) -> Vec<String> {
    stream
        .lines()
        .filter(|line| string_field(line, "reason").as_deref() == Some("compiler-artifact"))
        .filter(|line| profile_is_test(line))
        .filter_map(|line| string_field(line, "executable"))
        .collect()
}

/// The value of a `"name":"value"` field, or None when it is absent or
/// null — a library artifact carries `"executable":null`.
fn string_field(line: &str, name: &str) -> Option<String> {
    let key = format!("\"{name}\":\"");
    let at = line.find(&key)? + key.len();
    let rest = &line[at..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Whether the artifact was built as a test, read inside the `profile`
/// object's own braces. Cargo's profile has no nested object, so its first
/// closing brace is its end.
fn profile_is_test(line: &str) -> bool {
    const KEY: &str = "\"profile\":{";
    let Some(at) = line.find(KEY) else {
        return false;
    };
    let rest = &line[at + KEY.len()..];
    let Some(end) = rest.find('}') else {
        return false;
    };
    rest[..end].contains("\"test\":true")
}

/// A `docker run` against this checkout in the app image, up to but not
/// including the tag — the caller adds that and the command.
///
/// `network` false is what this module is for: an empty network namespace,
/// which is the property CI's `unshare -n` establishes. The mounts are the
/// ones every other run here uses, so the build directory is the same
/// docker volume and nothing is compiled twice.
fn docker_run(root: &Path, network: bool, env: &[(&str, &str)]) -> Command {
    let mut cmd = super::carried();
    if !network {
        cmd.arg("--network").arg("none");
    }
    for (name, value) in env {
        cmd.arg("--env").arg(format!("{name}={value}"));
    }
    cmd.arg("--volume")
        .arg(format!("{}:{WORK}", mount_path(root)))
        .arg("--volume")
        .arg(format!("{}:{TARGET_MOUNT}", volume(root, "target")))
        .arg("--volume")
        .arg(format!("{IMAGE}-registry:{REGISTRY_MOUNT}"))
        .arg("--workdir")
        .arg(WORK);
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two artifact lines as cargo writes them, cut down to the fields that
    /// decide: the lib itself, which has `target.test` and no executable,
    /// and its test binary, which has `profile.test`.
    const LIB: &str = r#"{"reason":"compiler-artifact","target":{"test":true,"name":"platitude-core"},"profile":{"opt_level":"0","test":false},"executable":null}"#;
    const TEST: &str = r#"{"reason":"compiler-artifact","target":{"test":true,"name":"platitude-core"},"profile":{"opt_level":"0","test":true},"executable":"/work/target/debug/deps/platitude_core-f444"}"#;
    const BUILD_SCRIPT: &str = r#"{"reason":"build-script-executed","package_id":"cc","executable":"/work/target/debug/build/x/build-script-build"}"#;

    #[test]
    fn only_the_test_binaries_come_out() {
        let stream = format!("{LIB}\n{TEST}\n{BUILD_SCRIPT}\n");
        assert_eq!(
            executables(&stream),
            vec!["/work/target/debug/deps/platitude_core-f444".to_string()],
            "target.test is every lib; profile.test is the artifact to run"
        );
    }

    #[test]
    fn a_stream_with_nothing_to_run_is_empty_not_wrong() {
        assert!(executables("").is_empty());
        assert!(executables("not json at all\n").is_empty());
    }

    /// This job's container comes off the one constructor too, so it is
    /// under the launcher's ticket like every other: a run with no
    /// network is still a run on this machine.
    #[test]
    fn the_offline_container_is_under_the_launchers_ticket() {
        let said = crate::linux::tests::args_of(&docker_run(Path::new("."), false, &[]));
        for mark in [crate::still::UNDER, crate::budget::HELD] {
            assert!(
                said.contains(&format!("{mark}=1")),
                "{mark} is missing from {said:?}"
            );
        }
    }
}
