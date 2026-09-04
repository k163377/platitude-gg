import QtQuick

// The hand under a framed text box, over the band its view leaves between the frame and the words.
//
// A box is a surface a click drops a caret on (デザイン規約 §コミットメッセージの 2 つの枠), and the box is the whole
// of what the frame encloses — but the view inside it is inset by `spaceXs` on all four sides, and a `TextArea` in a
// `ScrollView` is sized to that view. So the band around it took no press at all: a click within four pixels of the
// summary frame put no caret anywhere, which on a one-line box is a quarter of its height (qmltestrunner measured,
// `tst_messageband`).
//
// **It lies under the view rather than over it** (規約 §右のペインの字は掴める「手はその面の一番下に 1 枚だけ敷き、
// 上の誰も取らなかった press だけが届くようにする」) — so the words, the scroll bar and the description box's grip
// all keep every press they had, and that is structural rather than a claim to re-prove.
//
// **A plain `MouseArea`, not a handler** — the pair `SweepRoom` measured holds here too: a passive `PointHandler`
// loses half a drag and a `TapHandler` never fires over a selectable field.
Item {
    id: band

    /// The text this band hands its press to — anything a `TextEdit` answers.
    required property Item field

    /// The character nearest a point of this band, in the field's own coordinates. A point outside the text comes
    /// back clamped to the nearest position, which is the whole of what the band is for; the map carries the view's
    /// scroll with it, so a band press on a box that has been sent lands on the line under the pointer rather than
    /// on the line that would be there unscrolled.
    function caretAt(x, y) {
        const at = band.field.mapFromItem(band, x, y)
        return band.field.positionAt(at.x, at.y)
    }
    /// The two the hand below calls. **The anchor is taken at the press**, the way `SweepPad`'s is: a reader who
    /// presses in the band and drags gets the selection running from the character nearest where they started.
    function pressAt(x, y) {
        band.field.forceActiveFocus()
        band.field.cursorPosition = band.caretAt(x, y)
    }
    function moveAt(x, y) {
        band.field.moveCursorSelection(band.caretAt(x, y), TextEdit.SelectCharacters)
    }

    MouseArea {
        id: hand
        anchors.fill: parent
        acceptedButtons: Qt.LeftButton
        // Held against the surface this box stands on: the commit block and the details pane are both `Flickable`s,
        // and a drag they steal takes the selection with it.
        preventStealing: true
        cursorShape: Qt.IBeamCursor
        onPressed: mouse => band.pressAt(mouse.x, mouse.y)
        onPositionChanged: mouse => { if (hand.pressed) band.moveAt(mouse.x, mouse.y) }
    }
}
