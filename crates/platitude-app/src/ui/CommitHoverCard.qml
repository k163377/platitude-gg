pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// What a graph row says when the pointer rests on it: the message in full, and the facts the row's three columns do not
// carry — who wrote it, when, and whoever they credited.
//
// A card rather than a `ToolTip`: a tooltip is a string laid out in one block, and it appears wherever the style
// decides — which is what made the old one feel detached from the pointer (P3-確認事項 §B). The ground stopped being a
// reason when the shared tip took the same card (`SharedToolTip`); the one-block shape is still one.
//
// It answers and offers nothing: nothing in it opens anything further. Whoever wants the addresses behind the credited
// names, or the message as an editable field, has the details pane a click away — a preview that grows its own second
// popup is a preview asking to be read like a pane (規約 §co-author の表示).
//
// Owned by the page, not the delegate: rows are recycled the moment they scroll off, and a popup parented to one goes
// with it.
AppCard {
    id: hoverCard

    property string subject: ""
    property string body: ""
    property string author: ""
    property double atime: 0
    /// Packed co-author records for the line beside the date.
    property string mates: ""
    /// How wide the message may run before it wraps. The owner sets it from the pane the card opens over — there is no
    /// token for it because it is not a fixed size, it is a share of what is there.
    property real textWidth: 0
    /// How tall the subject may grow before it stops, from the same share of the same pane as the width. A subject has
    /// no length git enforces, and a card that grows with one runs off the screen and takes its own footer with it:
    /// measured at a 2,000-byte subject, the date and the credit line ended below the window. It is a share rather than
    /// a count of lines because the subject is the row's own heading — what may be cut off it is whatever the screen
    /// cannot hold, not a figure somebody chose. **The body is the other way round** (`Metrics.hoverBodyRows`): a
    /// paragraph is what a glance is worth, and the room there happens to be has nothing to do with it (規約 §hover の
    /// ツールチップ).
    property real subjectHeight: 0

    /// What the credit line was actually given, and whether the names ran past it. Read by the headless runs, which
    /// cannot see an ellipsis and cannot measure a card from a PNG (0 when the commit credits nobody).
    readonly property real creditWidth: mateLine.visible ? mateLine.width : 0
    readonly property bool creditCut: mateLine.visible && mateLine.clipped

    /// The cap stopped the message somewhere. What the body says while it is hidden cannot be read off it — a hidden
    /// layout child is a zero-width one, and a field wrapped at zero calls itself cut every time.
    readonly property bool messageCut: subjectLine.clipped || (bodyLine.visible && bodyLine.clipped)

    margins: Theme.spaceXs
    // The pointer walks into this one and reads it, so both halves of `AppCard.pointerInside` are wanted — the face's
    // and the content's.
    tracksPointer: true
    contentPointed: contentHover.hovered
    // Every gap in this card is a place a selection can start (規約 §hover のツールチップ).
    textContent: cardBody

    contentItem: ColumnLayout {
        id: cardBody
        spacing: Theme.spaceXs
        HoverHandler {
            id: contentHover
        }
        CardText {
            id: subjectLine
            Layout.fillWidth: true
            Layout.maximumWidth: hoverCard.textWidth
            text: hoverCard.subject
            color: Theme.textPrimary
            pixelSize: Theme.fontMd
            weight: Font.DemiBold
            capHeight: hoverCard.subjectHeight
        }
        CardText {
            id: bodyLine
            visible: hoverCard.body !== ""
            Layout.fillWidth: true
            Layout.maximumWidth: hoverCard.textWidth
            text: hoverCard.body
            color: Theme.textSecondary
            pixelSize: Theme.fontMd
            // A paragraph's worth, not a share of the pane: the card is what a message is glanced at in, and a preview
            // that reached half the window was promising a read it could not give (規約 §hover のツールチップ).
            capRows: Metrics.hoverBodyRows
        }
        // Where the rest of it is, for the message this card had to stop. Nothing here opens anything, so naming the
        // place that holds the whole of it is all the card can do — and the place is a click away, on the row the
        // pointer is already resting on (規約 §hover のツールチップ). **The commit is named rather than left to
        // "click"**: the hand reading this is inside the card, where a press does nothing.
        //
        // **It stands with the message**, under the mark and ahead of the author: put at the foot of the card it has
        // two lines of somebody else's facts between it and the `…` it answers, and reads as a note about the card
        // (2026-08-28 ユーザー報告). **Centred, alone among these lines** (2026-08-28 ユーザー指示) — the others begin at
        // the left margin because each is a value read down a column, and this one is not a value. **Only when
        // something was cut**: a message that fits has nowhere further to go, and a line under every row the pointer
        // crosses would be the card talking about itself.
        //
        // A `Label` rather than a `CardText`, because this is the card speaking and not the commit: a note that joined
        // the selection would be dragged out along with the message somebody came here to copy.
        Label {
            visible: hoverCard.messageCut
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            text: qsTr("Click the commit for the whole message")
            color: Theme.textMuted
            font.family: Theme.uiFamily
            font.pixelSize: Theme.fontSm
        }
        // The same width the message is held to. Nothing in git bounds an author's name either, and this one line was
        // the only field here outside the share: a name of a couple of hundred characters widened the whole card past
        // the pane it opens over (measured 1,031px over a 775px pane), because a Popup is as wide as its widest child
        // no matter what the others were told. What will not fit wraps rather than being cut — the name is here to be
        // read and taken away (規約 §hover のツールチップ).
        CardText {
            Layout.fillWidth: true
            Layout.maximumWidth: hoverCard.textWidth
            text: hoverCard.author
            color: Theme.textPrimary
            pixelSize: Theme.fontMd
        }
        // Date, and beside it whoever the message credits — written out and comma separated, because this card is the
        // preview and the details pane is where a commit is read in full (規約 §co-author の表示).
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceSm
            CardText {
                text: Words.stamp(hoverCard.atime)
                color: Theme.textSecondary
                pixelSize: Theme.fontSm
            }
            CoAuthorLine {
                id: mateLine
                packed: hoverCard.mates
                plain: true
                // The card's own face, so the mark a cut leaves is drawn on it rather than on the pane's ground
                // (`LineText.ground`).
                ground: Theme.bgElevated
                Layout.alignment: Qt.AlignVCenter
                // The message sets this card's width; the credit line takes what is left of it and elides.
                // `preferredWidth` 0 is what keeps the names out of the card's own size hint — a crowd would otherwise
                // widen the card past the message it belongs to — and the minimum is the floor under that: what the
                // names need, but never more than a message column's worth, so a one-line subject still opens wide
                // enough to credit somebody without leaving empty room when it already was (規約 §co-author の表示).
                Layout.fillWidth: true
                Layout.preferredWidth: 0
                Layout.minimumWidth: Math.min(Metrics.messageMinW, mateLine.implicitWidth)
            }
        }
    }
}
