import QtQuick
import platitude.ui

/// The separator between two short facts on one line, drawn: `·` is East Asian Ambiguous, so the CJK families this app
/// names hold it in a full-width cell whose leftover the row cannot control. Drawn in the character's place, its seat is
/// its ink and the row's spacing the gaps either side (デザイン規約 §アイコンの席 の字の代替).
Item {
    id: mark
    property color tint: Theme.textMuted
    /// Where the dot's middle stands, in this item's y: a sign beside words stands on the face's own `+`'s middle
    /// (デザイン規約 §アイコンの席 の字の代替, `WordFace.signMiddle`); the box's middle by default.
    property real middle: mark.height / 2
    implicitWidth: dot.width
    implicitHeight: Theme.iconXs
    Rectangle {
        id: dot
        anchors.horizontalCenter: parent.horizontalCenter
        y: Math.round(mark.middle - dot.height / 2)
        /// Twice the hairline the drawn flag dash takes (規約 §git 用語のコード表記): a round mark of one pixel would
        /// disappear beside 12px text.
        width: Theme.borderWidth * 2
        height: dot.width
        radius: dot.width / 2
        color: mark.tint
    }
}
