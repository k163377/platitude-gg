import QtQuick
import QtQuick.Layouts
import platitude.ui

// The row a dialog ends on: the way out on the left, the thing the dialog
// is for on the right, both against the right edge.
//
// The accent is on the one that acts, and it is dropped while that one
// cannot be pressed — a lit button that does nothing is a lie
// (デザイン規約 §状態). A dialog that has nothing to undo offers no way
// out and says so by leaving `cancelText` empty rather than by drawing a
// button that promises to put something back.
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
    /// Whether the acting button wears the accent. Follows what it can
    /// do, which is right wherever the dialog has one thing to offer;
    /// a dialog whose accent depends on why it was opened writes it.
    property bool acceptAccented: actions.acceptEnabled

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
        frameColor: actions.acceptAccented ? Theme.accent : Theme.borderDefault
        activeFocusOnTab: true
        text: actions.acceptText
        enabled: actions.acceptEnabled
        onActivated: actions.accepted()
    }
}
