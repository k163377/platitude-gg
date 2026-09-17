//! The find bar's side of the rows: which of them the line is in, and
//! the marking that keeps that answer true as rows arrive and change.

use super::*;

impl GraphModel {
    /// Whether the query — if there is one — is somewhere in this row.
    ///
    /// The three packed fields are unpacked through the readers that sit
    /// beside their encoders, so the search sees names and addresses and
    /// never the flags, separators or identicon codes they are packed
    /// with.
    pub(super) fn hits(query: &Query, item: &GraphRowItem) -> bool {
        let mut people: Vec<&str> = vec![item.author.as_str()];
        let mut addresses: Vec<&str> = vec![item.author_email.as_str()];
        if !item.co_authors.is_empty() {
            for (name, address) in co_author_pairs(&item.co_authors) {
                people.push(name);
                addresses.push(address);
            }
        }
        let mut tokens: Vec<&str> = label_names(&item.labels).collect();
        if !item.stash_ref.is_empty() {
            tokens.push(item.stash_ref.as_str());
        }
        // `body` is not passed: the description is not searched (see
        // `platitude-core::find`). The row still carries it for the hover
        // card.
        query.matches(&Row {
            oid_hex: &item.oid_hex,
            subject: &item.subject,
            people: &people,
            addresses: &addresses,
            tokens: &tokens,
        })
    }

    /// Sets `matched` on the rows already in the model and counts them.
    ///
    /// Written in place: [`Self::splice_notified`] takes a whole new
    /// `Vec`, and cloning every row's strings on every keystroke is
    /// exactly the work this search exists to avoid. Only the runs that
    /// actually changed are notified.
    pub(super) fn remark_notified(&mut self) {
        let mut ranges: Vec<(usize, usize)> = Vec::new();
        let mut count = 0;
        for i in 0..self.rows.len() {
            let now = match &self.query {
                Some(q) => Self::hits(q, &self.rows[i]),
                None => false,
            };
            if now {
                count += 1;
            }
            if self.rows[i].matched != now {
                self.rows[i].matched = now;
                push_run(&mut ranges, i);
            }
        }
        self.match_count = count;
        self.settle_first();
        self.notify_runs(ranges);
    }

    /// Marks rows on their way in, before anyone sees them.
    ///
    /// A row arriving under a standing query has to arrive already lit:
    /// marking it afterwards would notify a change on a row nobody has
    /// drawn yet, and a chunk streaming in mid-search would flicker dark
    /// for a frame.
    pub(super) fn mark_incoming(&self, items: &mut [GraphRowItem]) {
        let Some(query) = &self.query else {
            return;
        };
        for item in items {
            item.matched = Self::hits(query, item);
        }
    }

    /// Re-reads whether the newest row answers the query. Called wherever
    /// the rows or their marks move.
    pub(super) fn settle_first(&mut self) {
        self.first_matched = self.rows.first().is_some_and(|r| r.matched);
    }

    /// Rows answering the query, as their indices in order.
    pub(super) fn match_rows(&self) -> impl DoubleEndedIterator<Item = i32> {
        self.rows
            .iter()
            .enumerate()
            .filter(|(_, r)| r.matched)
            .map(|(i, _)| i as i32)
    }
}
