//! Fetching a repository that has no tab yet.
//!
//! Its own object: a clone runs outside every session, before there is
//! anything to open. `Main.qml` hands the landed folder to the tab strip,
//! so neither object knows the other.

use std::sync::Arc;

use qtbridge::{QObjectHolder, qobject};

use crate::hub::{CloneMsg, Feed, Hub};
use crate::urlpath::file_url_to_path;

use super::qml_register;

/// The clone in flight, and what became of the last one.
#[derive(Default)]
pub struct CloneModel {
    /// The background task's answers; attached on the first clone.
    feed: Arc<Feed<CloneMsg>>,
    attached: bool,
    cloning: bool,
    /// Held here, not in the dialog: the dialog is rebuilt with the
    /// window's overlay, and the token must outlive every rebuild.
    cancel: Option<tokio_util::sync::CancellationToken>,
}

#[qobject(ConvertToCamelCase, NoQmlElement)]
impl CloneModel {
    // Whether git is out fetching; the dialog's ring, fields and buttons
    // bind to it.
    qproperty!("cloning", Member = cloning, Notify = cloning_changed);

    #[qsignal]
    fn cloning_changed(&mut self);

    /// The clone landed in `path`. Its own signal: `cloning` falls for a
    /// refusal too.
    #[qsignal]
    fn clone_done(&mut self, path: String);

    /// git refused, in `message`'s words — shown in the dialog that asked,
    /// which is still standing (デザイン規約 §リポジトリを取り寄せる).
    #[qsignal]
    fn clone_failed(&mut self, message: String);

    /// Fetches `url` into a folder called `name` inside the folder
    /// `parent_url` names (a `file://` URL from the FolderDialog).
    ///
    /// Not checked here: a taken destination or a dead URL is git's to
    /// refuse, and the dialog stays open to show what git says.
    #[qslot]
    fn clone_repository(&mut self, url: String, parent_url: String, name: String) {
        let url = url.trim().to_string();
        let name = name.trim().to_string();
        // One at a time; the modal dialog refuses a second accept anyway.
        if url.is_empty() || name.is_empty() || self.cloning {
            return;
        }
        let parent = file_url_to_path(&parent_url);
        if parent.as_os_str().is_empty() {
            return;
        }
        if !self.attached {
            self.feed.attach(self.get_qml_method_invoker());
            self.attached = true;
        }
        let feed = Arc::clone(&self.feed);
        let into = parent.join(&name);
        tracing::info!(url = %url, into = %into.display(), "cloning");
        let Some(Some(cancel)) = Hub::with(|hub| hub.clone_repo(url, into, feed)) else {
            return;
        };
        self.cancel = Some(cancel);
        self.settle(true);
    }

    /// Stops the clone in flight, if there is one. What git left behind is
    /// left behind: this application does not delete folders.
    #[qslot]
    fn cancel_clone(&mut self) {
        let Some(cancel) = self.cancel.take() else {
            return;
        };
        cancel.cancel();
        // The answer may already be on its way (git exited 0 as the press
        // landed). Replace the feed rather than drain it: a late push then
        // wakes nobody, and a stopped clone cannot open a tab.
        self.feed.release();
        self.feed = Arc::new(Feed::default());
        self.attached = false;
        tracing::info!("clone cancelled");
        self.settle(false);
    }

    #[qslot]
    fn drain(&mut self) {
        for msg in self.feed.drain() {
            self.cancel = None;
            self.settle(false);
            match msg {
                CloneMsg::Done { path } => {
                    self.clone_done(path.to_string_lossy().into_owned());
                }
                CloneMsg::Failed { message } => {
                    tracing::info!(message = %message, "clone refused");
                    self.clone_failed(message);
                }
            }
        }
    }
}

impl CloneModel {
    fn settle(&mut self, cloning: bool) {
        if self.cloning != cloning {
            self.cloning = cloning;
            self.cloning_changed();
        }
    }
}

qml_register!(CloneModel, "CloneModel", singleton = false);
