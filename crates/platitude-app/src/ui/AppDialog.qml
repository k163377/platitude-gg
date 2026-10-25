import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Modal shell for every app dialog. Open, a Dialog is parented into the window overlay, so `parent` is the window.
Dialog {
    id: dialog

    /// Take the whole window, without corners or frame (the settings screen, which as a card would scroll as soon as
    /// one chapter grew).
    property bool fills: false

    /// Set by a dialog that is a block of words (a failure to copy out) so a press in any gap starts a selection
    /// (`AppCardFace.textContent`); null on forms, where a gap is where the next press is aimed
    /// (規約 §右のペインの字は掴める).
    property alias textContent: dialogFace.textContent

    /// Automation only (rules-refs/app-ui.md「自動化フック専用の露出」): a run reads `caretHand.stands`; whether a press
    /// reaches it only a real pointer can ask (`tests/qml/tst_fieldrelease.qml`).
    readonly property alias caretHand: caretHand

    anchors.centerIn: parent
    // The width prose is read at (規約 §レイアウト初期値, `textWidth`).
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

        // On the face, under everything the dialog draws: a press nothing else took hands the keyboard back, so a
        // selection goes with the next gesture (規約 §右のペインの字は掴める「選択は窓に 1 つだけ」). The window's own
        // cannot reach a modal (rules-refs/app-ui.md「モーダルには窓の `FocusRelease` が届かない」).
        FocusRelease {
            id: caretHand
            window: Window.window
            // So a key pressed after the caret walked away still lands inside this screen.
            home: dialog.contentItem
        }
    }
}
