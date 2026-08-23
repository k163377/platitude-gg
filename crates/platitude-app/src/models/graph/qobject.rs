//! Everything QML sees of the graph: the properties it binds to, the
//! feed it drains, and the questions it asks of the loaded rows.
//!
//! One `#[qobject]` block, and it cannot be split further — QMetaInfo is
//! built per file (app-ui.md).

use super::*;

#[qobject(Base = QListModel, ConvertToCamelCase, NoQmlElement)]
impl GraphModel {
    qproperty!("loading", Member = loading, Notify = stats_changed);
    // Chips the window is already showing as gone while git is still
    // being asked to delete the refs they name — kind letter + name,
    // separated by U+001F (デザイン規約 §消す操作は先に画面から消す). Held
    // here, beside the rows that carry those names, for the same reason
    // the sidebar's sections hold theirs (`NavSectionModel::set_hidden`):
    // the page tells every list what the window is standing in for, and
    // the drawing is where the names are left out — the rows still have
    // them until the walk that follows the delete lands.
    qproperty!("goneChips", Member = gone_chips, Notify = stats_changed);
    qproperty!("rowTotal", Member = row_total, Notify = stats_changed);
    qproperty!("walkedTotal", Member = walked_total, Notify = stats_changed);
    qproperty!("maxLanes", Member = max_lanes, Notify = stats_changed);
    qproperty!(
        "firstChunkMs",
        Member = first_chunk_ms,
        Notify = stats_changed
    );
    qproperty!("totalMs", Member = total_ms, Notify = stats_changed);
    qproperty!("truncated", Member = truncated, Notify = stats_changed);
    qproperty!("finishCount", Member = finish_count, Notify = stats_changed);
    qproperty!("resetCount", Member = reset_count, Notify = stats_changed);
    // How many loaded rows the find bar's line is in. A property rather
    // than the return of the slot that sets the query: a background
    // refresh re-marks the rows without anybody typing, and a binding is
    // the only thing that hears about that (app-ui.md §QML バインディング
    // はプロパティにしか反応しない). Doc comments do not go on
    // `qproperty!` — the macro rejects attributes.
    qproperty!("matchCount", Member = match_count, Notify = stats_changed);
    // Whether a search is on, and whether the newest row answers it. Both
    // properties for the same reason `matchCount` is.
    qproperty!("searching", Member = searching, Notify = stats_changed);
    qproperty!(
        "firstMatched",
        Member = first_matched,
        Notify = stats_changed
    );
    qproperty!(
        "tailGeometry",
        Member = tail_geometry,
        Notify = stats_changed
    );
    // Which row the working tree stands on and what a stand-in for it
    // draws. Properties for the reason `matchCount` is one: the rows
    // arrive in chunks and their chips a pass later, and the pane has to
    // hear about both (`head::settle_head`).
    qproperty!("headRow", Member = head_row, Notify = stats_changed);
    qproperty!("headLabels", Member = head_labels, Notify = stats_changed);
    qproperty!("headSubject", Member = head_subject, Notify = stats_changed);
    qproperty!("headColor", Member = head_color, Notify = stats_changed);
    qproperty!("headLane", Member = head_lane, Notify = stats_changed);
    qproperty!(
        "headGeometry",
        Member = head_geometry,
        Notify = stats_changed
    );
    qproperty!("headAvatar", Member = head_avatar, Notify = stats_changed);
    qproperty!(
        "headAvatarUrl",
        Member = head_avatar_url,
        Notify = stats_changed
    );
    qproperty!("error", Member = error, Notify = stats_changed);

    #[qsignal]
    pub(super) fn stats_changed(&mut self);

