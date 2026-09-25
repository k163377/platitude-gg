import QtQuick

// Clipboard access for copy actions (QML has no direct clipboard API): an invisible TextEdit selects-all and copies.
Item {
    visible: false
    /// What went out last, for a headless run to read back — the clipboard itself will not tell it (verify-ui).
    readonly property alias lastCopied: scratch.text
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
