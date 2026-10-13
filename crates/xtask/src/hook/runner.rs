//! Git supplies the hook wrapper's sh on Windows too, so the cloud
//! guard needs neither Cargo nor Claude's hook-specific Git Bash.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use super::payload::{own_string_field, own_value_field};

#[test]
fn every_hook_skips_cargo_only_in_the_cloud_and_preserves_local_io() {
    let yard = crate::yard::Yard::new("cloud-hooks");
    let bin = yard.join("bin");
    let repo = yard.join("repo");
    fs::create_dir_all(&bin).unwrap();
    fs::create_dir_all(repo.join("nested folder")).unwrap();
    fs::create_dir_all(repo.join(".claude/hooks")).unwrap();
    let runner = repo.join(".claude/hooks/runner.sh");
    fs::write(
        &runner,
        include_bytes!("../../../../.claude/hooks/runner.sh"),
    )
    .unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .arg(&repo)
            .status()
            .unwrap()
            .success()
    );
    fs::write(yard.join("stdin"), "payload\n").unwrap();
    let cargo = bin.join("cargo");
    fs::write(&cargo, "#!/bin/sh\nprintf 'cargo-called\\n'\nprintf '%s\\n' \"$@\"\ngit rev-parse --show-prefix\nIFS= read -r input\nprintf '%s' \"$input\"\nprintf 'cargo-stderr' >&2\nexit \"$CLOUD_TEST_EXIT\"\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755)).unwrap();
        fs::set_permissions(&runner, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let hooks = configured_hooks();
    for args in &hooks {
        for remote in [
            None,
            Some(""),
            Some("false"),
            Some("TRUE"),
            Some("1"),
            Some("true"),
        ] {
            let output = run_hook(&yard, args, &repo.join("nested folder"), remote, 23);
            if remote == Some("true") {
                assert!(output.status.success(), "{args:?}: {output:?}");
                assert!(
                    output.stdout.is_empty() && output.stderr.is_empty(),
                    "{output:?}"
                );
            } else {
                assert_local(&output, &args[3], "nested folder/", 23);
            }
        }
    }
    // Success is preserved too, including when Git's alias starts at
    // the repository root and there is no prefix to restore.
    let output = run_hook(&yard, &hooks[0], &repo, None, 0);
    assert_local(&output, &hooks[0][3], "", 0);
    #[cfg(windows)]
    for remote in [None, Some("true")] {
        let mut child = Command::new("powershell.exe");
        child
            .args(["-NoProfile", "-NonInteractive", "-Command"])
            .arg(format!("git {}; exit $LASTEXITCODE", hooks[0].join(" ")));
        let mut output = run_command(child, &yard, &repo, remote, 23);
        if remote == Some("true") {
            assert!(output.status.success(), "{output:?}");
            assert!(
                output.stdout.is_empty() && output.stderr.is_empty(),
                "{output:?}"
            );
        } else {
            // PowerShell writes native output as lines; the Cargo call,
            // payload and failure must survive that same existing shell.
            output.stdout = String::from_utf8(output.stdout)
                .unwrap()
                .replace("\r\n", "\n")
                .trim_end_matches('\n')
                .as_bytes()
                .to_vec();
            assert_local(&output, &hooks[0][3], "", 23);
        }
    }
}

fn configured_hooks() -> Vec<Vec<String>> {
    let settings = include_str!("../../../../.claude/settings.json");
    let mut hooks = Vec::new();
    for fragment in settings.split("\"type\": \"command\"").skip(1) {
        let hook = format!("{{\"type\":\"command\"{fragment}");
        assert!(
            own_value_field(&hook, "shell").is_none(),
            "no forced hook shell"
        );
        assert!(
            own_value_field(&hook, "args").is_none(),
            "keep the existing command form"
        );
        let command = own_string_field(&hook, "command").expect("a hook command");
        let args: Vec<String> = command
            .strip_prefix("git ")
            .unwrap()
            .split_whitespace()
            .map(str::to_string)
            .collect();
        assert_eq!(args.len(), 4);
        assert_eq!(args[0], "-c");
        assert!(args[1].starts_with(&format!("alias.{}=!", args[2])));
        assert_eq!(
            args[1].split_once('!').unwrap().1,
            ".claude/hooks/runner.sh"
        );
        hooks.push(args);
    }
    assert!(!hooks.is_empty(), "the settings contain command hooks");
    hooks
}

fn run_hook(yard: &Path, args: &[String], cwd: &Path, remote: Option<&str>, exit: i32) -> Output {
    let mut child = Command::new("git");
    child.args(args);
    run_command(child, yard, cwd, remote, exit)
}

fn run_command(
    mut child: Command,
    yard: &Path,
    cwd: &Path,
    remote: Option<&str>,
    exit: i32,
) -> Output {
    let mut paths = vec![yard.join("bin")];
    paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    child
        .current_dir(cwd)
        .env("PATH", std::env::join_paths(paths).unwrap())
        .env("CLOUD_TEST_EXIT", exit.to_string())
        .env_remove("BASH_ENV")
        .stdin(fs::File::open(yard.join("stdin")).unwrap());
    if let Some(value) = remote {
        child.env("CLAUDE_CODE_REMOTE", value);
    } else {
        child.env_remove("CLAUDE_CODE_REMOTE");
    }
    child.output().expect("run the configured Git hook")
}

fn assert_local(output: &Output, event: &str, prefix: &str, exit: i32) {
    assert_eq!(output.status.code(), Some(exit), "{output:?}");
    assert_eq!(output.stdout, format!("cargo-called\nrun\n--quiet\n-p\nxtask\n--profile\nhooks\n--\nhook\n{event}\n{prefix}\npayload").as_bytes());
    assert_eq!(output.stderr, b"cargo-stderr");
}
