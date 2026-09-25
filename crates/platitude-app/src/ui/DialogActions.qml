import QtQuick
import QtQuick.Layouts
import platitude.ui

// The row a dialog ends on: the way out, then the thing the dialog is for, against the right edge. The acting button
// wears the plain frame — the accent is kept for the two buttons that write a commit — and says it cannot be pressed
// by its word (`ActionButton.fg`). A dialog with nothing to undo leaves `cancelText` empty: a way out would promise
// to put something back.
RowLayout {
    id: actions

    /// The word on the way out; empty draws no button at all.
    property string cancelText: ""
    property string acceptText: ""
    /// The mark beside that word — empty for a button that is only a word (`NavIcon.kind`).
    property string acceptKind: ""
    property bool acceptEnabled: true
    /// git is out on the network for this button: it spins and takes no press (`ActionButton.busy` — デザイン規約
    /// §進行中・長押しの定数). The way out stays live — it is what stops the call.
    property alias busy: accept.busy

    /// The acting button, for a dialog that opens with the focus on it.
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
