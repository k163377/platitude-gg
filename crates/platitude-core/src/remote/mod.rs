//! Remotes: listing, fetch and push.
//!
//! This is the only place the application causes network traffic, and it
//! causes it the same way a terminal would — by running git. Authentication
//! is git's (`GIT_TERMINAL_PROMPT=0` keeps a missing credential helper from
//! hanging the process instead of failing).
//!
//! Network commands take a timeout rather than running unbounded.
//! Cancellation is the normal way to stop one early; the timeout is the
//! backstop for a connection that neither finishes nor fails.

use std::time::Duration;

mod branch;
mod fetch;
mod list;
mod push;
mod standing;
mod tags;

pub use self::branch::{RemoteBranchState, branch_tip, delete_remote_branch, rename_remote_branch};
pub use self::fetch::fetch;
pub use self::list::{
    PushDefault, Remote, Remotes, add, clear_push_default, list, push_default, read,
    set_push_default, set_url,
};
pub use self::push::{PushForce, PushSpec, plan_current_push, plan_publish, push};
pub use self::standing::{PushStanding, push_standing};
pub use self::tags::{RemoteTag, list_tags, parse_ls_remote_tags};

/// Default time budget for commands that talk to a remote.
///
/// Three minutes covers an ordinary fetch of a large repository over a slow
/// link without leaving a hung connection running for an hour. A user on a
/// genuinely slow line can raise it (the `network_timeout_secs` settings
/// key; the dialog's input field is what is still missing — 実装計画 §7).
pub const DEFAULT_NETWORK_TIMEOUT: Duration = Duration::from_secs(180);
