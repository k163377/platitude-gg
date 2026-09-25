//! Remotes: cloning, listing, fetch and push.
//!
//! The only place the application causes network traffic, always by
//! running git. Authentication is git's (`GIT_TERMINAL_PROMPT=0` makes a
//! missing credential helper fail at once).
//!
//! Network commands take a timeout: the backstop for a connection that
//! neither finishes nor fails. Cancellation is the normal early stop.

use std::time::Duration;

mod branch;
mod clone;
mod fetch;
mod list;
mod marks;
mod pull;
mod push;
mod refusal;
mod standing;
mod tags;

pub use self::branch::{
    RemoteBranchState, branch_tip, delete_remote_branch, replace_remote_branch,
};
pub use self::clone::{clone, folder_name_for};
pub use self::fetch::fetch;
pub use self::list::{Remote, Remotes, add, list, read, set_url};
pub use self::marks::{
    OriginMarks, PushDefault, PushMarks, clear_origin, mark_origin, origin_marks, push_marks,
};
pub use self::pull::pull;
pub use self::push::{PushForce, PushSpec, plan_current_push, plan_publish, push};
pub use self::standing::{PushStanding, push_standing, push_target};
pub use self::tags::{
    RemoteTag, delete_remote_tag, list_tags, parse_ls_remote_tags, push_tag, replace_remote_tag,
};

/// Default time budget for commands that talk to a remote.
///
/// Covers an ordinary fetch of a large repository over a slow link without
/// leaving a hung connection running. Raised by the `network_timeout_secs`
/// settings key (its dialog field is still missing — 実装計画 §7).
pub const DEFAULT_NETWORK_TIMEOUT: Duration = Duration::from_secs(180);
