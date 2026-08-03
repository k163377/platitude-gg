import QtQuick
import platitude.ui

// Change-kind icon (pen = edit, + / − = add / delete, → = rename,
// stacked squares = copy, ! = conflict). Edits are amber by request;
// adds/deletes reuse the diff colors.
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
    tint: conflict ? Theme.danger
          : letter === "A" ? Theme.diffAddedFg
          : letter === "D" ? Theme.diffRemovedFg
          : letter === "R" ? Theme.textLink
          : letter === "C" ? Theme.textSecondary
          : letter === "?" ? Theme.diffAddedFg
          : Theme.warning
}
