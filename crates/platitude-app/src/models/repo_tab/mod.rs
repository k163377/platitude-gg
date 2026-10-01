//! Per-tab lifecycle + error surface + refresh entry points.

use std::sync::Arc;

use platitude_core::{OperationKind, ReportKind};
use qtbridge::{QmlObject, qobject};

use crate::encode::{Fields, Optional, Record, field};
use crate::hub::{Feed, Hub, TabMsg};
use crate::urlpath::picker_folder_url;

use super::qml_register;

/// The name a remote was asked about (`checkRemoteBranch`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteBranch {
    pub remote: String,
    pub branch: String,
}

impl Record for RemoteBranch {
    fn to_map(&self) -> qtbridge::qtbridge_type_lib::QVariantMap {
        Fields::new()
            .put("remote", &self.remote)
            .put("branch", &self.branch)
            .done()
    }

    fn from_map(map: &qtbridge::qtbridge_type_lib::QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            remote: field(map, "remote")?,
            branch: field(map, "branch")?,
        })
    }
}

mod drain;
#[cfg(test)]
mod drain_remotes_tests;
#[cfg(test)]
mod drain_report_tests;
#[cfg(test)]
mod drain_signature_tests;
#[cfg(test)]
mod drain_tests;
mod ops_config;
mod ops_conflict;
mod ops_delete;
mod ops_remote;
mod ops_stage;
mod qobject;
mod restand;
mod state;
mod write_watch;

use write_watch::WriteWatch;

/// Failed fetches in a row before the timer is stopped — more than one,
/// so a brief offline blip does not stop it.
const FETCH_FAILURES_BEFORE_STOP: i32 = 3;

