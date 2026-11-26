//! The identity as `.mailmap` spells it — how the graph and the details
//! pane show the commits it records (`%aN` / `%aE`, `parse::log`), the
//! spelling faces are drawn from and told apart by (`avatar::key`).

use super::*;

/// `identity` through `git check-mailmap`: the name and address `.mailmap`
/// gives the pair, or the pair as it is where it maps neither. Nothing to
/// look up without an address. When it is read again is the session's
/// (`RepoSession::read_identity_shown`).
pub async fn shown(
    executor: &GitExecutor,
    workdir: &Path,
    identity: &Identity,
    cancel: &CancellationToken,
) -> Result<Identity, GitError> {
    let Some(email) = identity.email.as_deref() else {
        return Ok(identity.clone());
    };
    let contact = match identity.name.as_deref() {
        Some(name) => format!("{name} <{email}>"),
        None => format!("<{email}>"),
    };
    // A name may start with a dash.
    let cmd = GitCommand::new().cwd(workdir).args([
        "check-mailmap",
        "--end-of-options",
        contact.as_str(),
    ]);
    let out = executor.run(cmd, cancel).await?;
    parse_contact(&out.stdout_utf8()).ok_or_else(|| GitError::UnexpectedOutput {
        command: "git check-mailmap".to_string(),
        message: "the contact is not `Name <address>`".to_string(),
    })
}

/// One line as `check-mailmap` prints it: `Name <address>`, or `<address>`
/// where neither the identity nor `.mailmap` gives a name (v2.43.0
/// `builtin/check-mailmap.c`).
fn parse_contact(text: &str) -> Option<Identity> {
    let inner = text.trim_end_matches('\n').strip_suffix('>')?;
    let (name, email) = inner.rsplit_once('<')?;
    let non_empty = |s: &str| (!s.is_empty()).then(|| s.to_string());
    Some(Identity {
        name: non_empty(name.trim_end_matches(' ')),
        email: non_empty(email),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_contact_reads_back_in_its_two_halves() {
        let both = parse_contact("Tanaka Yuki <yuki.tanaka@example.com>\n").expect("a contact");
        assert_eq!(both.name.as_deref(), Some("Tanaka Yuki"));
        assert_eq!(both.email.as_deref(), Some("yuki.tanaka@example.com"));
        let bare = parse_contact("<yuki@example.com>\n").expect("a contact");
        assert_eq!(bare.name, None);
        assert_eq!(bare.email.as_deref(), Some("yuki@example.com"));
        assert_eq!(parse_contact("no address\n"), None);
    }
}
