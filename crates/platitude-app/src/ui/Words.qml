pragma Singleton

import QtQuick

// Wording that more than one place has to say the same way. A second copy
// of a sentence is a second answer, and the two drift.
QtObject {
    /// What the two sides each did to a conflicted file, from the two
    /// stage letters git reports (デザイン規約 §conflict の種別). A side
    /// that has a name is called by it; one that has none falls back to
    /// where it stands — the names swap over during a rebase, so they are
    /// handed in from `WorkTreeModel` rather than worked out here.
    ///
    /// Said by the file row's icon (hover) and by the diff pane on the
    /// conflicts git prints no patch for.
    /// What to call a side git left nothing to name. Reached from a
    /// cherry-pick of a commit no branch can see, among others.
    function ourSide(name) {
        return name !== "" ? name : qsTr("this branch")
    }
    function theirSide(name) {
        return name !== "" ? name : qsTr("the incoming side")
    }

    /// Why a folder would not open, from the kind core answered with
    /// (`plain` / `bare` / `other`). Said in two places — the dialog the
    /// picker's answer raises, and the screen a tab that could not open
    /// shows — so the six they make between them are three sentences.
    ///
    /// `other` is git having trouble of its own rather than an answer
    /// about the folder, and its line says only that much: what happened
    /// is git's to say, and the screen quotes it underneath.
    function openFailure(kind) {
        switch (kind) {
        case "bare": return qsTr("A bare repository has nothing to show")
        case "other": return qsTr("Could not open this folder")
        default: return qsTr("Not a git repository")
        }
    }

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
}