pub struct RepoTab {
    tab_id: i32,
    state: String,
    title: String,
    repo_path: String,
    /// Where the picker opens, as a URL: the folder holding the
    /// repository's own working copy (`Hub::home_copy`), not a linked
    /// copy's — repositories sit side by side.
    picker_folder_url: String,
    /// Why the repository would not open, in git's words. Read only on
    /// the `other` kind — the heading names the other two.
    error: String,
    /// Which of the three the failure was (`plain` / `bare` / `other`),
    /// the same word the picker's dialog branches on.
    error_kind: String,
    /// The folder that would not open, for the line under the heading.
    error_path: String,
    last_error: String,
    /// Whether `last_error` is fetch news — only that line is taken down
    /// by a later fetch that lands.
    last_error_from_fetch: bool,
    tags_shown: bool,
    /// Write commands currently in flight (they are serialized per session,
    /// but requests can queue up).
    busy_count: i32,
    busy_op: String,
    /// The tab moved to another working copy and the new session has not
    /// said where it is yet (`Hub::restand_tab`).
    ///
    /// Counted in `busy_count` while it lasts, so every door waits: a
    /// write asked in the gap reaches a session with no repository open
    /// and is refused (`RepoSession::run_write`). Held apart from the
    /// count so putting it down is not a write's answer.
    standing: bool,
    /// Whether the write in flight replays history a commit at a time
    /// (`OperationKind::replays_history`); the page holds its write doors
    /// down off this alone.
    replaying: bool,
    /// Merge tool names the settings field can offer; empty is a working
    /// state (the field takes a typed name).
    merge_tools: Vec<String>,
    /// A candidate read is out. Asking git what is installed takes
    /// seconds, so the field says so while it runs.
    merge_tools_loading: bool,
    /// Configured remote names as one property, for the publish question
    /// to bind to (`remoteAt` is a slot).
    remote_names: Vec<String>,
    /// The name the last answer to `checkRemoteBranch` was about; none
    /// while a read is out. Readers check it — the box may have moved on.
    remote_branch_asked: Optional<RemoteBranch>,
    /// Moves with every answer and every question, so a binding on it
    /// re-reads `remoteBranchAsked()`.
    remote_branch_revision: i32,
    /// What a push under that name would meet over there, as the wire
    /// name of `platitude_core::remote::RemoteBranchState` — one string,
    /// since the five are exclusive.
    remote_branch_state: String,
    /// The commit that remote advertised for the name, hex — what an
    /// overwrite leases against, being the one the question showed.
    remote_branch_tip: String,
    /// How many commits that tip has that this branch does not: what an
    /// overwrite would take off it.
    remote_branch_theirs: i32,
    /// Last answer to `checkBranchDelete`: the branch asked about, and
    /// whether it is merged into what `branch --delete` measures against
    /// (its upstream, else HEAD). The menu checks the echo — it may be
    /// open over another row by now.
    ///
    /// `"yes"` / `"no"` / `"unknown"` as the rows give it
    /// (`GraphModel::branch_delete_merged`), empty until one lands. Only
    /// `"no"` dresses the row: unknown draws like merged and git answers
    /// the press.
    branch_delete_asked: String,
    branch_delete_merged: String,
    /// The plain `branch --delete` out for git's answer — the one press
    /// whose menu stays up, so a refusal lands on the row that asked
    /// (`AppMenuItem.staysOpen`); `-D` and `Delete both` stand for
    /// nothing. The press writes the name down with its write's id
    /// (`ops::BranchDeleteOut`): a drain can carry several answers, and
    /// only the id tells which is this card's.
    branch_delete_out: crate::ops::BranchDeleteOut,
    /// How git answered it: the branch whose plain delete it took, and the
    /// one it turned down. The card reads its own name here and goes or
    /// turns its row into `-D` (`RefBranchMenu`). Kept until the next
    /// plain delete is asked, unlike the write group: the card reads it
    /// as an edge and may not have been drawn yet.
    branch_delete_landed: String,
    branch_delete_refused: String,
    /// Where that card's own answer stands in `write_answers`, or -1
    /// where this notify carried none — how the page tells the answer in
    /// hand from the one standing above (`RepoPage.absorbBranchDelete`).
    branch_delete_answer: i32,
    /// The rows a delete is standing in for, one name per list, which the
    /// sidebar and the graph draw without (デザイン規約 §消す操作は先に画面から消す).
    ///
    /// A copy: `ops::StandIn` decides, held by the hub per tab so a write
    /// outlives the page; kept here so a binding reads a plain member
    /// (`ops_delete`).
    gone_branch: String,
    gone_remote: String,
    gone_tag: String,
    gone_stash: String,
    gone_worktree: String,
    /// The commit the standing `checkSignature` is about — the question,
    /// where `signature_oid` and its three are the answer.
    ///
    /// Two quick selections leave two reads racing; the stale one landing
    /// would take the one seat and the pane's own answer would never show.
    /// Dropped here (デザイン規約 §署名の表示「答えはその選択限り」).
    ///
    /// A plain member: QML matches the answer against its own selection
    /// (`RepoPage.signatureIsForSelection`).
    signature_wanted: String,
    /// Author identity; `identityReady` false means git cannot commit yet
    /// and the UI should ask for a name and address.
    author_name: String,
    author_email: String,
    identity_ready: bool,
    /// The identity's face: its identicon, and the picture assigned to its
    /// address if any (デザイン規約 §アバターを与える). Properties because
    /// the commit editor's badge is a binding.
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
    /// Paths gathered one call at a time for the next multi-file write: a
    /// git path may hold any byte but NUL, so no separator can pack a list
    /// into one string. Emptied by whichever write consumes it, so a set
    /// an abandoned question left cannot be spent later.
    pending_paths: Vec<String>,
    /// What the file menu's discard row would take, worked out as the menu
    /// opens (`planDiscard`): how many rows (conflicted ones uncounted),
    /// and the bucket when exactly one, which picks the row's tag
    /// (デザイン規約 §その他の操作).
    discard_count: i32,
    discard_only: String,
    /// Configured remote names, and their fetch URLs in the same order
    /// (what the form that corrects one opens with).
    remotes: Vec<String>,
    remote_urls: Vec<String>,
    /// Derived from `remotes` on arrival, for bindings.
    remote_count: i32,
    default_remote: String,
    /// `remote.pushDefault` (empty where unset), and whether it is this
    /// repository's own to clear. The key itself, for the sidebar's badge
    /// and the dialog's box; `default_remote` is the resolved destination
    /// (falls back to `origin`).
    push_default: String,
    push_default_local: bool,
    /// The remote both keys `Mark as origin` writes name
    /// (`remote.pushDefault` and `checkout.defaultRemote`), empty where
    /// they part or neither is set — a remote only one names still has
    /// the mark to finish.
    marked_origin: String,
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
    /// The most recently finished write: git's message (empty on success)
    /// and a counter QML compares to spot a new one — the bridge carries
    /// only parameterless signals. Shown raw; what an answer means is the
    /// group below.
    last_write_error: String,
    /// git stopped part-way through the write in flight. Raised by the
    /// message before that write's answer, which takes it as its own
    /// (`WriteAnswer::stopped`).
    last_write_stopped: bool,
    write_seq: i32,
    /// The one write a run is waiting for, by the id its ask was given,
    /// compared by equality alone (`write_watch`).
    write_watch: WriteWatch,
    /// The last answer of this notify that no press named, classified
    /// (`drain::fold_into_group`); each such answer rewrites the whole
    /// group. An answer that went to an owner is left out (`ops::Press`) —
    /// a copy here would have the page act on it twice. Emptied at the top
    /// of every drain.
    ///
    /// git would not do it, or could not reach the far side to;
    /// `last_write_error` holds its words.
    write_refused: bool,
    /// The shown diff is stale: a landed stage / unstage / discard /
    /// commit / stash moved it, and a refused stage / unstage / discard
    /// was refused because the rows on screen drifted.
    write_stale_diff: bool,
    /// HEAD moved (a checkout or a reset landed), so the working tree
    /// under an open diff was rewritten.
    write_moved_head: bool,
    /// A reword landed: the saved message is on its commit.
    write_reworded: bool,
    /// The answer was about a branch (create / delete / rename), whichever
    /// way it went; what the page armed for one reads this beside its own
    /// state.
    write_branch_op: bool,
    /// The same as `write_branch_op`, for a tag.
    write_tag_op: bool,
    /// The answer was a fetch, landed or not. A failed fetch has said so
    /// already (count, header line, panel), so the page tells that news
    /// from another report in the same panel (`CommandsOwner`).
    write_fetched: bool,
    /// The write answers this notify carried, oldest first; the group
    /// above says only what the last was.
    ///
    /// One drain empties the whole queue and notifies once
    /// (`drain::absorb`), so a run's answer and the fetch behind it reach
    /// QML as one `changed`: read off the group, the run's answer and its
    /// stop are lost to the fetch. What waits for one answer looks here.
    ///
    /// Emptied at the top of every drain, or a notify raised elsewhere
    /// would have the list read a second time.
    write_answers: Vec<WriteAnswer>,
    /// The commit the editor sent, waiting for the answer that empties
    /// its boxes — by the id the press wrote down
    /// (`state::commit_from_fields`), whatever order the answers arrive
    /// in (`ops::Press`).
    commit_out: crate::ops::Press,
    /// Where the editor's own answer stands in `write_answers`, or -1
    /// where this notify carried none.
    commit_answer: i32,
    /// The open diff read again where a write answered, waiting for the
    /// status that write publishes behind it (`ops::DiffReread`). A slot,
    /// not a property: it has an answer only while a status is being
    /// read, and asking spends it (`takeDiffRead`).
    diff_reread: crate::ops::DiffReread,
    /// The working tree as the page last put it on screen: the report of
    /// HEAD its counts stood beside, and whether they left it empty
    /// (`noteTreeRead`).
    ///
    /// Kept once: `diff_reread` and `stash_out` are both answered against
    /// it, and two copies could disagree. Written whether or not anything
    /// waits — status and answer are drained apart, and this often
    /// arrives first.
    tree_seen: u64,
    tree_emptied: bool,
    /// The stash this window pressed to take the working tree away,
    /// waiting for the reading that finds the tree empty
    /// (`ops::StashOut`).
    ///
    /// Whether that tree is the one on screen is asked once, when the page
    /// acts on a tree, and asking spends it (`takeStashLanding`) — a
    /// binding would read an answer already gone.
    stash_out: crate::ops::StashOut,
    /// Where that press's own answer stands in `write_answers`, or -1
    /// where this notify carried none.
    stash_answer: i32,
    /// The toolbar's push — plain, leased, or the question's first push —
    /// waiting for its own answer, with the branch it was sent for
    /// (`ops::PushOut`). A remote branch's replace and delete answer as
    /// `push` too, beside a fetch in the same drain; only the id says
    /// whose it was.
    push_out: crate::ops::PushOut,
    /// Where that press's own answer stands in `write_answers`, or -1
    /// where this notify carried none, and the branch a refusal is
    /// remembered against (デザイン規約 §リモートへ送る).
    push_answer: i32,
    push_answer_branch: String,
    /// The pushes a ref row sends — the two `push --delete`s, the replace
    /// git has no command for, and the two pairs that reach over there
    /// after doing something here. Owned for the toolbar's reason: in the
    /// group, a fetch in the same drain would take the refusal over
    /// (`drain::fold_into_group`).
    ref_push_out: crate::ops::PushOut,
    /// Where that press's own answer stands in `write_answers`, or -1
    /// where this notify carried none of it, and the row it was about.
    ref_push_answer: i32,
    ref_push_target: String,
    /// That write did not happen, and something outside this application
    /// said so — a protected branch, a rule, a hook, a stale picture of
    /// the remote (デザイン規約 §答えの要らない報せ).
    ///
    /// `kind` (`delete` / `update` / `outdated` / `commit`) is what the
    /// sentence turns on; `remote` and `name` are what it is about, and
    /// the reason is shown as it came. Empty `kind` is "no report in this
    /// answer".
    write_report_kind: String,
    write_report_remote: String,
    write_report_name: String,
    write_report_reason: String,
    /// A branch move that would leave commits unreachable, waiting to be
    /// asked about. Nothing has happened yet.
    move_ask_local: String,
    move_ask_start: String,
    move_ask_seq: i32,
    /// Auto fetch, reported apart from the shared busy/error
    /// surface, which an offline machine would set off every interval.
    auto_fetch_running: bool,
    /// Fetches that came back with something to say, in a row, whoever
    /// asked for them. Any fetch that comes back clean puts it to zero.
    fetch_failures: i32,
    /// The run has already had the command log raised over it. Held apart
    /// from the count, so a failure that may not speak — the fetch an
    /// opening fires — counts without using the run's one telling up.
    fetch_log_raised: bool,
    /// The timer was stopped because of those. Only the button that
    /// says so starts it again — an automatic resume would happen
    /// behind the reader's back.
    auto_fetch_suspended: bool,
    feed: Option<Arc<Feed<TabMsg>>>,
}

