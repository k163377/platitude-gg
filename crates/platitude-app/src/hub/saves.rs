//! The configuration writes the application itself asks for — the
//! identity screens' — held from the ask until git has answered, by
//! something that outlives the screen that asked.
//!
//! A save is spawned against a path (`models::repo_config`, `AppBackend`),
//! outside any session: the settings screen offers every repository in the
//! strip, and only the tab in front has a session. Unheld, the window
//! could close with `git config` half way through its pair. So the hub
//! holds them (`Hub::spawn_save`), the quit gate counts them
//! (`Hub::writes_settled`) and the shutdown joins them (`Hub::shutdown`),
//! as a session's local write is.
//!
//! Holding is all this does: slots are the executor's
//! (`platitude_core::process::Slots`), and the answer travels on the feed
//! the save was given — unread and harmless if the screen has gone. A save
//! is never cancelled: half a pair is worse than either whole.

use platitude_core::GitExecutor;
use tokio::task::JoinHandle;

/// The application's handle with the stock time budget lifted, as for the
/// local write lane (`operation::Lane::Local`): a save killed at a budget
/// is the half-landed pair this module rules out.
pub(super) fn executor_for(base: &GitExecutor) -> GitExecutor {
    base.clone().without_stock_timeouts()
}

/// The saves still out; finished ones are pruned wherever this is read.
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

    /// Everything still out, handed to the shutdown to join.
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

    #[test]
    fn a_save_runs_without_the_stock_budget_the_reads_keep() {
        let reads = GitExecutor::new();
        assert!(reads.stock_timeout().is_some());
        assert_eq!(executor_for(&reads).stock_timeout(), None);
    }

    /// Counted until it ends whether or not the screen that asked is still
    /// around — what the quit gate waits on.
    #[tokio::test]
    async fn a_save_is_pending_until_it_has_ended_and_its_answer_is_kept() {
        let mut saves = Saves::default();
        let (release, held) = tokio::sync::oneshot::channel::<()>();
        // The screen's end of the feed, and the save's own.
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

        // The screen goes away.
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
