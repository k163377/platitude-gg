//! Per-tab lifecycle + error surface + refresh entry points.

use std::sync::Arc;

use qtbridge::{QObjectHolder, qobject};

use crate::hub::{Feed, Hub, TabMsg};
use crate::urlpath::picker_folder_url;

use super::qml_register;

mod drain;
mod ops_config;
mod ops_conflict;
mod ops_remote;
mod ops_stage;
mod qobject;
mod state;

/// Failed fetches in a row before the timer is stopped — more than one,
/// so a brief offline blip (a lid closed on a train) does not stop it.
const FETCH_FAILURES_BEFORE_STOP: i32 = 3;

pub struct RepoTab {
    tab_id: i32,
    state: String,
    title: String,
    repo_path: String,
    /// Where the picker opens: the folder this repository sits in, as a
    /// URL — repositories are kept side by side far more often than
    /// inside one another, so the dialog's own "last folder" is not used.
    picker_folder_url: String,
    /// Why the repository would not open: git's own words. Read only on
    /// the `other` kind — for the two the application can name itself,
    /// the heading says it and this would only repeat it in lower case.
    error: String,
    /// Which of the three the failure was (`plain` / `bare` / `other`),
    /// the same word the picker's dialog branches on.
    error_kind: String,
    /// The folder that would not open, for the line under the heading.
    error_path: String,
    last_error: String,
    /// Whether `last_error` is fetch news. Only that line is taken down
    /// by a later fetch that lands — the state it described is over. A
    /// background read's line has no such ending and waits for a reader.
    last_error_from_fetch: bool,
    tags_shown: bool,
    /// Write commands currently in flight (they are serialized per session,
    /// but requests can queue up).
    busy_count: i32,
    busy_op: String,
    /// Merge tool names the settings field can offer, joined by U+001F the
    /// way the graph's label records are. Empty means none to offer, which
    /// is a working state — the field takes a typed name either way.
    merge_tools: String,
    /// A candidate read is out. Asking git what is installed takes about
    /// eight seconds on Windows, so the field says so rather than looking
    /// like it has nothing.
    merge_tools_loading: bool,
    /// Configured remote names joined by U+001F, so the publish question
    /// can offer them without a model of its own (`remoteAt` answers one
    /// at a time, and a slot is not something a binding can follow).
    remote_names: String,
    /// Last answer to `checkRemoteBranch`, as `<remote>\u{1f}<branch>`.
    /// Empty while a read is out — the question the answer belongs to has
    /// to be checked, because the box may have moved on to another name.
    remote_branch_asked: String,
    /// What a push under that name would meet over there
    /// (`platitude_core::remote::RemoteBranchState`): `free` /
    /// `fast-forward` / `refused` / `unknown` / `unreachable`. One string
    /// rather than a pair of flags — the five are exclusive, and the two
    /// that ask the question to hold back are not the same two that ask it
    /// to warn.
    remote_branch_state: String,
    /// The commit that remote advertised for the name, hex. What an
    /// overwrite leases against — the question shows this state, so this is
    /// the commit it showed.
    remote_branch_tip: String,
    /// How many commits that tip has that this branch does not: what an
    /// overwrite would take off it.
    remote_branch_theirs: i32,
    /// Last answer to `checkBranchDelete`: the branch asked about, and
    /// whether it is merged into what `branch --delete` measures against
    /// (its upstream, or HEAD without one). Empty branch means nothing
    /// asked; the menu that reads it checks the echo, because it may be
    /// open over another row by now.
    branch_delete_asked: String,
    branch_delete_merged: bool,
    /// Last answer to `checkPublish`: how much of a range a remote has.
    publish_range: String,
    publish_total: i32,
    publish_published: i32,
    /// Last answer to `checkInHistory`: the commit asked about, and whether
    /// HEAD can reach it. Empty oid means nothing has been asked yet.
    history_oid: String,
    history_in: bool,
    /// Whether something other than the current branch still reaches its
    /// tip — whether a rewrite here leaves the old commits drawn or leaves
    /// them to the reflog. False until the session says otherwise, which
    /// is the answer that asks more of the person doing it.
    head_reached_elsewhere: bool,
    /// Author identity; `identityReady` false means git cannot commit yet
    /// and the UI should ask for a name and address.
    author_name: String,
    author_email: String,
    identity_ready: bool,
    /// The same face the identity wears everywhere else: the identicon
    /// its name packs to, and the picture assigned to its address if
    /// there is one (デザイン規約 §アバターを与える). Held here rather
    /// than asked for on demand because the commit editor's badge is a
    /// binding, and a binding only re-reads a property.
    author_avatar: i32,
    author_avatar_url: String,
    /// Whether new commits here get signed, and with what kind of key.
    signs_commits: bool,
    signing_format: String,
    /// Last answer to `checkSignature`: the commit asked about, what to
    /// show for it ("" = unsigned), git's `%G?` letter and the signer.
    signature_oid: String,
    signature_kind: String,
    signature_code: String,
    signature_signer: String,
    /// Paths gathered for the next write over several files at once, one
    /// call at a time: a git path may hold any byte but NUL, so there is no
    /// separator safe enough to pack a list into one string
    /// (デザイン規約 §その他の操作). Emptied by whichever write consumes it, so a set
    /// left behind by an abandoned question cannot be spent later.
    pending_paths: Vec<String>,
    /// Configured remote names — where a branch with no upstream can go —
    /// and their fetch URLs in the same order, which is what the form that
    /// corrects one opens with.
    remotes: Vec<String>,
    remote_urls: Vec<String>,
    /// Derived from `remotes` on arrival rather than computed on demand:
    /// QML bindings only re-evaluate on a property change, so anything a
    /// binding reads has to be a property.
    remote_count: i32,
    default_remote: String,
    /// The remote this repository sends pushes to (`remote.pushDefault`),
    /// empty where none is marked, and whether the mark is this
    /// repository's own to clear. `default_remote` is what actually
    /// decides a destination and falls back to a remote called `origin`;
    /// this is the mark itself, which is what the sidebar draws and the
    /// dialog's box reads.
    push_default: String,
    push_default_local: bool,
    /// HEAD's message split into the editor's two fields, filled on
    /// request so an amend starts from it.
    head_subject: String,
    head_body: String,
    /// Who HEAD is attributed to, and whether that is someone other than
    /// the identity git would record now — the one case where taking over
    /// authorship on an amend is worth offering.
    head_author_name: String,
    head_author_email: String,
    head_author_differs: bool,
    /// Bumped each time an answer arrives, so an editor waiting for one
    /// can tell "not loaded yet" from "loaded, and it is empty".
    head_commit_seq: i32,
    /// The most recently finished write: its name, git's message (empty on
    /// success) and a counter QML compares against to spot a new one. A
    /// signal with arguments would be the natural shape, but the bridge
    /// only carries parameterless ones.
    last_write_op: String,
    last_write_error: String,
    write_seq: i32,
    /// A branch move that would leave commits unreachable, waiting to be
    /// asked about. Nothing has happened yet.
    move_ask_local: String,
    move_ask_start: String,
    move_ask_seq: i32,
    /// Auto fetch, reported apart from the shared busy/error surface so an
    /// offline machine does not raise a banner every interval.
    auto_fetch_running: bool,
    auto_fetch_error: String,
    /// Fetches that came back with something to say, in a row, whoever
    /// asked for them. Any fetch that comes back clean puts it to zero.
    fetch_failures: i32,
    /// The run has already had the command log raised over it. Held apart
    /// from the count, so a failure that may not speak — the fetch an
    /// opening fires — counts without using the run's one telling up.
    fetch_log_raised: bool,
    /// The timer was stopped because of those. Nothing but the button
    /// that says so starts it again — an automatic resume would happen
    /// behind the reader's back.
    auto_fetch_suspended: bool,
    feed: Option<Arc<Feed<TabMsg>>>,
}
