//! Per-tab lifecycle + error surface + refresh entry points.

use std::sync::Arc;

use platitude_core::ReportKind;
use qtbridge::{QObjectHolder, qobject};

use crate::hub::{Feed, Hub, TabMsg};
use crate::urlpath::picker_folder_url;

use super::qml_register;

mod drain;
#[cfg(test)]
mod drain_tests;
mod ops_config;
mod ops_conflict;
mod ops_delete;
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
    /// Whether the write in flight is one that replays history a commit
    /// at a time — the kind's own answer, carried across the bridge
    /// (`TabMsg::WriteState::replays`), so the page holds its write
    /// doors down without branching on git vocabulary
    /// (app-ui.md「QML にビジネスロジックを書かない」).
    replaying: bool,
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
    ///
    /// The answer is `"yes"` / `"no"` / `"unknown"`, the same three the
    /// rows give (`GraphModel::branch_delete_merged`), and empty until
    /// one lands. **Only `"no"` dresses the row**: a delete nobody has
    /// shown to be refused is offered plain, so unknown draws like merged
    /// and git answers the press.
    branch_delete_asked: String,
    branch_delete_merged: String,
    /// The plain `branch --delete` out for git's answer — the one press
    /// that leaves its menu standing, because git may refuse it and the
    /// refusal has to land on the row that asked (`AppMenuItem.staysOpen`).
    /// Only the plain form: `-D` and `Delete both` stand for nothing. Set
    /// by the slot, spent by the branch answer that follows.
    branch_delete_out: String,
    /// How git answered it: the branch whose plain delete git took, and
    /// the one it turned down. The card that stayed up reads its own name
    /// here and either goes or turns its row into `-D` (`RefBranchMenu`)
    /// — whichever menu raised it, so nothing above the card has to know
    /// which one is standing. **Kept until the next plain delete is
    /// asked**, like the check's answer above and unlike the write group
    /// below: a fetch answering in the same drain would rewrite a group
    /// property before the card had seen it, and the card would be left
    /// standing. `branch_delete_seq` is the `write_seq` the answer was
    /// counted at, for a reader that has to know the answer in hand is
    /// this one (`RepoPage`).
    branch_delete_landed: String,
    branch_delete_refused: String,
    branch_delete_seq: i32,
    /// The rows a delete is standing in for, one name per list — what the
    /// sidebar's sections and the graph's chips are drawn without
    /// (デザイン規約 §消す操作は先に画面から消す).
    ///
    /// **A copy, and the machine is elsewhere.** When they go and when
    /// they come back is decided by `ops::StandIn`, which the hub holds
    /// per tab so a write outlives the page drawing it; these four are
    /// the picture it leaves, kept here so a QML binding reads a plain
    /// member and never reaches into the hub for it (`ops_delete`).
    gone_branch: String,
    gone_remote: String,
    gone_tag: String,
    gone_stash: String,
    /// The commit the standing `checkSignature` is about — **the
    /// question, where `signature_oid` and its three are the answer**.
    ///
    /// It is asked on every selection and runs a git of its own, so two
    /// selections in quick succession leave two reads racing, and the one
    /// about the row already left behind can win. It answers a question
    /// nobody is asking any more, and there is one seat for every answer,
    /// so letting it land throws away the answer the pane was waiting
    /// for — which reads on screen as no answer at all, and
    /// nothing asks a third time (observed: a verified
    /// commit whose tick never came). Dropped here instead, which is what
    /// デザイン規約 §署名の表示 asks for: 答えは持ち回らない — 選択が動けば
    /// 捨て、同じ行へ戻ってくれば投げ直す.
    ///
    /// Not a property: QML compares the answer with what it has selected
    /// (`RepoPage.signatureIsForSelection`), and that is a different
    /// question — the selection can move without this tab being asked
    /// anything.
    signature_wanted: String,
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
    /// What the file menu's discard row would take, worked out as the menu
    /// opens (`planDiscard` over the gathered rows): how many rows it
    /// would touch — a conflicted row rides along untouched and uncounted
    /// — and, when that is exactly one, the bucket that row was on, which
    /// picks the tag the menu row wears (デザイン規約 §その他の操作).
    discard_count: i32,
    discard_only: String,
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
    /// The most recently finished write: git's message (empty on success)
    /// and a counter QML compares against to spot a new one. A signal
    /// with arguments would be the natural shape, but the bridge only
    /// carries parameterless ones.
    ///
    /// The message stays raw data over there — the push flow shows it.
    /// What an answer *means* is the classified group below: the page
    /// never branches on git vocabulary (app-ui.md).
    last_write_error: String,
    /// git stopped part-way through the write in flight and left the
    /// operation standing. Raised by the message before that write's
    /// answer and read by the answer itself, which is where it becomes
    /// the answer's own (`WriteAnswer::stopped`): not an error and not a
    /// landing, and where the screen goes next differs for all three.
    last_write_stopped: bool,
    write_seq: i32,
    /// That answer, classified where the op names are known
    /// (`drain::settle_write`).
    ///
    /// **What is left over.** Every one of these describes the last
    /// answer of this notify that **nobody was waiting for by name**, and
    /// each such answer rewrites the whole group, so nothing stays armed
    /// for a later write to trip over. An answer that went to an owner is
    /// left out of it entirely (`ops::Press`): the page acts on it
    /// where the owner hands it over, and a copy here would have the page
    /// act on the same answer a second time. The group is emptied at the
    /// top of every drain like the answers beside it, so a notify that
    /// carried nothing for it says so rather than leaving the last
    /// drain's classification up to be read again.
    ///
    /// git would not do it, or could not reach the far side to;
    /// `last_write_error` holds its words.
    write_refused: bool,
    /// The shown diff is a picture of a file that is gone: a landed
    /// stage / unstage / discard / commit / stash moved what the two
    /// sides hold, and a refused stage / unstage / discard was refused
    /// *because* the rows on screen drifted. Either way the answer is
    /// the fresh file.
    write_stale_diff: bool,
    /// HEAD moved (a checkout or a reset landed), so the working tree
    /// under an open diff was rewritten.
    write_moved_head: bool,
    /// A reword landed: the saved message is on its commit.
    write_reworded: bool,
    /// A stash operation landed. Which one is not said — push, pop,
    /// apply, drop and rename all answer as one — so a reader waiting on
    /// a particular one finds its own answer by the id its press was
    /// given (`WriteAnswer::id`; `RepoPage.absorbPopLabel`).
    write_stashed: bool,
    /// The answer was about a branch (create / delete / rename),
    /// whichever way it went: what the page armed for one — a refusal to
    /// wear `-D`, a rename to carry to the remote — reads this beside
    /// its own state.
    write_branch_op: bool,
    /// The answer was a push, landed or not. A remote branch's rename
    /// and delete answer under the same name, so whether it was the
    /// toolbar button's push stays the flow's own slot to say
    /// (`PublishFlow.pushSentBranch`).
    write_pushed: bool,
    /// The answer was a fetch, landed or not. A fetch that could not
    /// reach the far side has already said so in the tab's own terms —
    /// the button's count, the header's line, the panel raised once for
    /// the run — so the page reads this to tell that news from another
    /// report landing in the same panel (`CommandsOwner`).
    write_fetched: bool,
    /// The write answers *this* notify carried, oldest first — the group
    /// above says only what the last of them was.
    ///
    /// One drain empties the whole queue and notifies once
    /// (`drain::absorb`), so a run's own answer and the fetch that came
    /// back behind it reach QML as one `changed`. Everything read off the
    /// group is then the fetch's: a reader waiting for its own write by
    /// name never sees the name, and a stop the run left standing is
    /// taken back down by the fetch *starting* — both measured as
    /// silence, not as a wrong answer, and both correlated with how
    /// loaded the machine is. So the answers are kept as they came, and
    /// what waits for one of them looks here.
    ///
    /// Emptied at the top of every drain: a notify raised from anywhere
    /// else (a slot that writes a setting) carries no answers, and the
    /// list left standing would be read a second time. `seq` is what
    /// tells an answer already counted from this one's own; `id` is what
    /// a reader that was handed one at the press looks for.
    write_answers: Vec<WriteAnswer>,
    /// The commit the editor sent, waiting for the answer that empties
    /// its boxes — **the id it was accepted under, written down by the
    /// press itself** (`state::commit_from_fields`), so no reader has to
    /// hold one across the bridge and none has to guess from the order
    /// answers arrive in.
    ///
    /// **A copy of what it says is beside it.** The owner is plain Rust
    /// and knows neither Qt nor git (`ops::Press`, whose transitions
    /// run under `cargo test`); `commit_answer` is the picture QML is
    /// handed, the way the four `gone_*` are the delete's.
    commit_out: crate::ops::Press,
    /// Where the editor's own answer stands in `write_answers`, or -1
    /// where this notify carried none of it — which is most notifies.
    /// What git said about it is read off that answer, like every other
    /// reader that waits for one write by name.
    commit_answer: i32,
    /// That write did not happen, and something outside this application
    /// said so — a protected branch, a repository rule, a hook over there
    /// or here, a remote this end had only an older picture of. Nothing
    /// here can put it right and nothing was half done, so the page
    /// reports it rather than raising the log over it
    /// (デザイン規約 §答えの要らない報せ).
    ///
    /// `kind` is which report this is (`delete` / `update` / `outdated` /
    /// `commit`), which is what the sentence turns on; `remote` and
    /// `name` are what it is written about, and the words are whoever
    /// said no, shown as they came. Empty `kind` is "no report in this
    /// answer" — the whole group is rewritten by every answer.
    write_report_kind: String,
    write_report_remote: String,
    write_report_name: String,
    write_report_reason: String,
    /// A write the user asked for is in flight. **The log does not raise
    /// itself for a command that fails inside one**: an operation is
    /// several commands and only its own answer says whether it failed,
    /// or whether the far side turned it down with something to report
    /// instead (デザイン規約 §git が言ったことを読む場所 — 開く判断は操作の
    /// 答えで下し、コマンド 1 本の終了コードでは下さない).
    write_running: bool,
    /// A branch move that would leave commits unreachable, waiting to be
    /// asked about. Nothing has happened yet.
    move_ask_local: String,
    move_ask_start: String,
    move_ask_seq: i32,
    /// Auto fetch, reported apart from the shared busy/error surface so an
    /// offline machine does not raise a banner every interval.
    auto_fetch_running: bool,
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

