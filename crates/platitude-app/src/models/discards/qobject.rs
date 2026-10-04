//! Everything QML sees of the discard log's reads and restores.

use super::*;

#[qobject(ConvertToCamelCase, NoQmlElement)]
impl DiscardModel {
    qproperty!("rows", Read = rows, Notify = changed);
    fn rows(&self) -> &DiscardRows {
        &self.rows
    }
    // "idle" | "reading" | "ready" | "error" — an empty list means "nothing
    // was taken away" only once `"ready"`.
    qproperty!("state", Member = state, Notify = changed);
    qproperty!("error", Member = error, Notify = changed);

    #[qsignal]
    pub(super) fn changed(&mut self);

    /// The tab whose session a restore goes to.
    #[qslot]
    fn attach(&mut self, tab_id: i32) {
        self.tab_id = tab_id;
    }

    /// Reads the repository at `path`, each reflog to `limit` lines from the
    /// newest (the graph's window, 破棄記録仕様.md §3); 0 reads them whole,
    /// for a history the window holds entirely.
    #[qslot]
    fn look(&mut self, path: String, limit: i32) {
        self.read(path, usize::try_from(limit).ok().filter(|n| *n > 0))
    }

    /// Where the entry's parts stand on the graph, each tip once, full hex.
    #[qslot]
    fn tips_at(&self, index: i32) -> Vec<String> {
        self.tips_of(index)
            .into_iter()
            .map(|(tip, _)| tip.to_hex())
            .collect()
    }

    /// How each of [`Self::tips_at`]'s tips draws: "commit" | "uncommitted"
    /// | "stash".
    #[qslot]
    fn looks_at(&self, index: i32) -> Vec<String> {
        self.tips_of(index)
            .into_iter()
            .map(|(_, look)| look.to_string())
            .collect()
    }

    /// The commits only the entry's tips reach, full hex.
    #[qslot]
    fn lost_at(&self, index: i32) -> Vec<String> {
        self.lost_of(index)
    }

    /// Each part's tip, full hex, in the parts' order — what has to be on
    /// the graph before the part can come back (破棄記録仕様.md §4).
    #[qslot]
    fn part_tips_at(&self, index: i32) -> Vec<String> {
        self.entry(index)
            .map(|entry| entry.parts.iter().map(|part| part.tip.to_hex()).collect())
            .unwrap_or_default()
    }

    /// Where each of the entry's copies on a base made for it draws:
    /// `<tip> <base> <commit>`, full hex (`GraphModel.showDiscard`).
    #[qslot]
    fn stands_at(&self, index: i32) -> Vec<String> {
        self.stands_of(index)
    }

    /// The entry's identity across reads — its kind, its moment and its
    /// parts' tips — "" for none: [`Self::index_of_key`] finds it again.
    #[qslot]
    fn key_at(&self, index: i32) -> String {
        self.entry(index).map(entry_key).unwrap_or_default()
    }

    /// Where the entry [`Self::key_at`] named stands now, -1 where this read
    /// lists it no more.
    #[qslot]
    fn index_of_key(&self, key: String) -> i32 {
        self.index_of(&key)
    }

    /// Brings back the entry's part `part`, or all of it with -1 — a write
    /// on the tab's session, answered as any other; false where the session
    /// did not take it.
    #[qslot]
    fn restore_at(&self, index: i32, part: i32) -> bool {
        self.restore(index, part)
    }

    /// The only slot an invoker ever calls (`models::mod`).
    #[qslot]
    fn drain(&mut self) {
        self.take_feed()
    }
}
