import QtQuick
import platitude.ui

/// The separator between two short facts on one line, drawn rather than typed.
///
/// `·` is East Asian Ambiguous, so the CJK families this app names hold it in a full-width cell: what reads as the
/// space either side of the dot is that cell's leftover, not a token of ours — the row cannot spend `spaceXs` there
/// because the glyph has already spent whatever its family decided (規約 §余白「印が自分で持っている余白は、隣の詰めに数える」). Drawn, the seat
/// is the ink and the gap is the spacing the row was already going to pay.
Item {
    id: mark
    property color tint: Theme.textMuted
    implicitWidth: dot.width
    implicitHeight: Theme.iconXs
    Rectangle {
        id: dot
        anchors.centerIn: parent
        /// Twice the hairline: a rule of `borderWidth` is what the drawn flag dash takes, and a round mark of that one
        /// pixel would disappear beside 12px text (規約 §git 用語のコード表記).
        width: Theme.borderWidth * 2
        height: dot.width
        radius: dot.width / 2
        color: mark.tint
    }
}
