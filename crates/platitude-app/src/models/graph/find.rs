//! The find bar's side of the rows: which of them the line is in, and
//! the marking that keeps that answer true as rows arrive and change.

use super::*;

impl GraphModel {
    /// Whether the query — if there is one — is somewhere in this row.
    ///
    /// The search sees names and addresses and never the marks beside
    /// them: a credit's identicon code is a number, and a reader that
    /// searched the whole record would answer a typed `12345` with
    /// somebody's face.
    pub(super) fn hits(query: &Query, item: &GraphRowItem) -> bool {
        let mut people: Vec<&str> = vec![item.author.as_str()];
        let mut addresses: Vec<&str> = vec![item.author_email.as_str()];
        for mate in item.co_authors.iter() {
            people.push(&mate.name);
            addresses.push(&mate.email);
        }
        let tokens: Vec<&str> = item.labels.iter().map(|c| c.name.as_str()).collect();
        // Two of the row's fields are withheld, each for its own reason
        // (`platitude-core::find`): `body` is the hover card's and does
        // not stand on the row, and `stash_ref` is the handle `apply`
        // and `pop` are run on and is not drawn on any row at all.
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
        self.settle_ends();
        self.settle_head();
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

    /// Re-reads whether the newest and the oldest loaded rows answer the
    /// query. Called wherever the rows or their marks move.
    pub(super) fn settle_ends(&mut self) {
        self.first_matched = self.rows.first().is_some_and(|r| r.matched);
        self.tail_matched = self.rows.last().is_some_and(|r| r.matched);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn hits(text: &str, item: &GraphRowItem) -> bool {
        GraphModel::hits(&Query::new(text).expect("a query"), item)
    }

    #[test]
    fn a_stash_row_answers_to_its_message_and_not_to_its_selector() {
        let row = GraphRowItem {
            oid_hex: "3f2a1b0cdeadbeef".into(),
            subject: "On main: the login refactor".into(),
            stash_ref: "stash@{0}".into(),
            ..GraphRowItem::default()
        };
        assert!(hits("login refactor", &row), "the row is its message");
        // The selector is drawn nowhere: the pane's band says `STASH`,
        // and the list on the left shows the message.
        assert!(!hits("stash@{0}", &row));
        assert!(!hits("stash@", &row));
    }

    /// The stand-in for HEAD's row reads its mark off the model, not off
    /// a delegate — the row is off screen whenever the stand-in is up —
    /// so a search that re-marks the rows has to re-mark that answer too.
    #[test]
    fn a_search_marks_the_head_row_for_its_stand_in() {
        let head = "a1".repeat(20);
        let oid = Oid::from_hex_str(&head).expect("an id");
        let mut model = GraphModel::default();
        model.rows = vec![GraphRowItem {
            oid_hex: head,
            subject: "the login refactor".into(),
            ..GraphRowItem::default()
        }];
        model.index = vec![(oid, 0)];
        model.head_oid = Some(oid);
        model.settle_head();
        assert!(!model.head_matched, "no search, no mark");

        model.query = Query::new("login");
        model.remark_notified();
        assert!(model.head_matched, "HEAD's row answers the search");

        model.query = Query::new("nowhere");
        model.remark_notified();
        assert!(!model.head_matched, "HEAD's row was passed over");

        model.query = None;
        model.remark_notified();
        assert!(!model.head_matched, "the search is off");
    }

    /// The footer carries the oldest loaded row's lanes on past the cut,
    /// at that row's strength, so a re-marking re-reads that row too.
    #[test]
    fn a_search_marks_the_oldest_row_for_the_footer() {
        let mut model = GraphModel::default();
        model.rows = vec![
            GraphRowItem {
                oid_hex: "b2".repeat(20),
                subject: "the newest".into(),
                ..GraphRowItem::default()
            },
            GraphRowItem {
                oid_hex: "c3".repeat(20),
                subject: "the oldest".into(),
                ..GraphRowItem::default()
            },
        ];
        model.query = Query::new("newest");
        model.remark_notified();
        assert!(!model.tail_matched, "the oldest row was passed over");

        model.query = Query::new("oldest");
        model.remark_notified();
        assert!(model.tail_matched, "the oldest row answers the search");

        model.query = None;
        model.remark_notified();
        assert!(!model.tail_matched, "the search is off");
    }
}
