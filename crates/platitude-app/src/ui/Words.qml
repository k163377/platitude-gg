pragma Singleton

import QtQuick

// Wording that more than one place has to say the same way. A second copy of a sentence is a second answer, and the two
// drift.
QtObject {
    /// The application's name, as the window title and both empty states write it.
    readonly property string appName: qsTr("Platitude GG")

    /// A held control's answer to a screen reader, wherever a HoldDriver drives one.
    readonly property string holdToActivate: qsTr("Hold to activate")

    /// Opening a repository, as the graph's empty state, the tab strip's `+` menu and its accessible name offer it.
    readonly property string openRepository: qsTr("Open repository…")

    /// Adding a remote, as the sidebar header, the collapsed rail and the publish flow offer it.
    readonly property string addRemote: qsTr("Add remote…")

    /// The window band's three badge words (BandStateGroup measures and draws them, BandStateCard titles them);
    /// the WIP pane's conflicts bucket header shares the first.
    readonly property string badgeConflicts: qsTr("CONFLICTS")
    readonly property string badgeSetIdentity: qsTr("SET IDENTITY")
    readonly property string badgeOldGit: qsTr("OLD GIT")

    /// The one way a commit's moment is written down: the rows carry epoch seconds, and the display side makes the
    /// `yyyy-MM-dd HH:mm` out of them (デザイン規約 — 行が持つのは epoch 秒). The commands panel's `HH:mm:ss` clock
    /// is a different thing and stays its own.
    function stamp(epochSeconds) {
        return Qt.formatDateTime(new Date(epochSeconds * 1000), "yyyy-MM-dd HH:mm")
    }

    /// What to call a side git left nothing to name. Reached from a cherry-pick of a commit no branch can see, among
    /// others. The names swap over during a rebase, so they are handed in from `WorkTreeModel` rather than worked out
    /// here.
    function ourSide(name) {
        return name !== "" ? name : qsTr("this branch")
    }
    function theirSide(name) {
        return name !== "" ? name : qsTr("the incoming side")
    }

    /// Why a folder would not open, from the kind core answered with (`plain` / `bare` / `other`). Said in two places —
    /// the dialog the picker's answer raises, and the screen a tab that could not open shows — so the six they make
    /// between them are three sentences.
    ///
    /// `other` is git having trouble of its own rather than an answer about the folder, and its line says only that
    /// much: what happened is git's to say, and the screen quotes it underneath.
    function openFailure(kind) {
        switch (kind) {
        case "bare": return qsTr("A bare repository has nothing to show")
        case "other": return qsTr("Could not open this folder")
        default: return qsTr("Not a git repository")
        }
    }

    /// What the two sides each did to a conflicted file, from the two stage letters git reports (デザイン規約 §conflict の種別).
    /// Shown by the diff pane on the conflicts git prints no patch for — the one place the sentence appears; the
    /// headless `conflict_kind` report reads it through `NavItemDelegate.conflictWords`.
    function conflict(change, ours, theirs) {
        const us = Words.ourSide(ours)
        const them = Words.theirSide(theirs)
        switch (change) {
        case "UU": return qsTr("Both changed it")
        case "AA": return qsTr("Both added it")
        case "DD": return qsTr("Both deleted it")
        case "DU": return qsTr("Deleted on %1, changed on %2").arg(us).arg(them)
        case "UD": return qsTr("Changed on %1, deleted on %2").arg(us).arg(them)
        case "AU": return qsTr("Added on %1 only").arg(us)
        case "UA": return qsTr("Added on %1 only").arg(them)
        default: return qsTr("Conflicted")
        }
    }

    /// What a change did to a file's line endings, from the pieces `DiffModel` took the notice apart into. `""` when
    /// there is nothing to say, which is most of the time.
    ///
    /// Every branch is a **whole sentence**: the two estimated cases can only claim as far as the sample reached, so
    /// the range is part of what is being said rather than a clause bolted on. Stitching fragments would also leave a
    /// translator with half a sentence and no way to reorder it.
    ///
    /// `LF` and `CRLF` are written plainly, not as code chips — the chip shape is lowercase monospace and an all-caps
    /// abbreviation does not sit in it (デザイン規約 §git 用語のコード表記).
    function lineEndings(kind, from, to, lines, scope, ext) {
        switch (kind) {
        case "flipped":
            return qsTr("Line endings change · %1 → %2").arg(from).arg(to)
        case "mixed":
            // One line is not "1 added lines", and a translator cannot fix that from the outside.
            return lines === 1
                ? qsTr("Mixed line endings · 1 added line uses %1, this file uses %2").arg(from).arg(to)
                : qsTr("Mixed line endings · %1 added lines use %2, this file uses %3").arg(lines).arg(from).arg(to)
        case "new":
            switch (scope) {
            case "here":
                return qsTr("New file uses %1 · other .%2 files here look like %3").arg(from).arg(ext).arg(to)
            case "ext":
                return qsTr("New file uses %1 · other .%2 files look like %3").arg(from).arg(ext).arg(to)
            default:
                return qsTr("New file uses %1 · other files in this repo look like %2").arg(from).arg(to)
            }
        case "first":
            switch (scope) {
            case "here":
                return qsTr("First line ending in this file · %1 · other .%2 files here look like %3")
                    .arg(from).arg(ext).arg(to)
            case "ext":
                return qsTr("First line ending in this file · %1 · other .%2 files look like %3")
                    .arg(from).arg(ext).arg(to)
            default:
                return qsTr("First line ending in this file · %1 · other files in this repo look like %2")
                    .arg(from).arg(to)
            }
        default:
            return ""
        }
    }
}
