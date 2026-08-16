use std::sync::Arc;

use qtbridge::{QObjectHolder, qobject};

use crate::hub::{Feed, Hub, TabMsg};
use crate::urlpath::picker_folder_url;

use super::qml_register;

// ---------------------------------------------------------------------------
// RepoTab: per-tab lifecycle + error surface + refresh entry points
// ---------------------------------------------------------------------------

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
    /// Lines of the shown diff picked by hand, as `(hunk, line)` indices
    /// into the diff they were read from. Gathered the same way and for
    /// the same reason as `pending_paths`: the write takes the whole set
    /// at once.
    pending_lines: Vec<(i32, i32)>,
    /// Configured remote names — where a branch with no upstream can go.
    remotes: Vec<String>,
    /// Derived from `remotes` on arrival rather than computed on demand:
    /// QML bindings only re-evaluate on a property change, so anything a
    /// binding reads has to be a property.
    remote_count: i32,
    default_remote: String,
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
    /// The timer was stopped because of those. Nothing but the button
    /// that says so starts it again — an automatic resume would happen
    /// behind the reader's back.
    auto_fetch_suspended: bool,
    feed: Option<Arc<Feed<TabMsg>>>,
}

impl Default for RepoTab {
    fn default() -> Self {
        Self {
            tab_id: 0,
            state: String::new(),
            title: String::new(),
            repo_path: String::new(),
            picker_folder_url: String::new(),
            error: String::new(),
            error_kind: String::new(),
            error_path: String::new(),
            last_error: String::new(),
            last_error_from_fetch: false,
            // Mirrors core LogOptions::default().
            tags_shown: true,
            busy_count: 0,
            busy_op: String::new(),
            merge_tools: String::new(),
            merge_tools_loading: false,
            remote_names: String::new(),
            remote_branch_asked: String::new(),
            remote_branch_state: String::new(),
            remote_branch_tip: String::new(),
            remote_branch_theirs: 0,
            branch_delete_asked: String::new(),
            branch_delete_merged: true,
            publish_range: String::new(),
            publish_total: 0,
            publish_published: 0,
            history_oid: String::new(),
            history_in: false,
            head_reached_elsewhere: false,
            author_name: String::new(),
            author_email: String::new(),
            // Assumed fine until the check says otherwise, so nothing
            // flashes a warning during startup.
            identity_ready: true,
            signs_commits: false,
            signing_format: String::new(),
            signature_oid: String::new(),
            signature_kind: String::new(),
            signature_code: String::new(),
            signature_signer: String::new(),
            pending_paths: Vec::new(),
            pending_lines: Vec::new(),
            remotes: Vec::new(),
            remote_count: 0,
            default_remote: String::new(),
            head_subject: String::new(),
            head_body: String::new(),
            head_author_name: String::new(),
            head_author_email: String::new(),
            head_author_differs: false,
            head_commit_seq: 0,
            last_write_op: String::new(),
            last_write_error: String::new(),
            write_seq: 0,
            move_ask_local: String::new(),
            move_ask_start: String::new(),
            move_ask_seq: 0,
            auto_fetch_running: false,
            auto_fetch_error: String::new(),
            fetch_failures: 0,
            auto_fetch_suspended: false,
            feed: None,
        }
    }
}

impl RepoTab {
    /// Runs `f` with this tab's session, if the tab is still open.
    fn with_session(&self, f: impl FnOnce(&Arc<platitude_core::session::RepoSession>)) {
        if let Some(Some(session)) = Hub::with(|hub| hub.session(self.tab_id)) {
            f(&session);
        }
    }

    /// A fetch ended, whoever asked for it. Counts the ones that failed
    /// and stops the timer once there have been enough of them, so a
    /// machine that has lost the network stops reaching for it every
    /// interval. Anything that comes back clean clears the run.
    fn fetch_settled(&mut self, error: &str) {
        if error.is_empty() {
            self.fetch_failures = 0;
            self.auto_fetch_error = String::new();
            // The header line is state, not history: a fetch that has
            // just landed makes "fetch cannot reach the remote" untrue,
            // and red kept up over that would contradict the button that
            // is already back to normal (デザイン規約 §リモートから取り込む
            // 「成功が 1 回入れば数は 0 に戻る」— its command-log side).
            // Rows are left alone: history stays until a reader clears it.
            if self.last_error_from_fetch {
                self.last_error = String::new();
                self.last_error_from_fetch = false;
            }
            return;
        }
        self.fetch_failures += 1;
        if self.fetch_failures == 1 {
            // The panel reads this; the ones after it are the same news.
            self.last_error = error.to_string();
            self.last_error_from_fetch = true;
            self.fetch_first_failed();
        }
        if self.fetch_failures < FETCH_FAILURES_BEFORE_STOP || self.auto_fetch_suspended {
            return;
        }
        // Whether there was a timer to stop is the answer to "is this a
        // repository that fetches on its own at all": one that does not
        // has nothing suspended, and nothing to be told about it.
        let mut stopped = false;
        if let Some(Some(session)) = Hub::with(|hub| hub.session(self.tab_id)) {
            stopped = session.suspend_auto_fetch();
        }
        self.auto_fetch_suspended = stopped;
    }

    /// Whether HEAD carries someone else's name — the only case where an
    /// amend has authorship to take over (`--reset-author`).
    ///
    /// git refuses to commit with an empty `user.name`, so a HEAD that is
    /// really there always has one: an empty name is "no HEAD read yet"
    /// rather than an identity to compare against.
    fn compare_head_author(&mut self) {
        self.head_author_differs = !self.head_author_name.is_empty()
            && (self.head_author_name != self.author_name
                || self.head_author_email != self.author_email);
    }

