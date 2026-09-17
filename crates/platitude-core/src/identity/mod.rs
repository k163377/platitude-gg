//! Who commits are attributed to, and how they are signed.
//!
//! Both are ordinary git configuration. This module reads them so the UI
//! can ask for an identity before the first commit fails, and can say
//! whether signing is in play — it does not reimplement either.
//!
//! **A signing passphrase is the agent's to ask for.** git delegates to
//! gpg-agent / ssh-agent, which prompt through their own pinentry. A pinentry
//! configured for a terminal (`pinentry-tty`, `pinentry-curses`) cannot work
//! here — the subprocess has no console — so a graphical pinentry is
//! required; that is a configuration matter, settled in git's own
//! configuration.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::config;
use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

mod local;
mod read;
mod verify;
mod write;

// The word for "which of git's files" belongs to the module that owns
// configuration — but it is spelled `identity::ConfigScope`
// everywhere that asks for an identity, so it keeps that door.
pub use crate::config::ConfigScope;
pub use local::{load_local, set_local_identity};
pub use read::{AuthorConfig, Identity, SignatureFormat, SigningConfig, load};
// Reached by the siblings through `use super::*`: the rule for "git was
// given nothing here" belongs beside the parse that first applies it, and
// so does the parse itself — one repository's own file answers with the
// same records, in fewer of them.
use read::{non_empty, parse_config};
pub use verify::{Signature, SignatureStatus, verify_commit};
pub use write::{IdentityWrite, set_identity};
