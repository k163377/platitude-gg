//! What the diff pane does with what its feed hands it — a plain `impl`
//! beside the one-file `#[qobject]` (structure.md §分割).

use super::*;

impl DiffModel {
    pub(super) fn drain(&mut self) {
        let Some(feed) = self.feed.clone() else {
            return;
        };
        let mine: Vec<DiffMsg> = wanted(feed.drain(), &self.current_key)
            .into_iter()
            .filter(|msg| !matches!(msg, DiffMsg::Loaded { asked, .. } if self.waits_over(*asked)))
            .collect();
        if mine.is_empty() {
            return;
        }
        let carries_rows = mine.iter().any(|m| matches!(m, DiffMsg::Loaded { .. }));
        // Only a batch carrying rows takes it — the colours are a second
        // message about a read already answered.
        let asked = carries_rows.then(|| self.requested_at.take()).flatten();
        // Rows over rows on screen are the same file read again
        // (`begin_request_in` empties the pane for another): they move only
        // the rows that changed. Rows for an empty pane are a pane opening.
        let rereading = !self.lines.is_empty() && carries_rows;
        if rereading {
            self.rows_arriving();
        }
        for msg in mine {
            match msg {
                DiffMsg::Loaded {
                    patches,
                    preview,
                    fingerprint,
                    endings,
                    marks,
                    embedded,
                    asked: answers,
                    ..
                } => {
                    self.loading = false;
                    self.relabelled = false;
                    self.unread = false;
                    if let Some(t0) = &asked {
                        tracing::info!(
                            elapsed_ms = t0.elapsed().as_millis() as u64,
                            "diff request round trip"
                        );
                    }
                    self.is_binary = patches.iter().any(|p| p.is_binary);
                    self.is_new_file = is_new_file(&patches);
                    self.whole_why = whole_only_why(&patches).to_string();
                    (self.mode_from, self.mode_to) =
                        mode_change(&patches).cloned().unwrap_or_default();
                    self.is_combined = is_combined(&patches);
                    self.unmerged = is_unmerged_only(&patches);
                    self.embedded = embedded.is_some();
                    self.embedded_sha8 = match embedded {
                        Some(platitude_core::details::Embedded::On(oid)) => oid.short_hex(8),
                        _ => String::new(),
                    };
                    self.fingerprint = format!("{fingerprint:016x}");
                    self.apply_endings(endings.as_ref());
                    self.apply_preview(preview.as_deref());
                    // The rows on screen, keyed off what they were laid out
                    // from, before that is replaced.
                    let before = self.keys_on_screen();
                    self.shown_has_preview = preview.is_some();
                    self.shown = Some(Arc::clone(&patches));
                    self.shown_print = Some(fingerprint);
                    self.shown_asked = answers;
                    self.drawn_from = 0;
                    // git's word first: the parts it speaks for go, the
                    // others are moved onto it where they still stand
                    // (`ops::StageAhead::diff_drawn`) — then drawn over it.
                    let mine = self.current_at.is_empty();
                    crate::hub::diff_drawn(
                        self.tab_id,
                        mine.then(|| (self.current_key.clone(), fingerprint)),
                        mine.then_some(patches),
                        answers,
                    );
                    let parts = self.parts_now();
                    self.take_parts(parts);
                    self.shown_marks = marks;
                    self.shown_colors = Default::default();
                    self.coloured = false;
                    match before {
                        Some(keys) => self.lay_out_kept(&keys),
                        None => self.lay_out_rows(&Default::default()),
                    }
                }
                DiffMsg::Coloured {
                    colors, settled, ..
                } => {
                    // A deep diff's quick first answer is a stand-in; only
                    // the settled colours set the flag.
                    if settled {
                        self.coloured = true;
                    }
                    self.repaint_rows(&colors);
                    self.shown_colors = colors;
                }
                // No rows are coming: the pane stops waiting, keeps what it
                // shows (the last rows for a file read again) and says the
                // read failed; git's words are in the command log.
                DiffMsg::Unread { .. } => self.went_unread(),
            }
        }
        self.changed();
        if rereading {
            self.rows_arrived();
        }
        if let Some(t0) = asked {
            tracing::info!(
                elapsed_ms = t0.elapsed().as_millis() as u64,
                "diff rows applied"
            );
        }
    }
}
