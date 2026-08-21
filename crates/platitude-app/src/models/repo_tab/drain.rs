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
                            self.fetch_settled(&error.clone(), true);
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
}
