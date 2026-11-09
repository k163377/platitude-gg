import QtQuick
import platitude.ui

// Change-kind icon. Which colour each kind wears, and why, is デザイン規約 §git / diff 専用.
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
          : letter === "A" ? Theme.plusFg
          : letter === "D" ? Theme.minusFg
          : letter === "R" ? Theme.textLink
          : letter === "C" ? Theme.textSecondary
          : letter === "?" ? Theme.plusFg
          : Theme.modifiedFg
}