    /// Names the refs whose chips are to be left undrawn — at most one
    /// per kind, since a delete touches at most one of each; empty
    /// halves put theirs back, which is what a refused delete does. The
    /// packed set the delegates filter with (`goneChips`,
    /// `encode::gone_keys`) is built here: which side of the bridge
    /// spells a wire format is not the page's business.
    #[qslot]
    fn set_gone(&mut self, branch: String, remote: String, tag: String) {
        let keys = crate::encode::gone_keys(&branch, &remote, &tag);
        if self.gone_chips == keys {
            return;
        }
        self.gone_chips = keys;
        self.stats_changed();
    }

    #[qslot]
    fn attach(&mut self, tab_id: i32) {
        self.tab_id = tab_id;
        let invoker = self.get_qml_method_invoker();
        self.feed = crate::hub::attach_feed(tab_id, |f| &f.graph, invoker);
    }

    #[qslot]
    fn drain(&mut self) {
        self.take_feed();
    }

    /// Row index of a commit (sidebar jump); -1 when absent.
    #[qslot]
    fn row_of(&self, oid_hex: String) -> i32 {
        self.rows
            .iter()
            .position(|r| r.oid_hex == oid_hex)
            .map_or(-1, |i| i as i32)
    }

    /// Reflog selector when the commit is a stash row (empty otherwise).
    #[qslot]
    fn stash_ref_of(&self, oid_hex: String) -> String {
        self.rows
            .iter()
            .find(|r| r.oid_hex == oid_hex)
            .map(|r| r.stash_ref.clone())
            .unwrap_or_default()
    }

    /// The lane colour of the row a ref sits on, as an index into the
    /// graph palette; -1 when no row on screen carries that name.
    ///
    /// What it is for: a conflicted file's diff paints each side in the
    /// colour its branch already has in the graph, so the pane borrows an
    /// answer rather than inventing a second one. -1 is a real answer —
    /// the walk is a window (`--max-count`), and a branch outside it has
    /// no colour to borrow.
    #[qslot]
    fn color_of_ref(&self, name: String) -> i32 {
        if name.is_empty() {
            return -1;
        }
        self.rows
            .iter()
            .find(|r| crate::encode::label_names(&r.labels).any(|n| n == name))
            .map_or(-1, |r| r.node_color)
    }

    /// The colour each side of a conflict is drawn in — the graph's answer
    /// where it has one, a stable one off the name where it does not, and
    /// never the same on both sides
    /// (`encode::conflict_side_colors` decides; these two only pick a half
    /// out of its answer, since a slot cannot hand back a pair).
    #[qslot]
    fn conflict_color_ours(&self, ours: String, theirs: String) -> i32 {
        self.conflict_colors(&ours, &theirs).0
    }

    #[qslot]
    fn conflict_color_theirs(&self, ours: String, theirs: String) -> i32 {
        self.conflict_colors(&ours, &theirs).1
    }

    /// Re-reads the assigned pictures onto the rows already loaded.
    ///
    /// Assigning one is not a thing git knows about, so nothing about the
    /// repository changed and re-walking the history to find that out
    /// would be the most expensive way to move a handful of pixels. The
    /// splice only notifies the rows whose author actually got one.
    #[qslot]
    fn refresh_avatars(&mut self) {
        let avatars = crate::hub::AvatarUrls::current();
        let rows: Vec<GraphRowItem> = self
            .rows
            .iter()
            .map(|row| GraphRowItem {
                avatar_url: avatars.url_of(&row.author_email),
                ..row.clone()
            })
            .collect();
        self.splice_notified(rows);
    }

    /// How many loaded rows carry a picture. Automation only: QML cannot
    /// walk this model's rows, so counting them there would be counting
    /// nothing (measured — a helper doing exactly that reported zero
    /// while the faces were on screen).
    #[qslot]
    fn avatar_row_count(&self) -> i32 {
        self.rows
            .iter()
            .filter(|row| !row.avatar_url.is_empty())
            .count() as i32
    }

