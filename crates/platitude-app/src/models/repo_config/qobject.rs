//! Everything QML sees of one repository's own git configuration.

use super::*;

#[qobject(ConvertToCamelCase, NoQmlElement)]
impl RepoConfigModel {
    qproperty!("repoPath", Member = repo_path, Notify = changed);
    // "idle" | "reading" | "ready" | "error". An empty box means "not set
    // here" only once `"ready"`
    // (rules-refs/app-ui.md「『まだ答えが無い』と値 0 / false を分ける」).
    qproperty!("state", Member = state, Notify = changed);
    // What this repository's own file sets, empty for a key it leaves
    // alone — which is the same thing an empty box asks for.
    qproperty!("localName", Member = local_name, Notify = changed);
    qproperty!("localEmail", Member = local_email, Notify = changed);
    // What git would put on a commit made here right now, override and
    // all — the line under the boxes, since an empty box cannot say what
    // it falls back to.
    qproperty!("effectiveName", Member = effective_name, Notify = changed);
    qproperty!("effectiveEmail", Member = effective_email, Notify = changed);
    // A save is out.
    qproperty!("writeBusy", Member = write_busy, Notify = changed);
    // A save finished without both halves landing; false until a save is
    // tried, which keeps the two marks below off a screen only read.
    qproperty!("writeUnsaved", Member = write_unsaved, Notify = changed);
    // Which half git now reports as what was asked for — the pair is not
    // atomic (rules-refs/core.md「identity の 2 連書きは原子化できない」).
    qproperty!(
        "writeNameSaved",
        Member = write_name_saved,
        Notify = changed
    );
    qproperty!(
        "writeEmailSaved",
        Member = write_email_saved,
        Notify = changed
    );
    // git's words from whichever of the read and the write last had
    // something to say.
    qproperty!("error", Member = error, Notify = changed);

    #[qsignal]
    pub(super) fn changed(&mut self);

    /// Shows one repository. The only way in — everything else here
    /// answers about whatever this last named.
    #[qslot]
    fn look(&mut self, path: String) {
        self.read_repo(path)
    }

    /// Writes the pair into the repository on screen. An empty box takes
    /// that key out, which is how an override is given back.
    #[qslot]
    fn save(&mut self, name: String, email: String) {
        self.write_repo(name, email)
    }

    /// The only slot an invoker ever calls (`models::mod`).
    #[qslot]
    fn drain(&mut self) {
        self.take_feed()
    }
}
