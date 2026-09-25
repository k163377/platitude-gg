//! Who commits are attributed to, and how they are signed — both ordinary
//! git configuration, read so the UI can ask for an identity before the
//! first commit fails and say whether signing is in play.
//!
//! A signing passphrase is the agent's to ask for. The subprocess has no
//! console, so a terminal pinentry (`pinentry-tty`, `pinentry-curses`)
//! cannot work; a graphical one is the user's configuration matter.

use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::config;
use crate::error::GitError;
use crate::process::{GitCommand, GitExecutor};

mod local;
mod read;
mod verify;
mod write;

// Callers asking for an identity spell it `identity::ConfigScope`.
pub use crate::config::ConfigScope;
pub use local::{load_local, set_local_identity};
pub use read::{AuthorConfig, Identity, SignatureFormat, SigningConfig, load};
// Reached by the siblings through `use super::*`.
use read::{non_empty, parse_config};
pub use verify::{Signature, SignatureStatus, verify_commit};
pub use write::{IdentityWrite, set_identity};
