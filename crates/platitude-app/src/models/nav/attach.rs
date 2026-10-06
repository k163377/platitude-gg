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

    pub(super) fn attach_working_tree_feed(&mut self, tab_id: i32, run: String) {
        self.tab_id = tab_id;
        self.section = "files".to_string();
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
                tracing::warn!(run = other, "unknown working-tree bucket run");
                return;
            }
        };
        self.status_feed = Some(attached(feed, invoker));
    }

    /// The list that shows another worktree's changes
    /// ([`crate::hub::CarriedStatusMsg`]). One list where this window's
    /// tree has three ([`super::Bucket::Whole`]): the three split by the
    /// index, and nothing here moves that worktree's index. A separate instance
    /// all the same — both trees are fed while the pane shows either.
    pub(super) fn attach_carried_feed(&mut self, tab_id: i32) {
        self.tab_id = tab_id;
        self.section = "files".to_string();
        self.run = "whole".to_string();
        let Some(Some(feeds)) = Hub::with(|hub| hub.feeds(tab_id)) else {
            return;
        };
        let invoker = self.get_qml_method_invoker();
        self.carried_feed = Some(attached(&feeds.carried_nav, invoker));
    }
}
