import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Modal dialog shell shared by every app dialog: centered over the window, escape to close, elevated card face, no
// built-in header/footer chrome. When open, a Dialog is parented into the window overlay, so `parent` here is the full
// window.
Dialog {
    /// The words this dialog hands over, for the hand that takes them from the air around them
    /// (`AppCardFace.textContent`). **Null unless a dialog asks**, which is what keeps the rule off the ones that are
    /// forms: there a press belongs to a field, a chooser or a button, and every gap between them is somewhere the
    /// next press is aimed. A dialog that is a block of words instead — a failure the reader has to be able to copy
    /// out — sets it to its own content and every gap in it becomes a place a selection can start
    /// (規約 §右のペインの字は掴める).
    property alias textContent: dialogFace.textContent

    anchors.centerIn: parent
    width: Math.min(640, (parent ? parent.width : 640) - 2 * Theme.spaceXxl)
    modal: true
    closePolicy: Popup.CloseOnEscape
    focus: true
    padding: Theme.spaceXxl
    header: null
    footer: null
    background: AppCardFace {
        id: dialogFace
    }
}
