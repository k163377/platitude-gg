import QtQuick

// Clipboard access for copy actions (QML has no direct clipboard API):
// an invisible TextEdit selects-all and copies.
Item {
    visible: false
    function copy(value) {
        scratch.text = value
        scratch.selectAll()
        scratch.copy()
    }
    TextEdit {
        id: scratch
        visible: false
    }
}
