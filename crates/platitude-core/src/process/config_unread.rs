//! A read run once more when git could not open a configuration file for it.
//!
//! git replaces a configuration file by writing `<file>.lock` and renaming it
//! over the file. On Windows a git started inside that moment cannot open
//! the file: a repository command dies over it, and `git config --get*`
//! carries on without it, answering from the files it could read — a key
//! set only there reads as unset. The next git reads the file whole, so a
//! read is asked once more; a write never is, since it may have written
//! (internal-docs/gitの扱い.md §設定の書き換えと重なった読み). A file that
//! stays unreadable fails the second run the same way, and that answer
//! stands.
//!
//! The words are git's own, the same in every release from 2.43 through
//! 2.56 and in Git for Windows (config.c `repo_read_config`, wrapper.c
//! `warn_on_fopen_errors` and `access_or_die`); the executor runs git under
//! `LC_ALL=C`, so they are never translated.

use std::ffi::OsString;

/// What a repository command dies with when a configuration file it found
/// would not open.
const CONFIG_UNREAD: &str = "unknown error occurred while reading the configuration files";

/// What git says of a file it could not open — as a warning before going
/// on, or as the fatal line where it asked for the file first. A URL in the
/// quotes is a remote out of reach, not a file.
const FILE_UNOPENED: &str = "unable to access '";

/// Whether a run that ended with `code` and `stderr` is asked once more.
/// `restartable`: nothing it printed has been handed on yet (a buffered
/// read, or a stream that died before its first byte).
pub(super) fn again(
    args: &[OsString],
    code: i32,
    stderr: &str,
    restartable: bool,
    buffered: bool,
) -> bool {
    if !restartable || !reads_only(args) {
        return false;
    }
    if code == 128 {
        return died_over_a_file(stderr);
    }
    // Only `git config` goes on without the file: a repository command that
    // warned and answered still answered about the repository.
    buffered && is_config_read(args) && names_a_file(stderr, "warning: ")
}

/// Whether git's words are the ones it dies with over a configuration file
/// it could not open — what a read is asked once more over, and what an
/// opening that is still refused on that run is named by.
pub(crate) fn died_over_a_file(stderr: &str) -> bool {
    stderr.contains(CONFIG_UNREAD) || names_a_file(stderr, "fatal: ")
}

/// Whether a line opening with `lead` names a file git could not open.
fn names_a_file(stderr: &str, lead: &str) -> bool {
    stderr.lines().any(|line| {
        line.strip_prefix(lead)
            .and_then(|rest| rest.strip_prefix(FILE_UNOPENED))
            .and_then(|rest| rest.split_once('\''))
            .is_some_and(|(file, _)| !file.contains("://"))
    })
}

/// The subcommand and what follows it: the global options in front (`-c
/// <setting>`, `-C <folder>`, `--no-pager`) are passed over.
fn command(args: &[OsString]) -> Option<(&str, Vec<&str>)> {
    let mut words = args.iter().map(|a| a.to_str().unwrap_or(""));
    while let Some(word) = words.next() {
        match word {
            "-c" | "-C" => {
                words.next();
            }
            _ if word.starts_with('-') => {}
            _ => return Some((word, words.collect())),
        }
    }
    None
}

/// Whether the command only reads: the subcommands that never write, and
/// the reading forms of the ones that can.
fn reads_only(args: &[OsString]) -> bool {
    let Some((sub, rest)) = command(args) else {
        return false;
    };
    let positional = || rest.iter().filter(|w| !w.starts_with('-'));
    let flagged = |flags: &[&str]| rest.iter().any(|w| flags.contains(w));
    match sub {
        "rev-parse" | "rev-list" | "log" | "show" | "status" | "diff" | "diff-index"
        | "diff-tree" | "diff-files" | "cat-file" | "ls-files" | "ls-tree" | "ls-remote"
        | "merge-base" | "for-each-ref" | "show-ref" | "check-attr" | "check-ignore"
        | "describe" | "name-rev" | "blame" | "var" | "version" => true,
        "config" => is_config_read(args),
        "stash" => matches!(positional().next(), Some(&"list" | &"show")),
        "worktree" => positional().next() == Some(&"list"),
        // One name and no value reads it; `-d` with one name deletes it.
        "symbolic-ref" => positional().count() == 1 && !flagged(&["-d", "--delete"]),
        "remote" => matches!(positional().next(), None | Some(&"get-url" | &"show")),
        "branch" | "tag" => flagged(&["--list", "-l"]),
        _ => false,
    }
}