    /// Decodes the push-force pair QML sends.
    fn push_force(force: &str, lease_expect: &str) -> platitude_core::remote::PushForce {
        use platitude_core::remote::PushForce;
        match force {
            "lease" => PushForce::WithLease {
                expect: (!lease_expect.is_empty()).then(|| lease_expect.to_string()),
            },
            "force" => PushForce::Force,
            _ => PushForce::None,
        }
    }

    /// Moves HEAD, taking uncommitted work along — through a stash when
    /// git will not carry it itself, which needs nothing asked here.
    fn move_head(&self, target: platitude_core::branch::CheckoutTarget) {
        self.with_session(|s| s.checkout(target.clone()));
    }
}

#[qobject(ConvertToCamelCase, NoQmlElement)]
impl RepoTab {
    qproperty!("state", Member = state, Notify = changed);
    qproperty!("title", Member = title, Notify = changed);
    qproperty!("repoPath", Member = repo_path, Notify = changed);
    qproperty!(
        "pickerFolderUrl",
        Member = picker_folder_url,
        Notify = changed
    );
    qproperty!("error", Member = error, Notify = changed);
    qproperty!("errorKind", Member = error_kind, Notify = changed);
    qproperty!("errorPath", Member = error_path, Notify = changed);
    qproperty!("lastError", Member = last_error, Notify = changed);
    qproperty!("tagsShown", Member = tags_shown, Notify = changed);
    qproperty!("busyCount", Member = busy_count, Notify = changed);
    qproperty!("busyOp", Member = busy_op, Notify = changed);
    qproperty!("mergeTools", Member = merge_tools, Notify = changed);
    qproperty!(
        "mergeToolsLoading",
        Member = merge_tools_loading,
        Notify = changed
    );
    qproperty!("remoteNames", Member = remote_names, Notify = changed);
    qproperty!(
        "remoteBranchAsked",
        Member = remote_branch_asked,
        Notify = changed
    );
    qproperty!(
        "remoteBranchState",
        Member = remote_branch_state,
        Notify = changed
    );
    qproperty!(
        "remoteBranchTip",
        Member = remote_branch_tip,
        Notify = changed
    );
    qproperty!(
        "remoteBranchTheirs",
        Member = remote_branch_theirs,
        Notify = changed
    );
    qproperty!(
        "branchDeleteAsked",
        Member = branch_delete_asked,
        Notify = changed
    );
    qproperty!(
        "branchDeleteMerged",
        Member = branch_delete_merged,
        Notify = changed
    );
    qproperty!("publishRange", Member = publish_range, Notify = changed);
    qproperty!("publishTotal", Member = publish_total, Notify = changed);
    qproperty!(
        "publishPublished",
        Member = publish_published,
        Notify = changed
    );
    qproperty!("historyOid", Member = history_oid, Notify = changed);
    qproperty!("historyIn", Member = history_in, Notify = changed);
    qproperty!(
        "headReachedElsewhere",
        Member = head_reached_elsewhere,
        Notify = changed
    );
    qproperty!("authorName", Member = author_name, Notify = changed);
    qproperty!("authorEmail", Member = author_email, Notify = changed);
    qproperty!("identityReady", Member = identity_ready, Notify = changed);
    qproperty!("signsCommits", Member = signs_commits, Notify = changed);
    qproperty!("signingFormat", Member = signing_format, Notify = changed);
    qproperty!("signatureOid", Member = signature_oid, Notify = changed);
    qproperty!("signatureKind", Member = signature_kind, Notify = changed);
    qproperty!("signatureCode", Member = signature_code, Notify = changed);
    qproperty!(
        "signatureSigner",
        Member = signature_signer,
        Notify = changed
    );
    qproperty!("remoteCount", Member = remote_count, Notify = changed);
    qproperty!("defaultRemote", Member = default_remote, Notify = changed);
    qproperty!("headSubject", Member = head_subject, Notify = changed);
    qproperty!("headBody", Member = head_body, Notify = changed);
    qproperty!(
        "headAuthorName",
        Member = head_author_name,
        Notify = changed
    );
    qproperty!(
        "headAuthorEmail",
        Member = head_author_email,
        Notify = changed
    );
    qproperty!(
        "headAuthorDiffers",
        Member = head_author_differs,
        Notify = changed
    );
    qproperty!("headCommitSeq", Member = head_commit_seq, Notify = changed);
    qproperty!("lastWriteOp", Member = last_write_op, Notify = changed);
    qproperty!(
        "lastWriteError",
        Member = last_write_error,
        Notify = changed
    );
    qproperty!("writeSeq", Member = write_seq, Notify = changed);
    qproperty!("moveAskLocal", Member = move_ask_local, Notify = changed);
    qproperty!("moveAskStart", Member = move_ask_start, Notify = changed);
    qproperty!("moveAskSeq", Member = move_ask_seq, Notify = changed);
    qproperty!(
        "autoFetchRunning",
        Member = auto_fetch_running,
        Notify = changed
    );
    qproperty!(
        "autoFetchError",
        Member = auto_fetch_error,
        Notify = changed
    );
    qproperty!("fetchFailures", Member = fetch_failures, Notify = changed);
    qproperty!(
        "autoFetchSuspended",
        Member = auto_fetch_suspended,
        Notify = changed
    );

    #[qsignal]
    fn changed(&mut self);

    /// The first fetch of a run to fail. Only the first: a machine that
    /// is simply offline fails every interval, and the panel that opens
    /// on this would then be opening over and over on the same news.
    #[qsignal]
    fn fetch_first_failed(&mut self);

    /// Starts the timer again on the interval it was set to, and fetches
    /// now — the hold on the toolbar button is what reaches this.
    #[qslot]
    fn resume_auto_fetch(&mut self) {
        self.auto_fetch_suspended = false;
        self.fetch_failures = 0;
        self.auto_fetch_error = String::new();
        // Only the line fetch wrote: a background read's news is not
        // this button's to take down.
        if self.last_error_from_fetch {
            self.last_error = String::new();
            self.last_error_from_fetch = false;
        }
        self.with_session(|s| s.resume_auto_fetch());
        self.with_session(|s| s.fetch(None));
        self.changed();
    }

