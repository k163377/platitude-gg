//! Wiring one instance to the feed of the section it is the list of.

use super::*;

impl NavSectionModel {
    pub(super) fn attach_section_feed(&mut self, tab_id: i32, section: String) {
        self.tab_id = tab_id;
        self.section = section;
        let Some(Some(feeds)) = Hub::with(|hub| hub.feeds(tab_id)) else {
            return;
        };
        let consumer = crate::hub::Wake::drain(self, Self::drain);
        match self.section.as_str() {
            "branches" => self.refs_feed = Some(attached(&feeds.refs_branches, consumer)),
            "remotes" => self.refs_feed = Some(attached(&feeds.refs_remotes, consumer)),
            "tags" => self.refs_feed = Some(attached(&feeds.refs_tags, consumer)),
            "stashes" => self.stash_feed = Some(attached(&feeds.stash, consumer)),
            "worktrees" => self.worktrees_feed = Some(attached(&feeds.worktrees, consumer)),
            other => tracing::warn!(section = other, "unknown sidebar section"),
        }
    }

    /// One instance and one feed per bucket run: a `Feed` has one consumer and
    /// is drained once. Only `arrange` narrows to the run, and every run's
    /// source is the worktree's whole status, so what asks about the tree
    /// (`holdsPath` / `besidePath` / `bucketOf` / `origOf` / `changeOf`)
    /// answers the same from any instance (the page keeps asking the unstaged
    /// one, `page.wipUnstaged`), while what asks about rows on screen
    /// (`rowOfFileIn` / `stepFile` / `edgeFile` / `shownRows`) answers for
    /// its own bucket only.
    pub(super) fn attach_worktree_feed(&mut self, tab_id: i32, run: String) {
        self.tab_id = tab_id;
        self.section = "files".to_string();
        self.run = run;
        let Some(Some(feeds)) = Hub::with(|hub| hub.feeds(tab_id)) else {
            return;
        };
        let consumer = crate::hub::Wake::drain(self, Self::drain);
        let feed = match self.run.as_str() {
            "conflicts" => &feeds.status_nav_conflicts,
            "staged" => &feeds.status_nav_staged,
            "unstaged" => &feeds.status_nav_unstaged,
            other => {
                tracing::warn!(run = other, "unknown worktree bucket run");
                return;
            }
        };
        self.status_feed = Some(attached(feed, consumer));
    }

    /// The list that shows another worktree's changes
    /// ([`crate::hub::CarriedStatusMsg`]). One list where this window's
    /// tree has three ([`super::Bucket::Whole`]): the three split by the
    /// index, and nothing here moves that worktree's index. A separate instance
    /// all the same — both trees are fed while the pane shows either — on a
    /// feed of its own (`carried_nav`): fed through this tree's runs, every
    /// paced read of this tree would take the other worktree's rows away.
    pub(super) fn attach_carried_feed(&mut self, tab_id: i32) {
        self.tab_id = tab_id;
        self.section = "files".to_string();
        self.run = "whole".to_string();
        let Some(Some(feeds)) = Hub::with(|hub| hub.feeds(tab_id)) else {
            return;
        };
        let consumer = crate::hub::Wake::drain(self, Self::drain);
        self.carried_feed = Some(attached(&feeds.carried_nav, consumer));
    }
}