/// Whether anybody asked for a write of this kind, or it is one of the
/// page's own fetches (the interval's, an opening's). Named in core
/// (`OperationKind::asked_for`) so this side and the queue agree.
pub(super) fn asked_for(kind: OperationKind) -> bool {
    kind.asked_for()
}

/// One write's answer as it arrived, for the readers that wait on a
/// particular one (`RepoTab::write_answers`), its meanings named on this
/// side of the bridge as `settle_write` names them for the group. `id` is
/// the one the queue handed back at the press; `seq` is where `write_seq`
/// counted it, so a run holding no id tells its answer from one counted
/// before it pressed; `head_seq` is the first report of HEAD after the
/// write (`TabMsg::WriteState::head_seq`), which a tip landing arms on.
struct WriteAnswer {
    id: u64,
    seq: i32,
    /// Which write — the type itself; the word QML reads is made at the
    /// slot (`writeAnswerOp`).
    kind: OperationKind,
    stopped: bool,
    failed: bool,
    /// git's own words where it was refused, empty where it landed — per
    /// answer, unlike `last_write_error`.
    error: String,
    at_tip: bool,
    head_seq: u64,
    /// What the far side, a hook, or this end said about refusing it — per
    /// answer, since a drain can bring several. Empty `kind` is git's
    /// plain refusal, the command log's news
    /// (デザイン規約 §答えの要らない報せ).
    report_kind: String,
    report_remote: String,
    report_name: String,
    report_reason: String,
}

/// A session's running number (a write's id, a HEAD report's) as the
/// bridge carries it. Both count from one and never reach `i32::MAX`, so
/// the saturation is a formality.
fn bridge_id(number: u64) -> i32 {
    i32::try_from(number).unwrap_or(i32::MAX)
}