    /// Name of one remote (a list property would need a model of its own
    /// for three strings).
    #[qslot]
    fn remote_at(&self, index: i32) -> String {
        usize::try_from(index)
            .ok()
            .and_then(|i| self.remotes.get(i))
            .cloned()
            .unwrap_or_default()
    }

    /// Local branch name a remote-tracking ref would take: the ref with
    /// its remote's prefix removed.
    ///
    /// Matched against the configured remotes rather than cut at the first
    /// slash — a remote may be named `my/fork`, and the longest matching
    /// prefix is the right one.
    #[qslot]
    fn local_name_for(&self, remote_ref: String) -> String {
        let mut best: Option<&str> = None;
        for remote in &self.remotes {
            let Some(rest) = remote_ref.strip_prefix(&format!("{remote}/")) else {
                continue;
            };
            if !rest.is_empty() && best.is_none_or(|found| rest.len() < found.len()) {
                best = Some(rest);
            }
        }
        best.unwrap_or(remote_ref.as_str()).to_string()
    }

    #[qslot]
    fn attach(&mut self, tab_id: i32) {
        self.tab_id = tab_id;
        self.state = "loading".into();
        self.changed();
        if let Some(Some(feeds)) = Hub::with(|hub| hub.feeds(tab_id)) {
            let feed = Arc::clone(&feeds.tab);
            feed.attach(self.get_qml_method_invoker());
            self.feed = Some(feed);
        }
    }

    /// Called when this page becomes the visible one. A tab restored from
    /// the last session has no repository open behind it until then — the
    /// feeds are already attached, so the page simply stops saying
    /// "loading" once this fills them. Doing nothing on a tab that is
    /// already open is the normal case.
    #[qslot]
    fn activate(&mut self) {
        let id = self.tab_id;
        Hub::with(|hub| hub.ensure_open(id));
    }

    #[qslot]
    fn drain(&mut self) {
        let Some(feed) = self.feed.clone() else {
            return;
        };
        for msg in feed.drain() {
            match msg {
                TabMsg::Opened { title, path } => {
                    self.state = "open".into();
                    self.title = title;
                    self.picker_folder_url = picker_folder_url(std::path::Path::new(&path));
                    self.repo_path = path;
                }
                TabMsg::OpenFailed {
                    kind,
                    path,
                    message,
                } => {
                    self.state = "error".into();
                    self.error_kind = kind.into();
                    self.error_path = path;
                    self.error = message;
                }
                TabMsg::OpError { message } => {
                    self.last_error = message;
                    self.last_error_from_fetch = false;
                }
                TabMsg::Author {
                    name,
                    email,
                    complete,
                    sign_commits,
                    signing_format,
                } => {
                    self.author_name = name;
                    self.author_email = email;
                    self.identity_ready = complete;
                    self.signs_commits = sign_commits;
                    self.signing_format = signing_format;
                    self.compare_head_author();
                }
                TabMsg::Signature {
                    oid,
                    kind,
                    code,
                    signer,
                } => {
                    self.signature_oid = oid;
                    self.signature_kind = kind;
                    self.signature_code = code;
                    self.signature_signer = signer;
                }
                TabMsg::Remotes { names } => {
                    // A push with no upstream goes to `origin` when there
                    // is one, otherwise to whichever remote comes first.
                    self.default_remote = names
                        .iter()
                        .find(|r| *r == "origin")
                        .or_else(|| names.first())
                        .cloned()
                        .unwrap_or_default();
                    self.remote_count = names.len() as i32;
                    self.remote_names = names.join("\u{1f}");
                    self.remotes = names;
                }
                TabMsg::RemoteBranch {
                    remote,
                    branch,
                    state,
                    tip,
                    theirs,
                } => {
                    self.remote_branch_asked = format!("{remote}\u{1f}{branch}");
                    self.remote_branch_state = state;
                    self.remote_branch_tip = tip;
                    self.remote_branch_theirs = theirs;
                }
                TabMsg::BranchDelete { branch, merged } => {
                    self.branch_delete_asked = branch;
                    self.branch_delete_merged = merged;
                }
                TabMsg::HeadCommit {
                    message,
                    author_name,
                    author_email,
                } => {
                    let (subject, body) = platitude_core::commit::split_message(&message);
                    self.head_subject = subject;
                    self.head_body = body;
                    self.head_author_name = author_name;
                    self.head_author_email = author_email;
                    self.compare_head_author();
                    self.head_commit_seq += 1;
                }
                TabMsg::AutoFetch { running, error } => {
                    self.auto_fetch_running = running;
                    if !running {
                        self.auto_fetch_error = error.clone();
                        self.fetch_settled(&error);
                    }
                }
                TabMsg::Publish {
                    range,
                    total,
                    published,
                } => {
                    self.publish_range = range;
                    self.publish_total = total;
                    self.publish_published = published;
                }
                TabMsg::InHistory { oid, in_history } => {
                    self.history_oid = oid;
                    self.history_in = in_history;
                }
                TabMsg::HeadReach { reached_elsewhere } => {
                    self.head_reached_elsewhere = reached_elsewhere;
                }
                TabMsg::MoveNeedsAsk { local, start } => {
                    self.move_ask_local = local;
                    self.move_ask_start = start;
                    self.move_ask_seq += 1;
                }
                TabMsg::MergeTools { names, settled } => {
                    self.merge_tools = names.join("\u{1f}");
                    // The fast half arrives first; the indicator keeps
                    // turning until the slow read has had its say.
                    if settled {
                        self.merge_tools_loading = false;
                    }
                }
                TabMsg::WriteState { op, running, error } => {
                    if running {
                        self.busy_count += 1;
                        self.busy_op = op;
                    } else {
                        self.busy_count = (self.busy_count - 1).max(0);
                        if self.busy_count == 0 {
                            self.busy_op = String::new();
                        }
                        // A fetch the user asked for counts the same way
                        // the timer's do: what the button says is about
                        // fetching, not about who started it.
                        if op == "fetch" {
                            self.fetch_settled(&error.clone());
                        }
                        self.last_write_op = op;
                        self.last_write_error = error;
                        self.write_seq += 1;
                    }
                }
            }
        }
        self.changed();
    }

