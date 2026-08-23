//! One tab's feed, drained into the properties QML reads.

use super::*;

impl RepoTab {
    #[expect(clippy::too_many_lines)]
    pub(super) fn take_feed(&mut self) {
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
                    self.read_author_avatar();
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
                TabMsg::Remotes {
                    names,
                    urls,
                    push_default,
                    push_default_local,
                } => {
                    // The marked remote is where pushes go. Only where
                    // nothing is marked does the old guess stand: `origin`
                    // when there is one, otherwise whichever remote comes
                    // first. A mark naming a remote this repository does
                    // not have is left out of it — git would take that
                    // name for a URL, and this application has nothing to
                    // point at.
                    let marked = names.iter().find(|r| **r == push_default);
                    self.default_remote = marked
                        .or_else(|| names.iter().find(|r| *r == "origin"))
                        .or_else(|| names.first())
                        .cloned()
                        .unwrap_or_default();
                    self.push_default = marked.cloned().unwrap_or_default();
                    self.push_default_local = push_default_local;
                    self.remote_count = names.len() as i32;
                    self.remote_names = names.join("\u{1f}");
                    self.remotes = names;
                    self.remote_urls = urls;
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
                TabMsg::AutoFetch {
                    running,
                    error,
                    announce,
                } => {
                    self.auto_fetch_running = running;
                    if !running {
                        self.auto_fetch_error = error.clone();
                        self.fetch_settled(&error, announce);
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
                // Arrives between this write's start and its end, so the
                // flag is already standing when the answer below is read.
                TabMsg::WriteStopped => self.last_write_stopped = true,
                TabMsg::WriteState { op, running, error } => {
                    if running {
                        self.busy_count += 1;
                        self.busy_op = op;
                        // Whatever the last write left standing, this one
                        // has not stopped yet.
                        self.last_write_stopped = false;
                    } else {
                        self.settle_write(op, error);
                    }
                }
            }
        }
        self.changed();
    }

    /// One write's answer, folded into the properties the page reads.
    ///
    /// The op names are turned into meanings **here**, on this side of
    /// the bridge, so the page sequences the screen — reload the diff,
    /// arm a landing, put taken rows back — without ever branching on
    /// git vocabulary (app-ui.md: no business logic in QML). Every
    /// answer rewrites the whole group, so nothing stays armed for a
    /// later write to trip over; `write_seq` says which answer the
    /// group describes.
    pub(super) fn settle_write(&mut self, op: String, error: String) {
        self.busy_count = (self.busy_count - 1).max(0);
        if self.busy_count == 0 {
            self.busy_op = String::new();
        }
        // A fetch the user asked for counts the same way the timer's do:
        // what the button says is about fetching, not about who started
        // it.
        if op == "fetch" {
            self.fetch_settled(&error, true);
        }
        let landed = error.is_empty();
        self.write_refused = !landed;
        // A landed write moved what the two sides hold; a refused stage,
        // unstage or discard was refused *because* the rows on screen
        // drifted (the fingerprint refuses on nothing else). Both mean
        // the shown diff no longer describes the file.
        self.write_stale_diff = matches!(op.as_str(), "stage" | "unstage" | "discard")
            || (landed && matches!(op.as_str(), "commit" | "stash"));
        // Stopped part-way is not a landing: there is no commit at the
        // tip to go to, and the answer to the press is the working tree
        // (`last_write_stopped`, raised by the message before this one).
        self.write_at_tip = landed
            && !self.last_write_stopped
            && matches!(op.as_str(), "revert" | "cherry-pick" | "merge");
        self.write_moved_head = landed && matches!(op.as_str(), "checkout" | "reset");
        self.write_committed = landed && op == "commit";
        self.write_reworded = landed && op == "reword";
        self.write_stashed = landed && op == "stash";
        self.write_branch_op = op == "branch";
        self.write_pushed = op == "push";
        self.last_write_op = op;
        self.last_write_error = error;
        self.write_seq += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settled(op: &str, error: &str) -> RepoTab {
        let mut tab = RepoTab::default();
        tab.settle_write(op.into(), error.into());
        tab
    }

    #[test]
    fn a_landed_stage_leaves_the_shown_diff_stale() {
        let tab = settled("stage", "");
        assert!(!tab.write_refused);
        assert!(tab.write_stale_diff);
    }

    #[test]
    fn a_refused_stage_is_the_drifted_rows_own_answer() {
        let tab = settled("stage", "error: patch does not apply");
        assert!(tab.write_refused);
        assert!(
            tab.write_stale_diff,
            "the refusal's answer is the fresh file"
        );
    }

    #[test]
    fn a_refused_commit_asks_for_no_reread() {
        let tab = settled("commit", "nothing to commit, working tree clean");
        assert!(tab.write_refused);
        assert!(!tab.write_stale_diff);
        assert!(
            !tab.write_committed,
            "a rejected commit keeps the editor's text"
        );
    }

    #[test]
    fn a_landed_commit_lands_and_moves_the_sides() {
        let tab = settled("commit", "");
        assert!(tab.write_committed);
        assert!(tab.write_stale_diff);
        assert!(
            !tab.write_at_tip,
            "the landing is the editor's, not a tip jump"
        );
    }

    #[test]
    fn a_merge_that_landed_answers_at_the_tip() {
        assert!(settled("merge", "").write_at_tip);
        assert!(settled("cherry-pick", "").write_at_tip);
        assert!(!settled("merge", "fatal: refusing to merge").write_at_tip);
    }

    #[test]
    fn a_merge_that_stopped_does_not_claim_the_tip() {
        let mut tab = RepoTab::default();
        // The stop arrives before the answer that ends the write
        // (`TabMsg::WriteStopped`), so the flag is already standing.
        tab.last_write_stopped = true;
        tab.settle_write("merge".into(), String::new());
        assert!(!tab.write_at_tip, "nothing landed at the tip to go to");
    }

    #[test]
    fn a_branch_answer_says_so_whichever_way_it_went() {
        assert!(settled("branch", "").write_branch_op);
        assert!(settled("branch", "error: not fully merged").write_branch_op);
        assert!(!settled("tag", "").write_branch_op);
    }

    #[test]
    fn a_push_answer_says_so_whichever_way_it_went() {
        assert!(settled("push", "").write_pushed);
        assert!(settled("push", "! [rejected]").write_pushed);
    }

    #[test]
    fn a_landed_checkout_or_reset_moved_head() {
        assert!(settled("checkout", "").write_moved_head);
        assert!(settled("reset", "").write_moved_head);
        assert!(!settled("checkout", "fatal: invalid reference").write_moved_head);
    }

    #[test]
    fn a_landed_reword_carries_the_saved_message() {
        assert!(settled("reword", "").write_reworded);
        assert!(!settled("reword", "fatal: bad revision").write_reworded);
    }

    #[test]
    fn every_answer_rewrites_the_whole_group() {
        let mut tab = RepoTab::default();
        tab.settle_write("stash".into(), String::new());
        assert!(tab.write_stashed);
        // Not a fetch: a failed fetch raises `fetch_first_failed`, and a
        // signal needs the proxy no unit test has.
        tab.settle_write("checkout".into(), "fatal: invalid reference".into());
        assert!(!tab.write_stashed, "nothing armed survives the next answer");
        assert!(tab.write_refused);
        assert_eq!(tab.write_seq, 2);
    }
}
