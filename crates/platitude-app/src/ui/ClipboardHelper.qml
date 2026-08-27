import QtQuick

// Clipboard access for copy actions (QML has no direct clipboard API): an invisible TextEdit selects-all and copies.
Item {
    visible: false
    /// What went out last. Not kept for its own sake — it is the scratch pad's own text, which has to hold the value
    /// for the copy to happen at all — but named so a headless run can read back what reached the clipboard, which
    /// the clipboard itself will not tell it (verify-ui).
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
