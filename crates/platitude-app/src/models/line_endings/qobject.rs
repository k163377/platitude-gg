//! Everything QML sees of one repository's line-ending setting.
//!
//! One `#[qobject]` block, and it cannot be split further — QMetaInfo is
//! built per file (app-ui.md).

use super::*;

#[qobject(ConvertToCamelCase, NoQmlElement)]
impl LineEndingsModel {
    // The work tree the reads and the write run in, which is the
    // repository whose own file they are about. Written by `look()` rather
    // than assigned: the read that follows is what makes the rest mean
    // anything.
    qproperty!("repoPath", Member = repo_path, Notify = changed);
    // "idle" | "reading" | "ready" | "error". The screen waits for
    // `"ready"` before it believes the empty row means "this repository
    // sets nothing of its own" — a field that has not been filled in yet
    // looks exactly the same
    // (規約 app-ui.md §「まだ答えが無い」と値 0 / false を分ける).
    qproperty!("state", Member = state, Notify = changed);
    // What that file sets, in git's own spelling — `"true"` / `"input"` /
    // `"false"`, empty for a repository that sets nothing of its own. The
    // words a reader sees are the screen's (`LineEndingField`), because
    // words live in `qsTr()`.
    qproperty!("held", Member = held, Notify = changed);
    // What git would use in the repository on screen right now, in the
    // same spelling. The line under the field says it: an empty row cannot
    // say what it falls back to, because that value lives in a file this
    // screen is not showing.
    qproperty!("effective", Member = effective, Notify = changed);
    // A pick is out.
    qproperty!("busy", Member = busy, Notify = changed);
    // git's own words from whichever of the read and the write last had
    // something to say; empty when neither did.
    qproperty!("error", Member = error, Notify = changed);

    #[qsignal]
    pub(super) fn changed(&mut self);

    /// Looks at one repository. The only way in — everything else here is
    /// about the repository this named.
    #[qslot]
    fn look(&mut self, path: String) {
        self.read_at(path)
    }

    /// Writes the picked value into that repository's own file, where **an
    /// empty value asks for the key to be taken out** — which is how an
    /// override is given back rather than replaced.
    #[qslot]
    fn save(&mut self, value: String) {
        self.write_value(value)
    }

    /// The only slot an invoker ever calls (`models::mod`).
    #[qslot]
    fn drain(&mut self) {
        self.take_feed()
    }
}
