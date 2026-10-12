//! Why a remote could not be reached for want of a credential, as far as
//! git's own words — and OpenSSH's, which git passes through — say. The
//! screen's header line names these three in its own words
//! (デザイン規約 §git が言ったことを読む場所); every other failure to reach
//! a remote (no route, a name that does not resolve, a timeout) is git's
//! first line as it stands.

/// What kept the remote out of reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unreached {
    /// HTTPS, and git had no credential to send: no helper answered and
    /// the prompt is off (`GIT_TERMINAL_PROMPT=0` — prompt.c `git_prompt`,
    /// the minimum to 2.55), or a helper's answer was turned down
    /// (remote-curl.c `Authentication failed for`).
    Credentials,
    /// SSH, and the server took none of the keys offered — OpenSSH's own
    /// `Permission denied (publickey)`, which git repeats as the remote's
    /// words.
    SshKey,
    /// SSH, and the server's host key is not in `known_hosts`: OpenSSH asks
    /// on a terminal and, with none, says `Host key verification failed`.
    HostKey,
}

impl Unreached {
    /// The word the screen picks its sentence by (`Words.unreachedWhy`).
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Credentials => "credentials",
            Self::SshKey => "ssh-key",
            Self::HostKey => "host-key",
        }
    }
}

/// Reads which of the three `said` — git's whole message, `LC_ALL=C` —
/// is, if any.
#[must_use]
pub fn unreached(said: &str) -> Option<Unreached> {
    if said.contains("terminal prompts disabled") || said.contains("Authentication failed for") {
        Some(Unreached::Credentials)
    } else if said.contains("Permission denied (publickey") {
        Some(Unreached::SshKey)
    } else if said.contains("Host key verification failed") {
        Some(Unreached::HostKey)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Copied off git 2.55 with `GIT_TERMINAL_PROMPT=0` and no helper, a
    /// helper's wrong answer, and OpenSSH 9's two refusals as git prints
    /// them.
    #[test]
    fn the_three_wants_are_read_off_gits_and_sshs_words() {
        assert_eq!(
            unreached(
                "fatal: could not read Username for 'https://github.com': terminal prompts disabled"
            ),
            Some(Unreached::Credentials)
        );
        assert_eq!(
            unreached(
                "remote: Invalid username or password.\n\
                 fatal: Authentication failed for 'https://github.com/o/r.git/'"
            ),
            Some(Unreached::Credentials)
        );
        assert_eq!(
            unreached(
                "git@github.com: Permission denied (publickey).\n\
                 fatal: Could not read from remote repository."
            ),
            Some(Unreached::SshKey)
        );
        assert_eq!(
            unreached(
                "Host key verification failed.\n\
                 fatal: Could not read from remote repository."
            ),
            Some(Unreached::HostKey)
        );
    }

    /// No route and no such name are git's own words to show.
    #[test]
    fn a_remote_that_is_simply_away_is_nobodys_want() {
        assert_eq!(
            unreached(
                "fatal: unable to access 'https://github.com/o/r.git/': Could not resolve host: github.com"
            ),
            None
        );
        assert_eq!(
            unreached("fatal: '../gone.git' does not appear to be a git repository"),
            None
        );
    }
}
