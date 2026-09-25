//! Everything QML sees of one repository's line-ending setting.
//!
//! One `#[qobject]` block, and it cannot be split further
//! (structure.md「1 型 1 ファイルから動かせない」).

use super::*;

#[qobject(ConvertToCamelCase, NoQmlElement)]
impl LineEndingsModel {
    qproperty!("repoPath", Member = repo_path, Notify = changed);
    // The screen waits for `"ready"` before it reads an empty `held` as
    // "sets nothing" — a field not filled in yet looks the same
    // (rules-refs/app-ui.md「『まだ答えが無い』と値 0 / false を分ける」).
    qproperty!("state", Member = state, Notify = changed);
    // `"true"` / `"input"` / `"false"`, or empty; the words a reader sees
    // are the screen's (`LineEndingField`).
    qproperty!("held", Member = held, Notify = changed);
    // The same spelling. The line under the field says it: an empty row
    // cannot show what it falls back to.
    qproperty!("effective", Member = effective, Notify = changed);
    qproperty!("busy", Member = busy, Notify = changed);
    qproperty!("error", Member = error, Notify = changed);

    #[qsignal]
    pub(super) fn changed(&mut self);

    /// Looks at one repository — the only way in.
    #[qslot]
    fn look(&mut self, path: String) {
        self.read_at(path)
    }

    /// Writes the picked value into that repository's own file; an empty
    /// value takes the key out, giving the override back.
    #[qslot]
    fn save(&mut self, value: String) {
        self.write_value(value)
    }

    #[qslot]
    fn drain(&mut self) {
        self.take_feed()
    }
}
