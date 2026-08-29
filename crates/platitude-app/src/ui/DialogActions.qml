import QtQuick
import QtQuick.Layouts
import platitude.ui

// The row a dialog ends on: the way out on the left, the thing the dialog
// is for on the right, both against the right edge.
//
// The one that acts is named by a frame, and the frame is the plain one:
// the accent is kept for the two buttons that write a commit, so a dialog
// that only ends itself does not borrow the colour they carry
// (2026-08-29 ユーザー報告). What says it cannot be pressed is the word,
// which takes the disabled step (`ActionButton.fg`). A dialog that has
// nothing to undo offers no way out and says so by leaving `cancelText`
// empty rather than by drawing a button that promises to put something
// back.
RowLayout {
    id: actions

    /// The word on the way out; empty draws no button at all.
    property string cancelText: ""
    /// The word on the button that does the thing.
    property string acceptText: ""
    /// The mark beside that word — empty for a button that is only a
    /// word (`NavIcon.kind`).
    property string acceptKind: ""
    property bool acceptEnabled: true
    /// git is out on the network for what this button asked for: the ring
    /// turns in the mark's own seat, the word stays and steps down, and
    /// nothing here answers a press (`ActionButton.busy` —
    /// デザイン規約 §進行中・長押しの定数). The way out beside it stays live:
    /// it is what stops the call.
    property alias busy: accept.busy

    /// The acting button itself, for a dialog that opens with the focus
    /// already on it.
    readonly property alias acceptButton: accept

    signal cancelled()
    signal accepted()

    Layout.alignment: Qt.AlignRight
    spacing: Theme.spaceSm

    ActionButton {
        visible: actions.cancelText !== ""
        implicitHeight: Theme.controlHeight
        activeFocusOnTab: true
        text: actions.cancelText
        onActivated: actions.cancelled()
    }
    ActionButton {
        id: accept
        implicitHeight: Theme.controlHeight
        kind: actions.acceptKind
        besideWord: actions.acceptKind !== ""
        frameColor: Theme.borderDefault
        activeFocusOnTab: true
        text: actions.acceptText
        enabled: actions.acceptEnabled
        onActivated: actions.accepted()
    }
}