    /// Cheap refresh: refs + status + stashes (window focus, post-op).
    #[qslot]
    fn refresh_quick(&mut self) {
        if let Some(Some(session)) = Hub::with(|hub| hub.session(self.tab_id)) {
            session.refresh_quick();
        }
    }

    /// The tick the page runs while it is on screen: refs + status, and a
    /// graph rebuild only when one of them moved. Ticks that arrive while
    /// the session is busy are dropped there, not queued.
    #[qslot]
    fn refresh_poll(&mut self) {
        if let Some(Some(session)) = Hub::with(|hub| hub.session(self.tab_id)) {
            session.refresh_poll();
        }
    }

    /// Full refresh: restarts the log stream as well (manual refresh).
    #[qslot]
    fn refresh_all(&mut self) {
        if let Some(Some(session)) = Hub::with(|hub| hub.session(self.tab_id)) {
            session.restart_log();
            session.refresh_quick();
        }
    }

    #[qslot]
    fn clear_last_error(&mut self) {
        self.last_error = String::new();
        self.last_error_from_fetch = false;
        self.changed();
    }

    // --- write operations -----------------------------------------------
    //
    // Every one of these is fire-and-forget: the session serializes them,
    // reports progress through `busyCount` and routes git's own error text
    // into `lastError`. Paths arrive one per call (see `pending_paths`).

    #[qslot]
    fn stage_path(&mut self, path: String) {
        self.with_session(|s| s.stage_paths(vec![path.clone()]));
    }

    #[qslot]
    fn unstage_path(&mut self, path: String) {
        self.with_session(|s| s.unstage_paths(vec![path.clone()]));
    }

    /// The same two over the gathered set, for when several rows are
    /// highlighted and the affordance on one of them is pressed: one git
    /// command however many rows were chosen (デザイン規約 §その他の操作).
    #[qslot]
    fn stage_paths(&mut self) {
        let paths = std::mem::take(&mut self.pending_paths);
        if paths.is_empty() {
            return;
        }
        self.with_session(move |s| s.stage_paths(paths.clone()));
    }

    #[qslot]
    fn unstage_paths(&mut self) {
        let paths = std::mem::take(&mut self.pending_paths);
        if paths.is_empty() {
            return;
        }
        self.with_session(move |s| s.unstage_paths(paths.clone()));
    }

    /// Opens a set of paths for the next write, and adds to it. One call
    /// per path (see [`RepoTab::pending_paths`]); the write that follows
    /// takes them all in one git command, however many rows were chosen.
    #[qslot]
    fn begin_paths(&mut self) {
        self.pending_paths.clear();
    }

    #[qslot]
    fn add_path(&mut self, path: String) {
        self.pending_paths.push(path);
    }

    /// The same shape for the lines picked out of one diff: opened, added
    /// to one pair at a time, and taken whole by the write that follows
    /// (see [`RepoTab::pending_lines`]). Lines chosen by hand go over in
    /// one write so the diff is read once and rebuilt once — clicking them
    /// through one at a time would rebuild it under the pointer every
    /// time.
    #[qslot]
    fn begin_lines(&mut self) {
        self.pending_lines.clear();
    }

    #[qslot]
    fn add_line(&mut self, hunk: i32, line: i32) {
        self.pending_lines.push((hunk, line));
    }

    /// Stages (or unstages) the gathered lines of the shown diff. The
    /// direction follows the side being looked at, the way
    /// [`RepoTab::stage_selection`] does.
    #[qslot]
    fn stage_lines(&mut self, kind: String, path: String, orig_path: String, fingerprint: String) {
        let lines = std::mem::take(&mut self.pending_lines);
        let Some(target) = crate::encode::worktree_target(&kind, &path, &orig_path) else {
            tracing::warn!(kind, "line staging asked for a non-worktree diff");
            return;
        };
        let selects = crate::encode::line_selection(&lines);
        if selects.is_empty() {
            return;
        }
        let Ok(seen) = u64::from_str_radix(&fingerprint, 16) else {
            tracing::warn!(fingerprint, "line staging without a diff fingerprint");
            return;
        };
        self.with_session(|s| s.apply_partial(target.clone(), selects.clone(), seen));
    }

    /// Throws away unstaged modifications of the gathered files
    /// (destructive).
    #[qslot]
    fn discard_paths(&mut self) {
        let paths = std::mem::take(&mut self.pending_paths);
        if paths.is_empty() {
            return;
        }
        self.with_session(|s| s.discard_paths(paths.clone()));
    }

    /// Throws away both sides of the gathered files, back to HEAD
    /// (destructive). A rename's old name is one of the gathered paths:
    /// restoring only the new one leaves the old staged as a deletion.
    #[qslot]
    fn discard_paths_to_head(&mut self) {
        let paths = std::mem::take(&mut self.pending_paths);
        if paths.is_empty() {
            return;
        }
        self.with_session(|s| s.discard_paths_to_head(paths.clone()));
    }

    /// Deletes the gathered untracked files (destructive).
    #[qslot]
    fn remove_untracked_paths(&mut self) {
        let paths = std::mem::take(&mut self.pending_paths);
        if paths.is_empty() {
            return;
        }
        self.with_session(|s| s.remove_untracked(paths.clone()));
    }

