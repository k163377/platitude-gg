//! The cloud exception lives before Cargo in settings.json, so starting
//! a Claude Code on the web session never builds the hook runner.

use std::process::Command;

use super::payload::string_field;

#[test]
fn every_hook_skips_cargo_only_in_the_cloud_and_preserves_local_io() {
    let settings = include_str!("../../../../.claude/settings.json");
    let hooks: Vec<_> = settings.split("\"type\": \"command\"").skip(1).collect();
    assert!(!hooks.is_empty(), "the settings contain command hooks");
    for hook in hooks {
        assert_eq!(string_field(hook, "shell").as_deref(), Some("bash"));
        assert!(!hook.contains("\"args\""), "the guard needs shell form");
        let command = string_field(hook, "command").expect("a hook command");
        let local = command
            .strip_prefix("test \"$CLAUDE_CODE_REMOTE\" = true || ")
            .expect("every command skips Cargo before it can start");
        let arguments = local.strip_prefix("cargo ").expect("the local Cargo hook");
        let expected = format!("cargo-called\n{}\npayload", arguments.replace(' ', "\n"));
        for remote in [
            None,
            Some(""),
            Some("false"),
            Some("TRUE"),
            Some("1"),
            Some("true"),
        ] {
            // A shell function witnesses the call, stdin and failure without
            // starting a real Cargo or writing shared test state.
            let script = format!(
                "cargo() {{ printf 'cargo-called\\n'; printf '%s\\n' \"$@\"; cat; return 23; }}; \
                 printf '%s' payload | {{ {command}; }}"
            );
            let mut child = shell();
            child.arg("-c").arg(script).env_remove("BASH_ENV");
            if let Some(value) = remote {
                child.env("CLAUDE_CODE_REMOTE", value);
            } else {
                child.env_remove("CLAUDE_CODE_REMOTE");
            }
            let output = child.output().expect("run the configured hook shell");
            assert!(output.stderr.is_empty(), "{command}: {:?}", output);
            if remote == Some("true") {
                assert!(output.status.success(), "{command}: {:?}", output);
                assert!(output.stdout.is_empty(), "cloud hook ran Cargo: {command}");
            } else {
                assert_eq!(output.status.code(), Some(23), "{command}: {remote:?}");
                assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
            }
        }
    }
}

fn shell() -> Command {
    #[cfg(windows)]
    {
        // Claude uses Git Bash on Windows; System32's bash.exe is WSL.
        let git = Command::new("git").arg("--exec-path").output().unwrap();
        assert!(git.status.success());
        let exec_path = String::from_utf8(git.stdout).unwrap();
        let root = std::path::Path::new(exec_path.trim())
            .ancestors()
            .nth(3)
            .unwrap();
        Command::new(root.join("bin/bash.exe"))
    }
    #[cfg(not(windows))]
    {
        Command::new("sh")
    }
}
