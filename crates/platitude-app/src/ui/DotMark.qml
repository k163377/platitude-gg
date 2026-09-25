import QtQuick
import platitude.ui

/// The separator between two short facts on one line, drawn: `·` is East Asian Ambiguous, so the CJK families this app
/// names hold it in a full-width cell whose leftover the row cannot control
/// (規約 §余白「印が自分で持っている余白は、隣の詰めに数える」).
Item {
    id: mark
    property color tint: Theme.textMuted
    implicitWidth: dot.width
    implicitHeight: Theme.iconXs
    Rectangle {
        id: dot
        anchors.centerIn: parent
        /// Twice the hairline the drawn flag dash takes (規約 §git 用語のコード表記): a round mark of one pixel would
        /// disappear beside 12px text.
        width: Theme.borderWidth * 2
        height: dot.width
        radius: dot.width / 2
        color: mark.tint
    }
}