    /// The authors of the loaded rows as the settings card's entry shows
    /// them — `Name <address>`, one per address, sorted, joined by
    /// `\u{1f}`. A prefill (the badge's own author) goes first and its
    /// address is not repeated below.
    ///
    /// What the card offers instead of asking somebody to type an
    /// address: the people whose commits are on screen are the people
    /// whose faces are worth setting. Read when the card opens, off rows
    /// already in memory — no git runs for it.
    #[qslot]
    fn author_choices(&self, prefill_name: String, prefill_email: String) -> String {
        let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
        let mut listed: Vec<String> = Vec::new();
        for row in &self.rows {
            if row.author_email.is_empty()
                || row.author_email == prefill_email
                || !seen.insert(&row.author_email)
            {
                continue;
            }
            listed.push(format!("{} <{}>", row.author, row.author_email));
        }
        listed.sort();
        let mut out: Vec<String> = Vec::new();
        if !prefill_email.is_empty() {
            out.push(format!("{prefill_name} <{prefill_email}>"));
        }
        out.extend(listed);
        out.join("\u{1f}")
    }

    /// Puts the find bar's line to the rows, lighting the ones it is in.
    ///
    /// An empty line — or one that is only whitespace — is not a search
    /// (`find::Query::new`): the marks come off and `matchCount` goes to
    /// zero, which is what the bar reads as "nothing is being looked
    /// for".
    #[qslot]
    fn set_find(&mut self, text: String) {
        let next = Query::new(&text);
        if next == self.query {
            return;
        }
        self.query = next;
        self.searching = self.query.is_some();
        self.remark_notified();
        self.stats_changed();
    }

    /// Row of the first match at or after `from`, wrapping to the first
    /// match of all when there is none below; -1 when nothing matches.
    /// Where an incremental search lands.
    #[qslot]
    fn match_from(&self, from: i32) -> i32 {
        self.match_rows()
            .find(|row| *row >= from)
            .or_else(|| self.match_rows().next())
            .unwrap_or(-1)
    }

    /// Row of the next match after `row`, wrapping past the end.
    #[qslot]
    fn match_after(&self, row: i32) -> i32 {
        self.match_rows()
            .find(|r| *r > row)
            .or_else(|| self.match_rows().next())
            .unwrap_or(-1)
    }

    /// Row of the previous match before `row`, wrapping past the start.
    #[qslot]
    fn match_before(&self, row: i32) -> i32 {
        self.match_rows()
            .rfind(|r| *r < row)
            .or_else(|| self.match_rows().next_back())
            .unwrap_or(-1)
    }

    /// Which match this row is, counting from 1; 0 when it is not one.
    /// The left half of the bar's count.
    #[qslot]
    fn match_ordinal(&self, row: i32) -> i32 {
        self.match_rows()
            .position(|r| r == row)
            .map_or(0, |i| i as i32 + 1)
    }

    /// Full commit id at a row (selection, its recovery after a rewrite,
    /// and the smoke hooks).
    #[qslot]
    fn oid_at(&self, row: i32) -> String {
        usize::try_from(row)
            .ok()
            .and_then(|i| self.rows.get(i))
            .map(|r| r.oid_hex.clone())
            .unwrap_or_default()
    }

    /// The draw tokens of a row (`encode::encode_geometry`) — the same
    /// string its delegate paints from. For the smoke hooks: a lane is a
    /// stroke a couple of pixels wide, and whether one of them is dotted
    /// is not a question a screenshot answers.
    #[qslot]
    fn geometry_at(&self, row: i32) -> String {
        usize::try_from(row)
            .ok()
            .and_then(|i| self.rows.get(i))
            .map(|r| r.geometry.clone())
            .unwrap_or_default()
    }
}
impl GraphModel {
    /// Shared by the two slots above, so the pair is decided once.
    fn conflict_colors(&self, ours: &str, theirs: &str) -> (i32, i32) {
        crate::encode::conflict_side_colors(
            (self.color_of_ref(ours.to_string()), ours),
            (self.color_of_ref(theirs.to_string()), theirs),
        )
    }
}