/// One write's answer as it arrived, for the readers that wait on a
/// particular one (`RepoTab::write_answers`).
///
/// Carries what the group properties would say about it and no more:
/// which write answered, whether git stopped part-way, whether it was
/// refused, and whether it left a commit at the tip — **the meanings are
/// named on this side of the bridge**, the same way `settle_write` names
/// them for the group (app-ui.md: no business logic in QML). `id` is the
/// one the queue handed back at the press (`platitude_core::OperationId`),
/// which is how a reader that holds one finds its answer whatever came
/// in between; `seq` is the number `write_seq` counted it at, so a run
/// that holds no id tells its own answer from one already counted before
/// it pressed; `head_seq` is the number the session named for the first
/// report of HEAD after the write (`TabMsg::WriteState::head_seq`), which
/// a landing on its tip arms on.
struct WriteAnswer {
    id: u64,
    seq: i32,
    op: String,
    stopped: bool,
    failed: bool,
    at_tip: bool,
    head_seq: i32,
    /// What the far side, a hook, or this end itself said about refusing
    /// it — **carried on the answer rather than beside the group**,
    /// because a report is the answer's and a drain can bring several.
    /// Empty `kind` is "this one came with nothing to report", which is
    /// git's plain refusal and the command log's news
    /// (デザイン規約 §答えの要らない報せ).
    report_kind: String,
    report_remote: String,
    report_name: String,
    report_reason: String,
}

/// One of a session's running numbers as the bridge carries it (`i32`)
/// — the id a write was accepted under, or the report of HEAD an answer
/// names. A process counts both from one and neither reaches the top of
/// the range, so the saturation is a formality rather than a case.
fn bridge_id(number: u64) -> i32 {
    i32::try_from(number).unwrap_or(i32::MAX)
}
