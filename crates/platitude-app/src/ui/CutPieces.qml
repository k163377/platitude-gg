import QtQuick
import platitude.ui
import platitude.ui.face

// A value cut by an elide or `WordFace.cutToFit`, drawn in pieces: the head and the tail in the words' own font, the
// cut mark between them (`CutMark`) on their baseline, each piece closed up to the last — the leftover of the room
// after them, not in the cut (a lone field or a chip: no column edge to line up, unlike `CutName`). `text` is the value
// as cut, holding the cut's `…` (`WordFace.cutPoint`), so a caller and a run read the cut as a string; a value cut
// nowhere is drawn whole.
Item {
    id: pieces

    /// The value in full, and as cut.
    property string whole: ""
    property string text: ""
    property color color: Theme.textPrimary
    /// The words' font: the pieces and the mark are drawn in it.
    property font font
    /// The mark's ruler and x (`CutMark.ruler`), for a holder that ends a ground on the mark
    /// (`WordFace.groundEndAtMark`).
    readonly property alias markRuler: mark.ruler
    readonly property alias markX: mark.x
    /// Where the cut's `…` stands in `text`; -1 where nothing was cut — a value's own `…` stays a character.
    readonly property int at: pieces.text === pieces.whole ? -1 : WordFace.cutPoint(pieces.whole, pieces.text)

    implicitWidth: pieces.at < 0 ? headLabel.implicitWidth
                                 : headLabel.implicitWidth + mark.implicitWidth + tailLabel.implicitWidth
    implicitHeight: headLabel.implicitHeight
    baselineOffset: headLabel.y + headLabel.baselineOffset

    PlainTextLabel {
        id: headLabel
        height: pieces.height
        verticalAlignment: Text.AlignVCenter
        text: pieces.at < 0 ? pieces.text : pieces.text.substring(0, pieces.at)
        color: pieces.color
        font: pieces.font
    }
    CutMark {
        id: mark
        visible: pieces.at >= 0
        x: headLabel.implicitWidth
        y: headLabel.y + headLabel.baselineOffset - mark.baselineOffset
        font: pieces.font
        color: pieces.color
    }
    PlainTextLabel {
        id: tailLabel
        visible: pieces.at >= 0 && pieces.at < pieces.text.length - 1
        x: mark.x + mark.implicitWidth
        height: pieces.height
        verticalAlignment: Text.AlignVCenter
        text: pieces.at < 0 ? "" : pieces.text.substring(pieces.at + 1)
        color: pieces.color
        font: pieces.font
    }
}
