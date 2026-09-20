import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Modal dialog shell shared by every app dialog: centered over the window, escape to close, elevated card face, the
// content its whole body. When open, a Dialog is parented into the window overlay, so `parent` here is the full
// window.
Dialog {
    id: dialog

    /// The dialog takes the whole window. For the two settings screens, which are read: a card sized to its own
    /// content grows a scrollbar as soon as one chapter does, and what is behind it is not what the reader is looking
    /// at anyway. A face that covers everything has no edge left to float above, so it gives up the corners and the
    /// frame as well.
    property bool fills: false

    /// The words this dialog hands over, for the hand that takes them from the air around them
    /// (`AppCardFace.textContent`). **Null unless a dialog asks**, which is what keeps the rule off the ones that are
    /// forms: there a press belongs to a field, a chooser or a button, and every gap between them is somewhere the
    /// next press is aimed. A dialog that is a block of words instead — a failure the reader has to be able to copy
    /// out — sets it to its own content and every gap in it becomes a place a selection can start
    /// (規約 §右のペインの字は掴める).
    property alias textContent: dialogFace.textContent

    /// The hand that hands the keyboard back, named so a run can say it is standing (`caretHand.stands`). An
    /// automation-only exposure, the same one `GraphPane.view` is (app-ui.md): whether a press *reaches* it is
    /// Qt's to answer and only a real pointer can ask (`tests/qml/tst_fieldrelease.qml`).
    readonly property alias caretHand: caretHand

    anchors.centerIn: parent
    // The card is set to the width a run of prose is read at (規約 §レイアウト初期値 — the per-screen tier's
    // `textWidth`, shared with the screen a repository would not open on and the settings screen's chapters).
    width: dialog.fills
           ? (parent ? parent.width : Theme.textWidth)
           : Math.min(Theme.textWidth, (parent ? parent.width : Theme.textWidth) - 2 * Theme.spaceXxl)
    height: dialog.fills ? (parent ? parent.height : 480) : implicitHeight
    modal: true
    closePolicy: Popup.CloseOnEscape
    focus: true
    padding: Theme.spaceXxl
    header: null
    footer: null
    background: AppCardFace {
        id: dialogFace
        radius: dialog.fills ? 0 : Theme.radiusMd
        border.width: dialog.fills ? 0 : Theme.borderWidth

        // A press that landed on this dialog but not on whatever is holding the caret hands the keyboard back, so
        // the selection a reader made goes out with the gesture they made next (規約 §右のペインの字は掴める
        // 「選択は窓に 1 つだけ」). **QML never drops a field's focus on its own**, and the window's own watcher
        // cannot answer for a dialog — a modal stands in the overlay above it.
        //
        // **On the face, behind everything the dialog draws.** What accepts a press above this either takes the
        // focus itself (a box, a tick, a button, a chooser) or is a reading surface's own hand, which puts the
        // caret where the press was; the only presses that reach here are the ones that would otherwise leave a
        // caret standing — a row that answers with a handler of its own, and the plain air
        // (measured, `tests/qml/tst_fieldrelease.qml`).
        FocusRelease {
            id: caretHand
            window: Window.window
            // The dialog's own content, so a key pressed after the caret was walked away from is still delivered
            // inside this screen.
            home: dialog.contentItem
        }
    }
}
