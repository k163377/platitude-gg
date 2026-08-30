//! On-demand commit details.
use super::*;

impl RepoSession {
    /// Loads full details of one commit (metadata + changed files).
    pub fn load_details(self: &Arc<Self>, oid: Oid) {
        self.spawn_read("details", move |s, workdir, cancel| async move {
            match details::commit_details(&s.executor, &workdir, &oid, &cancel).await {
                Ok(details) => Ok(SessionEvent::DetailsLoaded { details }),
                // The pane asked by oid and is showing a spinner for it; a
                // failure that only reached the error surface would leave
                // that spinner up until the next selection.
                Err(e) => {
                    s.sink.event(SessionEvent::DetailsFailed { oid });
                    Err(e)
                }
            }
        });
    }
}