    #[qslot]
    fn stage_all(&mut self) {
        self.with_session(|s| s.stage_all());
    }

    #[qslot]
    fn unstage_all(&mut self) {
        self.with_session(|s| s.unstage_all());
    }

    /// Stages (or unstages) part of one file's diff, addressed by the row
    /// the user clicked. `kind` is the diff-key prefix (`unstaged` /
    /// `staged` / `untracked`); a negative `line` takes the whole hunk.
    ///
    /// A staged diff is unstaged by the same call — the direction follows
    /// from which side the file is being looked at.
    #[qslot]
    fn stage_selection(
        &mut self,
        kind: String,
        path: String,
        orig_path: String,
        hunk: i32,
        line: i32,
        fingerprint: String,
    ) {
        let Some(target) = crate::encode::worktree_target(&kind, &path, &orig_path) else {
            tracing::warn!(kind, "selection staging asked for a non-worktree diff");
            return;
        };
        let selects = crate::encode::hunk_selection(hunk, line);
        if selects.is_empty() {
            return;
        }
        // The fingerprint of the diff the indices were made on (hex, from
        // DiffModel). Without one the selection addresses nothing.
        let Ok(seen) = u64::from_str_radix(&fingerprint, 16) else {
            tracing::warn!(fingerprint, "selection staging without a diff fingerprint");
            return;
        };
        self.with_session(|s| s.apply_partial(target.clone(), selects.clone(), seen));
    }

    /// Throws away part of one file's unstaged diff, addressed the same way
    /// (destructive). Only the unstaged side has a piece to throw away:
    /// what is staged is unstaged first, by the affordance beside this one.
    #[qslot]
    fn discard_selection(
        &mut self,
        kind: String,
        path: String,
        orig_path: String,
        hunk: i32,
        line: i32,
        fingerprint: String,
    ) {
        let Some(target) = crate::encode::worktree_target(&kind, &path, &orig_path) else {
            tracing::warn!(kind, "selection discard asked for a non-worktree diff");
            return;
        };
        let selects = crate::encode::hunk_selection(hunk, line);
        if selects.is_empty() {
            return;
        }
        let Ok(seen) = u64::from_str_radix(&fingerprint, 16) else {
            tracing::warn!(fingerprint, "selection discard without a diff fingerprint");
            return;
        };
        self.with_session(|s| s.discard_partial(target.clone(), selects.clone(), seen));
    }

    /// Commits the index from the editor's two fields. Both empty is only
    /// valid with `amend`, where it keeps the existing message.
    ///
    /// `reset_author` only means anything on an amend: it puts the current
    /// identity on a commit written under another one.
    #[qslot]
    fn commit(&mut self, subject: String, body: String, amend: bool, reset_author: bool) {
        let message = platitude_core::commit::join_message(&subject, &body);
        let options = platitude_core::commit::CommitOptions {
            amend,
            allow_empty: false,
            reset_author: amend && reset_author,
        };
        self.with_session(|s| s.commit(message.clone(), options));
    }

    /// Reads HEAD's message and author (an amend starts from them).
    #[qslot]
    fn request_head_commit(&mut self) {
        self.with_session(|s| s.load_head_commit());
    }

    // Every move is one job, whichever way it gets there: a failed half
    // cannot let the switch happen regardless.

    #[qslot]
    fn checkout_branch(&mut self, name: String) {
        let target = platitude_core::branch::CheckoutTarget::Branch { name };
        self.move_head(target);
    }

    /// Creates a local branch tracking a remote-tracking ref and switches.
    #[qslot]
    fn checkout_remote(&mut self, remote_ref: String, local: String) {
        let target = platitude_core::branch::CheckoutTarget::Track { remote_ref, local };
        self.move_head(target);
    }

    /// Moves an existing local branch to `start` and lands on it, but only
    /// where nothing is lost by it: a branch that has merely fallen behind
    /// fast-forwards straight away, and one holding commits of its own
    /// comes back as `moveAskSeq` for the UI to ask about.
    #[qslot]
    fn checkout_moving_branch(&mut self, local: String, start: String) {
        self.with_session(|s| s.checkout_moving_branch(local.clone(), start.clone()));
    }

    /// Moves an existing local branch to `start` and lands on it. What the
    /// branch alone had is left unreferenced, so the UI asks first.
    #[qslot]
    fn checkout_force_create(&mut self, local: String, start: String) {
        let target = platitude_core::branch::CheckoutTarget::ForceCreate { local, start };
        self.move_head(target);
    }

    /// Moves the current branch to `rev`. `mode` says what becomes of the
    /// index and the working tree: `"soft"` leaves both alone (the skipped
    /// commits end up staged), `"mixed"` clears the index, `"hard"` throws
    /// away everything uncommitted.
    #[qslot]
    fn reset_to(&mut self, rev: String, mode: String) {
        use platitude_core::branch::ResetMode;
        let mode = match mode.as_str() {
            "soft" => ResetMode::Soft,
            "mixed" => ResetMode::Mixed,
            "hard" => ResetMode::Hard,
            other => {
                tracing::warn!(mode = other, "unknown reset mode");
                return;
            }
        };
        self.with_session(|s| s.reset(rev.clone(), mode));
    }

    /// Creates a branch at `start_point` (HEAD when empty).
    #[qslot]
    fn create_branch(&mut self, name: String, start_point: String, switch_to: bool) {
        let start = (!start_point.is_empty()).then_some(start_point);
        self.with_session(|s| s.create_branch(name.clone(), start.clone(), switch_to));
    }

    /// Deletes a local branch. Without `force`, git refuses an unmerged one.
    #[qslot]
    fn delete_branch(&mut self, name: String, force: bool) {
        self.with_session(|s| s.delete_branch(name.clone(), force));
    }

