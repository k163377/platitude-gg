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
    function conflict(change, ours, theirs) {
        const us = ours !== "" ? ours : qsTr("this branch")
        const them = theirs !== "" ? theirs : qsTr("the incoming side")
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
