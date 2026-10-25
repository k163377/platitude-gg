import QtQuick

// The hand under a framed text box, over the inset band its view leaves between the frame and the words. The whole
// frame takes a caret (デザイン規約 §コミットメッセージの 2 つの枠), but a `TextArea` in a `ScrollView` is sized to the
// inset view, so without this the band takes no press (`tst_messageband`).
//
// It lies under the view, so the words, the scroll bar and the description box's grip keep every press they had
// (規約 §右のペインの字は掴める). A plain `MouseArea` (rules-refs/app-ui.md「手の実装は素の `MouseArea`」).
Item {
    id: band

    /// The text this band hands its press to — anything a `TextEdit` answers.
    required property Item field

    /// The character nearest a point of this band, clamped to the text — the whole of what the band is for. The map
    /// carries the view's scroll, so a press on a scrolled box lands on the line under the pointer.
    function caretAt(position) {
        const at = band.field.mapFromItem(band, position)
        return band.field.positionAt(at.x, at.y)
    }
    /// The two the hand below calls, at points in this item's coordinates. The anchor is taken at the press, as
    /// `SweepPad`'s is.
    function pressAt(position) {
        band.field.forceActiveFocus()
        band.field.cursorPosition = band.caretAt(position)
    }
    function moveAt(position) {
        band.field.moveCursorSelection(band.caretAt(position), TextEdit.SelectCharacters)
    }

    MouseArea {
        id: hand
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton
        // The commit block and the details pane are `Flickable`s, and a drag they steal takes the selection with it.
        preventStealing: true
        cursorShape: Qt.IBeamCursor
        onPressed: mouse => band.pressAt(Qt.point(mouse.x, mouse.y))
        onPositionChanged: mouse => { if (hand.pressed) band.moveAt(Qt.point(mouse.x, mouse.y)) }
    }
}
