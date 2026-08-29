//! Everything QML sees of one repository's own git configuration.
//!
//! One `#[qobject]` block, and it cannot be split further — QMetaInfo is
//! built per file (app-ui.md).

use super::*;

#[qobject(ConvertToCamelCase, NoQmlElement)]
impl RepoConfigModel {
    // The repository being shown, as the strip spells it. Written by
    // `look()` rather than assigned: the read that follows is what makes
    // the rest of these mean anything.
    qproperty!("repoPath", Member = repo_path, Notify = changed);
    // "idle" | "reading" | "ready" | "error". The screen waits for
    // `"ready"` before it believes an empty box means "not set here" — a
    // box that has not been filled in yet looks exactly the same
    // (規約 app-ui.md §「まだ答えが無い」と値 0 / false を分ける).
    qproperty!("state", Member = state, Notify = changed);
    // What this repository's own file sets, empty for a key it leaves
    // alone — which is the same thing an empty box asks for.
    qproperty!("localName", Member = local_name, Notify = changed);
    qproperty!("localEmail", Member = local_email, Notify = changed);
    // What git would put on a commit made here right now, override and
    // all. The line under the boxes says it: an empty box cannot say what
    // it falls back to, because that value lives in a file this screen is
    // not showing.
    qproperty!("effectiveName", Member = effective_name, Notify = changed);
    qproperty!("effectiveEmail", Member = effective_email, Notify = changed);
    // A save is out.
    qproperty!("writeBusy", Member = write_busy, Notify = changed);
    // A save finished without both halves landing — and, with it, that a
    // save has been tried at all, which is what keeps the two marks below
    // out of a screen that has only been read.
    qproperty!("writeUnsaved", Member = write_unsaved, Notify = changed);
    // Which half git now reports as what was asked for. The pair is not
    // atomic (core.md), so a half-written override has to show as one
    // rather than pass for a finished pair.
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
    // git's own words from whichever of the read and the write last had
    // something to say; empty when neither did.
    qproperty!("error", Member = error, Notify = changed);

    #[qsignal]
    pub(super) fn changed(&mut self);

    /// Shows one repository. The only way in — everything else here
    /// answers about whatever this last named.
    #[qslot]
    fn look(&mut self, path: String) {
        self.read_repo(path)
    }

    /// Writes the pair into the repository on screen. An empty box asks
    /// for that key to be taken out, which is how an override is given
    /// back rather than replaced.
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
