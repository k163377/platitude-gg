//! `cargo xtask linux offline` — the offline-test job of
//! .github/workflows/ci.yml, run here so ci/offline-test.sh has a caller
//! before CI runs: the same build, test executables, script and empty
//! network namespace (`--network none` where CI uses `unshare -n`). The
//! one difference: loopback is already up here, so the script's
//! `ip link set lo up` is the no-op it may be.

use std::path::Path;
use std::process::Command;

use super::{
    IMAGE, REGISTRY_MOUNT, TARGET_MOUNT, WORK, ensure_image, in_container, mount_path, volume,
};

/// `cargo xtask linux offline` — build online, run offline.
pub(super) fn offline(root: &Path) -> Result<(), String> {
    let app = ensure_image(root, "app", false)?;

    // With the harness: the script drives the app over `PGG_*`, and a
    // build without it stands until the script's timeout (exit 124).
    println!("building the app with the harness…");
    let build = [
        "cargo",
        "build",
        "--locked",
        "-p",
        "platitude-app",
        "--features",
        crate::app_build::HARNESS_FEATURE,
    ]
    .map(String::from);
    in_container(root, &app, &build, false, None, None)?;

    // Built here and listed again below, as the job does: the compile
    // output shows here, and the listing is a re-check that prints nothing.
    println!("building the test binaries…");
    let tests = ["cargo", "test", "--locked", "--workspace", "--no-run"].map(String::from);
    in_container(root, &app, &tests, false, None, None)?;

    let binaries = test_binaries(root, &app)?;
    if binaries.is_empty() {
        return Err("cargo reported no test executables to run offline".into());
    }
    println!("collected {} test binaries", binaries.len());

    let smoke = format!("{TARGET_MOUNT}/debug/platitude-gg");
    let mut cmd = engine_run(root, false, &[("PGG_SMOKE_BIN", &smoke)]);
    cmd.arg(&app)
        .args(["bash", "ci/offline-test.sh"])
        .args(&binaries);
    let status = crate::budget::watched(&mut cmd)
        .map_err(|e| format!("failed to run {}: {e}", super::engine::NAME))?;
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

/// The test executables cargo built, as paths inside the container —
/// the field CI reads with jq (.github/workflows/ci.yml), so what runs
/// offline is chosen as the job chooses it.
fn test_binaries(root: &Path, tag: &str) -> Result<Vec<String>, String> {
    let mut cmd = engine_run(root, true, &[]);
    cmd.arg(tag).args([
        "cargo",
        "test",
        "--locked",
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
/// test, read by hand (no serde) off `profile.test`: `target.test` is set
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
/// object's braces — it has no nested object, so its first `}` is its end.
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

/// An engine's `run` against this checkout, up to but not including the tag.
/// `network` false is the empty network namespace CI's `unshare -n` gives;
/// the mounts are every other run's, so nothing is compiled twice.
fn engine_run(root: &Path, network: bool, env: &[(&str, &str)]) -> Command {
    let mut cmd = super::carried();
    if !network {
        cmd.arg("--network").arg("none");
    }
    for (name, value) in env {
        cmd.arg("--env").arg(format!("{name}={value}"));
    }
    super::given_commit(&mut cmd, root);
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

    /// Artifact lines as cargo writes them, cut down to the fields that
    /// decide.
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

    /// A run with no network is still a run on this machine.
    #[test]
    fn the_offline_container_is_under_the_launchers_ticket() {
        let said = crate::linux::tests::args_of(&engine_run(Path::new("."), false, &[]));
        for mark in [crate::still::UNDER, crate::budget::HELD] {
            assert!(
                said.contains(&format!("{mark}=1")),
                "{mark} is missing from {said:?}"
            );
        }
    }
}
