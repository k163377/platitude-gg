import QtQuick
import platitude.ui

// Change-kind icon (pen = edit, + / − = add / delete, → = rename, stacked squares = copy, ! = conflict). All of these
// are kinds, not severities, so they come from the status tokens rather than from warning / danger -- even where the
// value is the same amber or red (デザイン規約 §git / diff 専用).
NavIcon {
    id: changeIcon
    property string change: ""
    readonly property string letter: change.length > 0 ? change[0] : ""
    readonly property bool conflict: change.length === 2
    kind: conflict ? "bang"
          : letter === "A" ? "plus"
          : letter === "D" ? "minus"
          : letter === "R" ? "arrow"
          : letter === "C" ? "copyicon"
          : letter === "?" ? "plus"
          : "pen"
    tint: conflict ? Theme.statusConflict
          : letter === "A" ? Theme.diffAddedFg
          : letter === "D" ? Theme.diffRemovedFg
          : letter === "R" ? Theme.textLink
          : letter === "C" ? Theme.textSecondary
          : letter === "?" ? Theme.diffAddedFg
          : Theme.statusUnstaged
}
