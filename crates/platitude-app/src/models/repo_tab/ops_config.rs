//! Wiring a tab to its feed, and the settings a tab holds.

use super::*;

impl RepoTab {
    pub(super) fn restart_auto_fetch(&mut self) {
        self.auto_fetch_suspended = false;
        self.fetch_failures = 0;
        self.fetch_log_raised = false;
        // Only the line fetch wrote: a background read's news is not
        // this button's to take down.
        if self.last_error_from_fetch {
            self.last_error = String::new();
            self.last_error_from_fetch = false;
        }
        self.with_session(|s| s.resume_auto_fetch());
        self.ask_session(|s| s.fetch(None));
        self.changed();
    }

    pub(super) fn remote_name_at(&self, index: i32) -> String {
        usize::try_from(index)
            .ok()
            .and_then(|i| self.remotes.get(i))
            .cloned()
            .unwrap_or_default()
    }

    pub(super) fn local_name_of(&self, remote_ref: String) -> String {
        let mut best: Option<&str> = None;
        for remote in &self.remotes {
            let Some(rest) = remote_ref.strip_prefix(&format!("{remote}/")) else {
                continue;
            };
            if !rest.is_empty() && best.is_none_or(|found| rest.len() < found.len()) {
                best = Some(rest);
            }
        }
        best.unwrap_or(remote_ref.as_str()).to_string()
    }

    pub(super) fn attach_feed(&mut self, tab_id: i32) {
        self.tab_id = tab_id;
        self.state = "loading".into();
        // Before the notify, so the first frame already leaves out what a
        // delete is standing in for (`ops_delete::read_stand_in`).
        self.read_stand_in();
        self.changed();
        let invoker = self.get_qml_method_invoker();
        self.feed = crate::hub::attach_feed(tab_id, |f| &f.tab, invoker);
    }

    pub(super) fn write_identity(&mut self, name: String, email: String, global: bool) {
        let scope = if global {
            platitude_core::identity::ConfigScope::Global
        } else {
            platitude_core::identity::ConfigScope::Local
        };
        self.ask_session(|s| s.set_identity(name.clone(), email.clone(), scope));
    }

    pub(super) fn write_tags_shown(&mut self, shown: bool) {
        if self.tags_shown == shown {
            return;
        }
        self.tags_shown = shown;
        self.changed();
        self.with_session(|s| s.set_include_tags(shown));
    }
}