    #[qslot]
    fn rename_branch(&mut self, from: String, to: String, force: bool) {
        self.with_session(|s| s.rename_branch(from.clone(), to.clone(), force));
    }

    /// Renames a tag. git has none, so core builds it out of a new name on
    /// the same object and a delete of the old one.
    #[qslot]
    fn rename_tag(&mut self, from: String, to: String) {
        self.with_session(|s| s.rename_tag(from.clone(), to.clone()));
    }

    /// Deletes a tag: only the name goes.
    #[qslot]
    fn delete_tag(&mut self, name: String) {
        self.with_session(|s| s.delete_tag(name.clone()));
    }

    /// Renames a stash entry. Built the same way, out of a re-store and a
    /// drop — so the entry moves to the top of the list.
    #[qslot]
    fn rename_stash(&mut self, selector: String, message: String) {
        self.with_session(|s| s.rename_stash(selector.clone(), message.clone()));
    }

    /// Whether a name is one git would take for a branch or a tag — asked
    /// per keystroke by the rename box, so it never runs git.
    #[qslot]
    fn valid_ref_name(&self, name: String) -> bool {
        platitude_core::tag::is_valid_name(&name)
    }

    /// The same question for a stash's label, which is free text on one
    /// line rather than a ref name.
    #[qslot]
    fn valid_stash_message(&self, message: String) -> bool {
        platitude_core::stash::is_valid_message(&message)
    }

    /// `git stash push` over the whole working tree.
    ///
    /// `staged_only` takes the index alone. It is not offered while a file
    /// is changed on both sides — git writes the stash entry and then
    /// fails to clear the tree, leaving an entry behind with nothing else
    /// done — so the caller checks `partiallyStagedCount` first.
    #[qslot]
    fn push_stash(
        &mut self,
        message: String,
        include_untracked: bool,
        keep_index: bool,
        staged_only: bool,
    ) {
        let options = platitude_core::stash::PushOptions {
            include_untracked,
            keep_index,
            staged_only,
        };
        self.with_session(|s| s.stash_push(message.clone(), options, Vec::new()));
    }

    /// `git stash push -- <paths>`: puts the gathered files' changes away
    /// and leaves the rest of the working tree as it is.
    ///
    /// Untracked files are included, since a path the user pointed at is
    /// meant to go whether or not git is tracking it yet.
    #[qslot]
    fn stash_paths(&mut self, message: String) {
        let paths = std::mem::take(&mut self.pending_paths);
        if paths.is_empty() {
            return;
        }
        let options = platitude_core::stash::PushOptions {
            include_untracked: true,
            keep_index: false,
            staged_only: false,
        };
        self.with_session(|s| s.stash_push(message.clone(), options, paths.clone()));
    }

    /// `git stash pop` on the given selector (stash-row action).
    #[qslot]
    fn pop_stash(&mut self, selector: String) {
        self.with_session(|s| s.stash_pop(selector.clone()));
    }

    /// `git stash apply` on the given selector (keeps the stash).
    #[qslot]
    fn apply_stash(&mut self, selector: String) {
        self.with_session(|s| s.stash_apply(selector.clone()));
    }

    /// `git stash drop` (destructive).
    #[qslot]
    fn drop_stash(&mut self, selector: String) {
        self.with_session(|s| s.stash_drop(selector.clone()));
    }

    /// `git fetch --prune`; an empty remote fetches all of them.
    #[qslot]
    fn fetch(&mut self, remote: String) {
        let remote = (!remote.is_empty()).then_some(remote);
        self.with_session(|s| s.fetch(remote.clone()));
    }

    /// Pushes the branch that is checked out to wherever it tracks, or to
    /// the default remote when it tracks nothing yet. `force` is `""` /
    /// `"lease"` / `"force"`; `lease_expect` pins the remote commit the
    /// user actually saw.
    #[qslot]
    fn push_current(&mut self, force: String, lease_expect: String) {
        let fallback = self.default_remote.clone();
        let force = Self::push_force(&force, &lease_expect);
        self.with_session(|s| s.push_current(fallback.clone(), force.clone()));
    }

    /// The first push of a branch, to the remote and name the question
    /// just took. Records the answer as the upstream, so the branch never
    /// asks again.
    #[qslot]
    fn publish_current(&mut self, remote: String, remote_branch: String, expect: String) {
        self.with_session(|s| {
            s.publish_current(remote.clone(), remote_branch.clone(), expect.clone())
        });
    }

    /// `git remote add <name> <url>`. Contacts nothing — a URL that goes
    /// nowhere is recorded just the same, and the push finds out.
    #[qslot]
    fn add_remote(&mut self, name: String, url: String) {
        self.with_session(|s| s.add_remote(name.clone(), url.clone()));
    }

    /// `git remote set-url <name> <url>` — the way back from a typo.
    #[qslot]
    fn set_remote_url(&mut self, name: String, url: String) {
        self.with_session(|s| s.set_remote_url(name.clone(), url.clone()));
    }

    /// Asks what a push under this branch name would meet on that remote.
    /// Reaches the network, so it is asked while the question stands and
    /// not on a poll. The answer arrives as `remoteBranchAsked` /
    /// `remoteBranchState`.
    #[qslot]
    fn check_remote_branch(&mut self, remote: String, branch: String) {
        self.remote_branch_asked = String::new();
        self.remote_branch_state = String::new();
        self.remote_branch_tip = String::new();
        self.remote_branch_theirs = 0;
        self.changed();
        self.with_session(|s| s.check_remote_branch(remote.clone(), branch.clone()));
    }