/// `git config` asked for values, not to change them: the forms that only
/// read (`--get*`, `--list`, and from 2.46 `get` / `list`).
fn is_config_read(args: &[OsString]) -> bool {
    let Some(("config", rest)) = command(args) else {
        return false;
    };
    rest.iter().any(|w| {
        matches!(
            *w,
            "--get"
                | "--get-all"
                | "--get-regexp"
                | "--get-urlmatch"
                | "--get-color"
                | "--get-colorbool"
                | "--list"
                | "-l"
        )
    }) || matches!(
        rest.iter().find(|w| !w.starts_with('-')),
        Some(&"get" | &"list")
    )
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::{again, died_over_a_file};

    fn args(words: &[&str]) -> Vec<OsString> {
        words.iter().map(OsString::from).collect()
    }

    /// git 2.55 on Windows, the global configuration replaced under the read.
    const DIED_ON_WINDOWS: &str = "warning: unable to access 'C:/Users/u/.gitconfig': Permission denied\n\
         warning: unable to access 'C:/Users/u/.gitconfig': Permission denied\n\
         fatal: unknown error occurred while reading the configuration files";
    /// git 2.43 in the container, the repository's own configuration a link
    /// to itself: `access_or_die` asks before it opens.
    const DIED_ON_POSIX: &str = "warning: unable to access '.git/config': Too many levels of symbolic links\n\
         fatal: unable to access '.git/config': Too many levels of symbolic links";
    /// What `git config --get*` says as it goes on without the file.
    const WENT_ON: &str = "warning: unable to access '.git/config': Permission denied";
    /// A remote out of reach says the same words over a URL.
    const UNREACHED: &str =
        "fatal: unable to access 'https://example.com/r.git/': Could not resolve host: example.com";
    /// A refusal of its own that happens to follow a warning about a file.
    const REFUSED_BESIDE: &str = "warning: unable to access '.gitattributes': Permission denied\n\
         fatal: not a git repository (or any of the parent directories): .git";

    #[test]
    fn a_read_that_died_over_a_configuration_file_runs_again() {
        let opening = args(&["rev-parse", "--show-toplevel", "--absolute-git-dir"]);
        assert!(again(&opening, 128, DIED_ON_WINDOWS, true, true));
        assert!(again(&opening, 128, DIED_ON_POSIX, true, true));
        // A stream that has not printed can start over; one that has cannot.
        let walk = args(&["log", "-z", "--format=%H"]);
        assert!(again(&walk, 128, DIED_ON_WINDOWS, true, false));
        assert!(!again(&walk, 128, DIED_ON_WINDOWS, false, false));
    }

    #[test]
    fn only_once_it_is_a_read_and_only_over_a_file() {
        // A write may have written before it died.
        for write in [
            args(&["commit", "-m", "x"]),
            args(&["config", "--local", "user.name", "x"]),
            args(&["config", "--global", "--unset", "merge.guitool"]),
            args(&["stash", "push", "-u"]),
            args(&["symbolic-ref", "HEAD", "refs/heads/x"]),
            args(&["symbolic-ref", "-d", "HEAD"]),
            args(&["symbolic-ref", "--delete", "HEAD"]),
            args(&["-c", "core.safecrlf=false", "apply", "--whitespace=nowarn"]),
        ] {
            assert!(
                !again(&write, 128, DIED_ON_WINDOWS, true, true),
                "{write:?}"
            );
        }
        // A remote out of reach is no file, and another 128 is git's answer.
        let reach = args(&["ls-remote", "--tags", "origin"]);
        assert!(!again(&reach, 128, UNREACHED, true, true));
        let status = args(&["status", "--porcelain=v2"]);
        assert!(!again(&status, 128, REFUSED_BESIDE, true, true));
        assert!(!again(&status, 1, DIED_ON_WINDOWS, true, true));
    }

    #[test]
    fn a_config_read_that_went_on_without_the_file_runs_again() {
        let read = args(&["config", "-z", "--get-regexp", "^user\\."]);
        assert!(again(&read, 1, WENT_ON, true, true), "it read as unset");
        assert!(
            again(&read, 0, WENT_ON, true, true),
            "it answered from the rest"
        );
        assert!(!again(&read, 0, "", true, true));
        let newer = args(&["config", "get", "user.name"]);
        assert!(again(&newer, 1, WENT_ON, true, true));
        // A repository command that warned still answered about the
        // repository.
        let status = args(&["status", "--porcelain=v2"]);
        assert!(!again(&status, 0, WENT_ON, true, true));
    }

    #[test]
    fn the_reading_forms_of_the_commands_that_can_also_write() {
        for read in [
            args(&["-c", "core.quotepath=false", "log", "-1"]),
            args(&["stash", "list"]),
            args(&["worktree", "list", "--porcelain"]),
            args(&["symbolic-ref", "-q", "HEAD"]),
            args(&["remote"]),
            args(&["remote", "get-url", "origin"]),
            args(&["branch", "--list", "x"]),
            args(&["config", "--type=bool", "--get", "filter.lfs.required"]),
        ] {
            assert!(again(&read, 128, DIED_ON_POSIX, true, true), "{read:?}");
        }
        for write in [
            args(&["worktree", "add", "x"]),
            args(&["remote", "add", "origin", "u"]),
            args(&["branch", "x"]),
            args(&["tag", "v1"]),
        ] {
            assert!(!again(&write, 128, DIED_ON_POSIX, true, true), "{write:?}");
        }
    }

    /// The opening names its refusal by the same words: the fatal line, not
    /// a warning beside another refusal or a remote's URL.
    #[test]
    fn only_the_fatal_words_name_an_unreadable_configuration() {
        assert!(died_over_a_file(DIED_ON_WINDOWS));
        assert!(died_over_a_file(DIED_ON_POSIX));
        assert!(!died_over_a_file(WENT_ON), "it went on");
        assert!(!died_over_a_file(REFUSED_BESIDE));
        assert!(!died_over_a_file(UNREACHED), "a URL is no file");
    }
}
