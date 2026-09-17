//! The configuration writes the application itself asks for — the
//! identity screens' — held from the ask until git has answered, by
//! something that outlives the screen that asked.
//!
//! **Why an owner, and why here.** A save is spawned against a path
//! (`models::repo_config`, `AppBackend`), because the settings screen
//! offers every repository in the strip and only the tab in front has a
//! session. Spawned bare, a save was a task
//! nobody held: the screen that asked could go, the tab it was about
//! could go, and the window could close with `git config` half way
//! through the pair — the quit gate counted the sessions' writes and
//! knew nothing of this one. So the hub holds them ([`Hub::spawn_save`]),
//! the quit gate counts them with the sessions' writes
//! (`Hub::writes_settled`) and the shutdown joins them
//! (`Hub::shutdown`), the same three places a session's local write is
//! seen from.
//!
//! **Holding is all this does.** Which slot the save's commands run in
//! is the executor's business (`platitude_core::process::Slots`); what
//! the answer means is the screen's, and travels on the feed the save
//! was given — a screen that has since gone leaves the answer queued
//! there, unread and harmless. A save's own token is nobody's to
//! cancel: a write asked for is a write finished, half a pair being the
//! one thing worse than either whole.

use platitude_core::GitExecutor;
use tokio::task::JoinHandle;

/// The handle a save runs on, from the application's: the stock time
/// budget lifted, as the write queue lifts it for its local lane
/// (`operation::Lane::Local`). A save killed at a budget is the
/// half-landed pair this module exists to rule out — and it would be
/// killed under the very shutdown that waits for it — while a `git
/// config` that is slow is slow for a reason of the machine's, and
/// ends.
pub(super) fn executor_for(base: &GitExecutor) -> GitExecutor {
    base.clone().without_stock_timeouts()
}

/// The saves still out. Finished ones are let go wherever the count is
/// read, so nothing here grows with the number of saves a run makes.
#[derive(Default)]
pub struct Saves {
    running: Vec<JoinHandle<()>>,
}

impl Saves {
    /// Holds `save` until it ends.
    pub fn hold(&mut self, save: JoinHandle<()>) {
        self.prune();
        self.running.push(save);
    }

    /// How many saves have not ended — what the quit gate reads beside
    /// the sessions' writes.
    pub fn pending(&mut self) -> usize {
        self.prune();
        self.running.len()
    }

    /// Everything still out, for the shutdown to join. Nothing is held
    /// afterwards: the joiner owns them.
    pub fn take_all(&mut self) -> Vec<JoinHandle<()>> {
        self.prune();
        std::mem::take(&mut self.running)
    }

    fn prune(&mut self) {
        self.running.retain(|save| !save.is_finished());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A save runs on no stock budget — the local write lane's rule —
    /// while the application's reads keep theirs.
    #[test]
    fn a_save_runs_without_the_stock_budget_the_reads_keep() {
        let reads = GitExecutor::new();
        assert!(reads.stock_timeout().is_some());
        assert_eq!(executor_for(&reads).stock_timeout(), None);
    }

    /// A save is counted from the ask to the end, whoever is still
    /// around to read the answer: what the quit gate waits on.
    #[tokio::test]
    async fn a_save_is_pending_until_it_has_ended_and_its_answer_is_kept() {
        let mut saves = Saves::default();
        let (release, held) = tokio::sync::oneshot::channel::<()>();
        // The screen's end of the feed, and the save's own: the screen
        // can drop its end and the save still has somewhere to answer.
        let feed = std::sync::Arc::new(crate::hub::Feed::<&'static str>::default());
        let screen = std::sync::Arc::clone(&feed);
        let screen_end = std::sync::Arc::clone(&feed);
        saves.hold(tokio::spawn(async move {
            if held.await.is_err() {
                return;
            }
            feed.push("written");
        }));
        assert_eq!(saves.pending(), 1, "the save is out");

        // The screen goes away: nothing of the save goes with it.
        drop(screen);
        assert_eq!(saves.pending(), 1, "still out");

        release.send(()).expect("the save is waiting on this");
        for save in saves.take_all() {
            // waits(ceiling): a save that never ends is a named failure here
            tokio::time::timeout(std::time::Duration::from_secs(900), save)
                .await
                .expect("the save ended within the ceiling")
                .expect("the save ended cleanly");
        }
        assert_eq!(saves.pending(), 0, "and once ended, nothing is held");
        assert_eq!(
            screen_end.drain(),
            vec!["written"],
            "the answer is where the screen would have read it"
        );
    }
}
