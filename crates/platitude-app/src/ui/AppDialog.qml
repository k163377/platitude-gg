import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// Modal dialog shell shared by every app dialog: centered over the
// window, escape to close, elevated card face, no built-in
// header/footer chrome. When open, a Dialog is parented into the
// window overlay, so `parent` here is the full window.
Dialog {
    anchors.centerIn: parent
    width: Math.min(640, (parent ? parent.width : 640) - 2 * Theme.spaceXxl)
    modal: true
    closePolicy: Popup.CloseOnEscape
    focus: true
    padding: Theme.spaceXxl
    header: null
    footer: null
    background: AppCardFace {}
}
