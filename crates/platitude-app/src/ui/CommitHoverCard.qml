pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// What a row says when the pointer rests on it: the message in full, and the facts the row does not carry — who wrote
// it, when, and whoever they credited. **Two lists open it** — the graph's rows and the ones the right pane lists
// under a choice — and what it holds back depends on what the row it came off had already shown (`bodyRows`).
//
// A card rather than a `ToolTip`: a tooltip is a string laid out in one block, and it appears wherever the style
// decides — which is what made the old one feel detached from the pointer (P3-確認事項 §B). The ground stopped being a
// reason when the shared tip took the same card (`SharedToolTip`); the one-block shape is still one.
//
// It opens nothing of its own: no second popup grows out of it (規約 §co-author の表示). The one thing it does answer
// for is the message it had to stop — the note under a cut message is pressed, and what the press does is take the
// reader to the pane that holds the whole of it, which is a place already on screen. **Only where the note stands**
// (`asksForMore`): a card that held nothing back has nothing to send anyone to.
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
    /// How many lines of the body are worth a glance. **0 means all of it**, which is what a card opened over a list
    /// that is already showing the subjects is for: there the body is the whole reason the pointer stopped, and a
    /// paragraph's worth of it would be the card withholding the one thing it was opened for
    /// (デザイン規約 §複数のコミットを選ぶ). The subject keeps its cap either way — that one is bounded by the room
    /// there is, and a card that grows past the screen takes its own footer with it.
    property int bodyRows: Metrics.hoverBodyRows
    /// And the room the body may take when it is not counted in lines — the same kind of bound the subject has, and
    /// for the same reason: **a card is only ever as tall as the window can hold**. Uncapped, a five thousand byte
    /// body made a slab the height of the screen that covered the pane the card was opened from (measured
    /// `--preset edges`). 0 leaves it to `bodyRows`.
    property real bodyHeight: 0
    /// Whether the card offers the way to the rest of a message it had to stop. Off where nothing here is pressable:
    /// a card standing over the choice must not be a door, since walking through it is walking away from what the
    /// reader was picking (デザイン規約 §複数のコミットを選ぶ).
    property bool asksForMore: true

    /// What the credit line was actually given, and whether the names ran past it. Read by the headless runs, which
    /// cannot see an ellipsis and cannot measure a card from a PNG (0 when the commit credits nobody).
    readonly property real creditWidth: mateLine.visible ? mateLine.width : 0
    readonly property bool creditCut: mateLine.visible && mateLine.clipped

    /// The cap stopped the message somewhere. What the body says while it is hidden cannot be read off it — a hidden
    /// layout child is a zero-width one, and a field wrapped at zero calls itself cut every time.
    readonly property bool messageCut: subjectLine.clipped || (bodyLine.visible && bodyLine.clipped)

    /// The note under a cut message was pressed. The card holds no commit of its own beyond the fields it was handed,
    /// so which one this is about is the owner's answer (`RowHoverHost`).
    signal messageAsked()
    /// The press itself, so a headless run enters where the hand does rather than beside it (verify-ui スキル).
    function askMessage() {
        hoverCard.messageAsked()
    }
    /// Whether the note has the pointer — the rule under it is the only ink that moves, and a PNG cannot say whether
    /// the line that is there is the resting one.
    readonly property bool notePointed: noteHand.containsMouse

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
            capRows: hoverCard.bodyRows
            capHeight: hoverCard.bodyHeight
        }
        // Where the rest of it is, for the message this card had to stop — **and the way there** (規約 §hover の
        // ツールチップ). The line names itself rather than the commit: the hand reading this is inside the card, and a
        // sentence that sends it back out to a row it has already left is a sentence about a target the reader cannot
        // see. The press does what the row's own click does, and the card is in the way of what it leads to, so it
        // goes with the press.
        //
        // **It stands with the message**, under the mark and ahead of the author: put at the foot of the card it has
        // two lines of somebody else's facts between it and the `…` it answers, and reads as a note about the
        // card. **Centred, alone among these lines** — the others begin at the left margin because each is a value
        // read down a column, and this one is not a value. **Only when something was cut**: a message that fits has
        // nowhere further to go, and a line under every row the pointer crosses would be the card talking about
        // itself.
        //
        // A `Label` rather than a `CardText`, because this is the card speaking and not the commit: a note that joined
        // the selection would be dragged out along with the message somebody came here to copy. It is a target now, so
        // it stands on the card's sweep pad the way the band's badges do (規約 §右のペインの字は掴める).
        Item {
            id: noteLine
            visible: hoverCard.messageCut && hoverCard.asksForMore
            Layout.fillWidth: true
            implicitHeight: noteWords.implicitHeight + 2 * Theme.borderWidth
            Label {
                id: noteWords
                anchors.horizontalCenter: parent.horizontalCenter
                text: qsTr("Click here for the whole message")
                color: Theme.textMuted
                font.family: Theme.uiFamily
                font.pixelSize: Theme.fontSm
            }
            // What says the line can be pressed before the hand is near it: at rest one step under the words, and up
            // to the words' own colour under the pointer (規約 §author の hover — the footer that loads more commits
            // and the parent hash say the same thing the same way). **The only ink a resting card gains is this 1px.**
            Rectangle {
                x: noteWords.x
                y: noteWords.height + Theme.borderWidth
                width: noteWords.width
                height: Theme.borderWidth
                color: hoverCard.notePointed ? Theme.textMuted : Theme.borderStrong
            }
            // The words' own width, not the row's: the target is the sentence, and a hand that reaches the empty half
            // of a centred line has not aimed at anything.
            MouseArea {
                id: noteHand
                x: noteWords.x
                width: noteWords.width
                height: noteLine.height
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: hoverCard.askMessage()
            }
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
