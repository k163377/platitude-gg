//! Everything QML sees of one level's line-ending setting.
//!
//! One `#[qobject]` block, and it cannot be split further — QMetaInfo is
//! built per file (app-ui.md).

use super::*;

#[qobject(ConvertToCamelCase, NoQmlElement)]
impl LineEndingsModel {
    // Which of git's files this stands at — `"global"` or `"local"`.
    // Named by the screen before it looks at anything, because it is what
    // both the read and the write mean.
    qproperty!("scope", Member = scope, Notify = changed);
    // The work tree the reads and the write run in; empty is the
    // application's own directory. Written by `look()` rather than
    // assigned: the read that follows is what makes the rest mean
    // anything.
    qproperty!("repoPath", Member = repo_path, Notify = changed);
    // "idle" | "reading" | "ready" | "error". The screen waits for
    // `"ready"` before it believes the empty row means "this level sets
    // nothing" — a field that has not been filled in yet looks exactly the
    // same (規約 app-ui.md §「まだ答えが無い」と値 0 / false を分ける).
    qproperty!("state", Member = state, Notify = changed);
    // What that file sets, in git's own spelling — `"true"` / `"input"` /
    // `"false"`, empty for a level that sets nothing. The words a reader
    // sees are the screen's (`LineEndingField`), because words live in
    // `qsTr()`.
    qproperty!("held", Member = held, Notify = changed);
    // What git would use in the repository on screen right now, in the
    // same spelling. Empty where nobody asked. The line under the field
    // says it: an empty row cannot say what it falls back to, because
    // that value lives in a file this screen is not showing.
    qproperty!("effective", Member = effective, Notify = changed);
    // A pick is out.
    qproperty!("busy", Member = busy, Notify = changed);
    // git's own words from whichever of the read and the write last had
    // something to say; empty when neither did.
    qproperty!("error", Member = error, Notify = changed);

    #[qsignal]
    pub(super) fn changed(&mut self);

    /// A write landed — whether or not git took it.
    ///
    /// Apart from `changed` because one screen holds two of these, and
    /// what a write at one level moves at the other is only the sentence
    /// under the chooser: the value the repository *inherits* is the one
    /// just written above it. Firing on every `changed` would put a
    /// `git config` behind the read that fills the screen.
    #[qsignal]
    pub(super) fn wrote(&mut self);

    /// Looks at one repository. The only way in for the repository level;
    /// the global one is asked with an empty path, which is the
    /// application's own directory and therefore nobody's repository.
    #[qslot]
    fn look(&mut self, path: String) {
        self.read_at(path)
    }

    /// Writes the picked value into this level, where **an empty value
    /// asks for the key to be taken out** — which is how an override is
    /// given back rather than replaced.
    #[qslot]
    fn save(&mut self, value: String) {
        self.write_value(value)
    }

    /// Asks git again about the level already on screen, without emptying
    /// what it is showing. For the chapter whose *inherited* value another
    /// chapter has just written.
    #[qslot]
    fn reread(&mut self) {
        self.refresh()
    }

    /// The only slot an invoker ever calls (`models::mod`).
    #[qslot]
    fn drain(&mut self) {
        self.take_feed()
    }
}