    /// Asks whether `git branch --delete` would refuse this branch (not
    /// merged into its upstream, or HEAD without one), so a menu's
    /// delete row can wear `-D` from the start. The answer arrives as
    /// `branchDeleteAsked` / `branchDeleteMerged`; no answer arrives
    /// where the reads fail, and the row stays on its plain form.
    #[qslot]
    fn check_branch_delete(&mut self, branch: String) {
        self.branch_delete_asked = String::new();
        self.branch_delete_merged = true;
        self.changed();
        self.with_session(|s| s.check_branch_delete(branch.clone()));
    }

    /// `git push`. `force` is `""` / `"lease"` / `"force"`; `lease_expect`
    /// pins the remote commit the user saw (empty = bare lease).
    #[qslot]
    fn push_branch(
        &mut self,
        remote: String,
        local: String,
        remote_branch: String,
        set_upstream: bool,
        force: String,
        lease_expect: String,
    ) {
        let force = Self::push_force(&force, &lease_expect);
        let spec = platitude_core::remote::PushSpec {
            remote,
            local,
            remote_branch,
            set_upstream,
            force,
        };
        self.with_session(|s| s.push(spec.clone()));
    }

    /// Renames a branch on a remote. git has none, so core pushes the new
    /// name and deletes the old — the UI holds the answer down first,
    /// because the old name is destroyed rather than moved.
    #[qslot]
    fn rename_remote_branch(&mut self, remote: String, from: String, to: String) {
        self.with_session(|s| s.rename_remote_branch(remote.clone(), from.clone(), to.clone()));
    }

    /// `git push <remote> --delete <branch>` (destructive).
    #[qslot]
    fn delete_remote_branch(&mut self, remote: String, branch: String) {
        self.with_session(|s| s.delete_remote_branch(remote.clone(), branch.clone()));
    }

    /// `git branch --delete` (`-D` under `force`) and then
    /// `git push <remote> --delete`, as one queued write: the local half
    /// refuses first where it refuses at all, and then the remote is
    /// never touched.
    #[qslot]
    fn delete_branch_everywhere(
        &mut self,
        branch: String,
        remote: String,
        remote_branch: String,
        force: bool,
    ) {
        self.with_session(|s| {
            s.delete_branch_everywhere(
                branch.clone(),
                remote.clone(),
                remote_branch.clone(),
                force,
            );
        });
    }

    /// `git merge <rev>`.
    #[qslot]
    fn merge(&mut self, rev: String, no_ff: bool, ff_only: bool, message: String) {
        let options = platitude_core::integrate::MergeOptions {
            no_ff,
            ff_only,
            squash: false,
            message: (!message.trim().is_empty()).then_some(message),
        };
        self.with_session(|s| s.merge(rev.clone(), options.clone()));
    }

    /// `git rebase <upstream>`; an empty `onto` uses `upstream` as the base.
    ///
    /// There is no autostash knob to pass: uncommitted work is carried
    /// across by core, the same way every other rewrite here carries it
    /// (デザイン規約 §未コミット変更がある状態で履歴を書き換える).
    #[qslot]
    fn rebase(&mut self, upstream: String, onto: String, update_refs: bool) {
        let options = platitude_core::integrate::RebaseOptions {
            onto: (!onto.is_empty()).then_some(onto),
            branch: None,
            update_refs,
            root: false,
        };
        self.with_session(|s| s.rebase(upstream.clone(), options.clone()));
    }

    #[qslot]
    fn cherry_pick(&mut self, rev: String) {
        self.with_session(|s| s.cherry_pick(vec![rev.clone()]));
    }

    /// Folds a commit into its parent (one-commit interactive rebase).
    #[qslot]
    fn squash_into_parent(&mut self, oid: String) {
        self.with_session(|s| s.squash_into_parent(oid.clone()));
    }

    /// Leaves one commit out of the history, replaying what came after it.
    #[qslot]
    fn drop_commit(&mut self, oid: String) {
        self.with_session(|s| s.drop_commit(oid.clone()));
    }

    /// Replaces one commit's message. HEAD is amended; anything older is
    /// replayed, which rewrites every commit after it.
    #[qslot]
    fn reword_commit(&mut self, oid: String, subject: String, body: String) {
        let message = platitude_core::commit::join_message(&subject, &body);
        if message.is_empty() {
            return;
        }
        self.with_session(|s| s.reword(oid.clone(), message.clone()));
    }

    #[qslot]
    fn revert(&mut self, rev: String) {
        self.with_session(|s| s.revert(vec![rev.clone()]));
    }

    /// Continues / aborts / skips whatever is in progress. `how` is
    /// `"continue"` / `"abort"` / `"skip"` / `"quit"`.
    #[qslot]
    fn resolve_operation(&mut self, how: String) {
        use platitude_core::integrate::Continuation;
        let continuation = match how.as_str() {
            "continue" => Continuation::Continue,
            "abort" => Continuation::Abort,
            "skip" => Continuation::Skip,
            "quit" => Continuation::Quit,
            other => {
                tracing::warn!(how = other, "unknown continuation");
                return;
            }
        };
        self.with_session(|s| s.resolve_current(continuation));
    }

    /// Resolves the gathered conflicted paths by taking one side
    /// (`"ours"`/`"theirs"`), in one git command however many were chosen.
    ///
    /// Which branch each side is called is `sideOurs` / `sideTheirs` on
    /// the working-tree model — during a rebase the two swap over, so the
    /// wording cannot be worked out from the flag alone.
    #[qslot]
    fn take_side_paths(&mut self, side: String) {
        use platitude_core::conflict::Side;
        let side = match side.as_str() {
            "ours" => Side::Ours,
            "theirs" => Side::Theirs,
            other => {
                tracing::warn!(side = other, "unknown conflict side");
                return;
            }
        };
        let paths = std::mem::take(&mut self.pending_paths);
        if paths.is_empty() {
            return;
        }
        self.with_session(|s| s.take_side(paths.clone(), side));
    }

