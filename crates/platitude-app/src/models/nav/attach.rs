//! Wiring one instance to the feed of the section it is the list of.

use super::*;

impl NavSectionModel {
    pub(super) fn attach_section_feed(&mut self, tab_id: i32, section: String) {
        self.tab_id = tab_id;
        self.section = section;
        let Some(Some(feeds)) = Hub::with(|hub| hub.feeds(tab_id)) else {
            return;
        };
        let invoker = self.get_qml_method_invoker();
        match self.section.as_str() {
            "branches" => self.refs_feed = Some(attached(&feeds.refs_branches, invoker)),
            "remotes" => self.refs_feed = Some(attached(&feeds.refs_remotes, invoker)),
            "tags" => self.refs_feed = Some(attached(&feeds.refs_tags, invoker)),
            "stashes" => self.stash_feed = Some(attached(&feeds.stash, invoker)),
            "worktrees" => self.worktrees_feed = Some(attached(&feeds.worktrees, invoker)),
            other => tracing::warn!(section = other, "unknown sidebar section"),
        }
    }

    pub(super) fn attach_worktree_feed(&mut self, tab_id: i32, run: String) {
        self.tab_id = tab_id;
        self.section = "worktree".to_string();
        self.run = run;
        let Some(Some(feeds)) = Hub::with(|hub| hub.feeds(tab_id)) else {
            return;
        };
        let invoker = self.get_qml_method_invoker();
        let feed = match self.run.as_str() {
            "conflicts" => &feeds.status_nav_conflicts,
            "staged" => &feeds.status_nav_staged,
            "unstaged" => &feeds.status_nav_unstaged,
            other => {
                tracing::warn!(run = other, "unknown worktree bucket run");
                return;
            }
        };
        self.status_feed = Some(attached(feed, invoker));
    }

    /// The list that shows another working copy's changes
    /// ([`crate::hub::CarriedStatusMsg`]).
    ///
    /// **One list, where this window's own tree is three**
    /// ([`super::Bucket::Whole`]): the split the three stand for is the
    /// index's, and nothing here can move that copy's index. A separate
    /// instance from them all the same — the two stand for different
    /// trees and both are fed while the pane is showing either.
    pub(super) fn attach_carried_feed(&mut self, tab_id: i32) {
        self.tab_id = tab_id;
        self.section = "worktree".to_string();
        self.run = "whole".to_string();
        let Some(Some(feeds)) = Hub::with(|hub| hub.feeds(tab_id)) else {
            return;
        };
        let invoker = self.get_qml_method_invoker();
        self.carried_feed = Some(attached(&feeds.carried_nav, invoker));
    }
}
