//! What the commit card does with what its feed hands it — a plain
//! `impl` beside the one-file `#[qobject]` (structure.md §分割).

use super::*;

impl DetailsModel {
    pub(super) fn drain(&mut self) {
        let Some(feed) = self.feed.clone() else {
            return;
        };
        let Some(msg) = feed.drain().pop() else {
            return;
        };
        if Some(msg.generation()) != self.requested_generation {
            return;
        }
        let details = match msg {
            crate::hub::DetailsMsg::Loaded { details, .. } => details,
            // A choice: only the file list arrives — the request settled
            // the header.
            crate::hub::DetailsMsg::Selection { files, .. } => {
                self.requested_at = None;
                self.loading = false;
                self.selection_loaded = true;
                self.take_files(&files);
                self.changed();
                return;
            }
            crate::hub::DetailsMsg::Failed {
                oid_hex, message, ..
            } => {
                // Only the spinner comes down (the failure itself is on
                // the error surface); an answer already on screen stays.
                if oid_hex == self.requested {
                    self.loading = false;
                    self.requested_at = None;
                    Hub::with(|hub| {
                        if let Some(feeds) = hub.feeds(self.tab_id) {
                            feeds.tab.push(crate::hub::TabMsg::op_error(message));
                        }
                    });
                    self.changed();
                }
                return;
            }
        };
        let hex = details.oid.to_hex();
        if hex != self.requested {
            return; // stale response for a previous selection
        }
        // For the read / apply marks (ci/baseline/perf-windows-x64.md
        // §操作 1 点の内訳).
        let asked = self.requested_at.take();
        if let Some(t0) = &asked {
            tracing::info!(
                elapsed_ms = t0.elapsed().as_millis() as u64,
                "details request round trip"
            );
        }
        self.sha_hex = hex;
        self.sha8 = details.oid.short_hex(8);
        self.parent_hexes = details.parents.iter().map(Oid::to_hex).collect();
        self.parent_hex = self.parent_hexes.first().cloned().unwrap_or_default();
        self.author_name = details.author_name.clone();
        self.author_email = details.author_email.clone();
        // Same input as the graph rows (author name) → same identicon.
        self.avatar = crate::encode::avatar_code(&details.author_name);
        self.avatar_url =
            Hub::with(|hub| hub.avatar_url(&details.author_email)).unwrap_or_default();
        self.co_authors = crate::encode::mates_of(&details.co_authors);
        self.author_time = details.author_time;
        self.committer_name = details.committer_name.clone();
        self.committer_email = details.committer_email.clone();
        self.committer_avatar = crate::encode::avatar_code(&details.committer_name);
        self.committer_avatar_url =
            Hub::with(|hub| hub.avatar_url(&details.committer_email)).unwrap_or_default();
        // The address tells two people apart (as it keys a face,
        // `platitude_core::avatar`); it arrives already mailmapped.
        self.committer_differs = !details
            .committer_email
            .eq_ignore_ascii_case(&details.author_email);
        self.commit_time_differs = details.committer_time != details.author_time;
        self.committer_time = details.committer_time;
        // Subject / body split mirrors the commit-editor fields.
        let (subject, body) = details
            .message
            .split_once('\n')
            .map(|(s, b)| (s.to_string(), b.trim_start_matches('\n').to_string()))
            .unwrap_or_else(|| (details.message.clone(), String::new()));
        self.message_subject = subject;
        self.message_body = body;
        self.loading = false;
        self.take_files(&details.files);
        self.changed();
        if let Some(t0) = asked {
            tracing::info!(
                elapsed_ms = t0.elapsed().as_millis() as u64,
                "details rows applied"
            );
        }
    }
}