    /// Opens the chosen conflicted paths in the configured merge tool, the
    /// same path-set route the rest of the file menu takes.
    ///
    /// Named paths only, never "all of them": git walks a bare `mergetool`
    /// one file at a time and the whole walk holds the write queue, so an
    /// unnamed launch would block every other write for as many tool
    /// sessions as there are conflicts.
    #[qslot]
    fn open_mergetool(&mut self) {
        let paths = std::mem::take(&mut self.pending_paths);
        if paths.is_empty() {
            return;
        }
        self.with_session(|s| s.mergetool(paths.clone()));
    }

    /// Records which merge tool to launch; empty clears the choice.
    #[qslot]
    fn set_merge_tool(&mut self, tool: String) {
        self.with_session(|s| s.set_merge_tool(tool.clone()));
    }

    /// Asks for the configured merge tool; the answer arrives on the
    /// working-tree model's `mergeTool`. The status refresh only names it
    /// where something is conflicted, so a settings field has to ask.
    #[qslot]
    fn ask_merge_tool(&mut self) {
        self.with_session(|s| s.ask_merge_tool());
    }

    /// Asks which tools could be offered; the answer lands on `mergeTools`.
    /// Off the write queue, and slow enough that `mergeToolsLoading` is
    /// worth showing while it runs.
    #[qslot]
    fn ask_merge_tools(&mut self) {
        if self.merge_tools_loading {
            return;
        }
        self.merge_tools_loading = true;
        self.with_session(|s| s.ask_merge_tools());
        self.changed();
    }

    /// Asks how much of `range` is already on a remote; the answer arrives
    /// as `publishRange` / `publishTotal` / `publishPublished`.
    #[qslot]
    fn check_publish(&mut self, range: String) {
        self.with_session(|s| s.check_publish(range.clone()));
    }

    /// Asks whether HEAD can reach `oid_hex` — whether a rewrite may start
    /// there; the answer arrives as `historyOid` / `historyIn`.
    #[qslot]
    fn check_in_history(&mut self, oid_hex: String) {
        let Ok(oid) = platitude_core::oid::Oid::from_hex_str(oid_hex.trim()) else {
            tracing::warn!(oid_hex, "invalid oid in history check");
            return;
        };
        self.with_session(|s| s.check_in_history(oid));
    }

    /// Asks what git makes of `oid_hex`'s signature; the answer arrives as
    /// `signatureOid` / `signatureKind` / `signatureCode` / `signatureSigner`.
    #[qslot]
    fn check_signature(&mut self, oid_hex: String) {
        let Ok(oid) = platitude_core::oid::Oid::from_hex_str(oid_hex.trim()) else {
            tracing::warn!(oid_hex, "invalid oid in signature check");
            return;
        };
        self.with_session(|s| s.check_signature(oid));
    }

    /// Records `user.name` / `user.email`. `global` writes the user's own
    /// configuration, which is the right default for a first-run prompt:
    /// the answer is about the person, not the project.
    #[qslot]
    fn set_identity(&mut self, name: String, email: String, global: bool) {
        let scope = if global {
            platitude_core::identity::ConfigScope::Global
        } else {
            platitude_core::identity::ConfigScope::Local
        };
        self.with_session(|s| s.set_identity(name.clone(), email.clone(), scope));
    }

    /// Time budget for fetch / push, in seconds. Zero is ignored.
    #[qslot]
    fn set_network_timeout(&mut self, seconds: i32) {
        if seconds <= 0 {
            return;
        }
        let timeout = std::time::Duration::from_secs(seconds as u64);
        self.with_session(|s| s.set_network_timeout(timeout));
    }

    /// Shows/hides tags in the graph walk (restarts the stream).
    #[qslot]
    fn set_tags_shown(&mut self, shown: bool) {
        if self.tags_shown == shown {
            return;
        }
        self.tags_shown = shown;
        self.changed();
        if let Some(Some(session)) = Hub::with(|hub| hub.session(self.tab_id)) {
            session.set_include_tags(shown);
        }
    }
}
qml_register!(RepoTab, "RepoTab", singleton = false);

#[cfg(test)]
mod tests {
    use super::*;

    // These stay off the first-failure branch: that one raises a Qt
    // signal, which wants an attached object. The fetch-recover verb
    // walks it in the real window instead.

    #[test]
    fn a_fetch_that_lands_takes_down_the_line_a_fetch_put_up() {
        let mut tab = RepoTab::default();
        tab.fetch_failures = 2;
        tab.last_error = "fatal: unable to access".into();
        tab.last_error_from_fetch = true;
        tab.fetch_settled("");
        assert_eq!(tab.fetch_failures, 0);
        assert_eq!(tab.last_error, "");
        assert!(!tab.last_error_from_fetch);
    }

    #[test]
    fn a_fetch_that_lands_leaves_a_background_reads_line_standing() {
        let mut tab = RepoTab::default();
        tab.last_error = "fatal: bad revision".into();
        tab.last_error_from_fetch = false;
        tab.fetch_settled("");
        assert_eq!(tab.fetch_failures, 0);
        assert_eq!(tab.last_error, "fatal: bad revision");
    }

    #[test]
    fn later_failures_neither_rewrite_nor_reclaim_the_line() {
        let mut tab = RepoTab::default();
        // The first failure's line has been dismissed, and a background
        // read has written its own news since.
        tab.fetch_failures = 1;
        tab.last_error = "fatal: bad revision".into();
        tab.last_error_from_fetch = false;
        tab.fetch_settled("fatal: unable to access");
        assert_eq!(tab.fetch_failures, 2);
        // The second failure is the same news as the first: it does not
        // touch the line, so it cannot claim it either.
        assert_eq!(tab.last_error, "fatal: bad revision");
        assert!(!tab.last_error_from_fetch);
        // And the recovery that follows respects the standing owner.
        tab.fetch_settled("");
        assert_eq!(tab.last_error, "fatal: bad revision");
    }
}
