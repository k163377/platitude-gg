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
        self.take_header(hex, &details);
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

    /// What the card says of the commit `hex` above its files: who wrote
    /// and committed it, when, and its message — as recorded, and as a
    /// save would write it.
    pub(super) fn take_header(
        &mut self,
        hex: String,
        details: &platitude_core::details::CommitDetails,
    ) {
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
        // The address tells two people apart, folded as it keys a face
        // (`avatar::key`); it arrives already mailmapped.
        self.committer_differs = platitude_core::avatar::key(&details.committer_email)
            != platitude_core::avatar::key(&details.author_email);
        self.commit_time_differs = details.committer_time != details.author_time;
        self.committer_time = details.committer_time;
        // Shown as recorded; an editable card's fields take the cut a save
        // writes as a caret comes in (`DetailsPane.startWriting`).
        (self.message_subject, self.message_body) =
            platitude_core::commit::shown_message(&details.message);
        (self.edit_subject, self.edit_body) =
            platitude_core::commit::split_message(&details.message);
    }
}

#[cfg(test)]
mod tests {
    use platitude_core::Oid;
    use platitude_core::details::CommitDetails;

    use super::DetailsModel;

    /// The card stood on a commit by `author`, committed by `committer`,
    /// with `message`.
    fn card_of(author: &str, committer: &str, message: &str) -> DetailsModel {
        let oid = Oid::from_hex_str(&"1".repeat(40)).expect("a full object name");
        let mut model = DetailsModel::default();
        model.take_header(
            oid.to_hex(),
            &CommitDetails {
                oid,
                parents: Vec::new(),
                author_name: "Ada".to_string(),
                author_email: author.to_string(),
                author_time: 0,
                committer_name: "Ada".to_string(),
                committer_email: committer.to_string(),
                committer_time: 0,
                co_authors: Vec::new(),
                message: message.to_string(),
                files: Vec::new(),
            },
        );
        model
    }

    /// The card shows the message as recorded — the subject's indent and
    /// the body's blanks stay, a CRLF is only a line break
    /// (`commit::shown_message`) — and holds beside it the cut a save
    /// writes (`commit::split_message`), which the fields take as a caret
    /// comes in.
    #[test]
    fn the_card_shows_a_message_as_recorded_and_holds_the_cut_a_save_writes() {
        let message = "    indented subject\r\n\r\nThe body.   \r\nmore";
        let card = card_of("ada@example.com", "ada@example.com", message);
        assert_eq!(
            (card.message_subject.as_str(), card.message_body.as_str()),
            ("    indented subject", "The body.   \nmore")
        );
        assert_eq!(
            (card.edit_subject, card.edit_body),
            platitude_core::commit::split_message(message),
            "the editor's own cut"
        );
    }

    /// Two addresses are one person where they fold to one face
    /// (`avatar::key`): letter case beyond ASCII and the blanks around them
    /// tell nobody apart; another address does.
    #[test]
    fn the_committer_is_told_apart_by_the_key_a_face_is_filed_under() {
        let differs =
            |author: &str, committer: &str| card_of(author, committer, "subject").committer_differs;
        assert!(
            !differs("ÄDA@example.com", "äda@example.com"),
            "case beyond ASCII"
        );
        assert!(
            !differs(" ada@example.com", "ada@example.com"),
            "blanks around"
        );
        assert!(
            differs("ada@example.com", "grace@example.com"),
            "another person"
        );
    }
}
