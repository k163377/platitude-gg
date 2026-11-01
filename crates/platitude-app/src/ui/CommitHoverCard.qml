pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// What a row says when the pointer rests on it: the message in full, and who wrote it, when, and whoever they
// credited. Opened from the graph's rows and from the right pane's list under a multi-selection; how much body it
// shows depends on what that row already showed (`bodyRows`).
//
// A card, not a tooltip: a tooltip lays its string out in one block.
//
// No second popup grows out of it (`CoAuthorLine`'s plain form). Its one press is the note under a cut message, which
// takes the reader to the pane holding the whole message (`asksForMore`) — handing over to something already standing,
// not opening anything (規約 §hover のツールチップ「ただし「渡す」は別の所作」).
//
// The note, only where the message was cut (user decisions — ask before changing): `Click here for the whole message`,
// a line that is itself the way — a sentence that only names where to go (`Click the commit for …`) is words that do
// nothing for a hand already inside the card. A press does what a click on the row does: the card goes at once, the
// row is chosen, the right pane shows the commit, and its two message frames take the `accent` frame
// (`MessageEditor.attention`). Its seat is right under the cut mark, above author and date — at the foot, two lines of
// other facts between it and the `…` it answers would read as a note about the card. The card's one centred line:
// left-aligned lines read as a column of values, this one is the card's own words. `fontSm` `textMuted` at rest, out of
// any selection (the card says it, so it does not come along with the message). The target is the words' width; the
// underline says it can be pressed — `borderStrong` at rest, the words' colour under the pointer — and the words rise
// with it (`textMuted` → `textSecondary`): from the darkest text token a rising rule alone barely shows. No ground
// under the pointer: this is the quiet kind of target, and a band reads as a disabled state.
//
// Owned by the page: rows are recycled the moment they scroll off, and a popup parented to a row goes with it.
AppCard {
    id: hoverCard

    property string subject: ""
    property string body: ""
    property string author: ""
    property double authorTime: 0
    /// The co-author records for the line beside the date (`encode::Mates`).
    property var mates: []
    /// How wide the message may run before it wraps: a share of the pane the card opens over, set by the owner (so
    /// Metrics holds no token for it).
    property real textWidth: 0
    /// How tall the subject may grow, from the same pane share as the width: git bounds no subject, and an uncapped
    /// card runs off the screen taking its own footer with it. The body is capped in lines instead (`bodyRows`).
    property real subjectHeight: 0
    /// How many body lines are worth a glance — a count of lines, not a share of the pane (user decision): a quarter
    /// of the pane promises "read it all here" and does not keep it, hence the note. The subject stays capped by room:
    /// it is the row's heading, the side that stays. 0 means all of it — for a card over a list already showing the
    /// subjects, where the body is what the pointer stopped for (`RowHoverHost.openRowCard`).
    property int bodyRows: Metrics.hoverBodyRows
    /// The body's height cap when it is not counted in lines, for the subject's reason: a card is only ever as tall as
    /// the window can hold. 0 leaves it to `bodyRows`.
    property real bodyHeight: 0
    /// Whether the card offers the way to the rest of a cut message. Off over a multi-selection, where a door would
    /// lead away from what the reader was picking (`RowHoverHost.openRowCard`).
    property bool asksForMore: true

    /// The credit line's given width (0 when nobody is credited) and whether the names ran past it — for headless
    /// runs, which cannot see an ellipsis.
    readonly property real creditWidth: mateLine.visible ? mateLine.width : 0
    readonly property bool creditCut: mateLine.visible && mateLine.clipped

    /// The cap stopped the message somewhere. The body is read only while visible: a hidden layout child is zero
    /// wide, and a field wrapped at zero always calls itself cut. Headless, only `--preset edges` row 7 carries a
    /// message the card cuts: `row-card 7 --preset edges` shows the note at rest, `card-note-lit` under the hand,
    /// `card-message` / `card-message-esc` pressed and taken down.
    readonly property bool messageCut: subjectLine.clipped || (bodyLine.visible && bodyLine.clipped)

    /// The note under a cut message was pressed; which commit that is, is the owner's answer (`RowHoverHost`).
    signal messageAsked()
    /// The press itself, so a headless run enters where the hand does (verify-ui スキル). The owner reads which commit
    /// and row before it closes the card, whose close clears both (`RowHoverHost`).
    function askMessage() {
        hoverCard.messageAsked()
    }
    /// Headless stand-in for the pointer on the note — hover cannot be injected (verify-ui スキル §hover の絵の撮り方).
    property bool notePointedAt: false
    readonly property bool notePointed: noteHand.containsMouse || hoverCard.notePointedAt
    /// What the note actually paints, for a run to read back (PGG_AUTO_ACT=card-note-lit).
    readonly property alias noteWordColor: noteWords.color
    readonly property alias noteRuleColor: noteRule.color

    margins: Theme.spaceXs
    // The pointer walks into this card, so both halves of `AppCard.pointerInside` are wired.
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
            weight: Theme.fontWeightStrong
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
            capRows: hoverCard.bodyRows
            capHeight: hoverCard.bodyHeight
        }
        // The way to the whole of a cut message (seat, centring and colours: the header above). A `Label`, so
        // it is not dragged out with a selection of the message; as a target it stands on the card's sweep pad
        // (規約 §右のペインの字は掴める).
        Item {
            id: noteLine
            visible: hoverCard.messageCut && hoverCard.asksForMore
            Layout.fillWidth: true
            implicitHeight: noteWords.implicitHeight + 2 * Theme.borderWidth
            Label {
                id: noteWords
                anchors.horizontalCenter: parent.horizontalCenter
                text: qsTr("Click here for the whole message")
                // The words rise too: from `textMuted`, a rule rising only to the words' colour would barely change.
                color: hoverCard.notePointed ? Theme.textSecondary : Theme.textMuted
                font.family: Theme.uiFamily
                font.pixelSize: Theme.fontSm
            }
            // The underline that says the line is pressable. Under the pointer it reads the words' own colour, so the
            // two cannot come apart.
            Rectangle {
                id: noteRule
                x: noteWords.x
                y: noteWords.height + Theme.borderWidth
                width: noteWords.width
                height: Theme.borderWidth
                color: hoverCard.notePointed ? noteWords.color : Theme.borderStrong
            }
            // The words' width only: the empty sides of a centred line are not the target.
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
        // Held to the message's width: git bounds no author name, and a Popup is as wide as its widest child whatever
        // the others were told. What will not fit wraps (規約 §hover のツールチップ).
        CardText {
            Layout.fillWidth: true
            Layout.maximumWidth: hoverCard.textWidth
            text: hoverCard.author
            color: Theme.textPrimary
            pixelSize: Theme.fontMd
        }
        // Date, and beside it the co-author credit, written out (`CoAuthorLine`). On one baseline, both one-line
        // fields set at a label's height, as `CommitAuthorRow`'s date line: a `CardText` date files the face's leading
        // under its line, so centred beside the names it stands half of it high, and on the baseline the row grows by it.
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceSm
            LineText {
                text: Words.stamp(hoverCard.authorTime)
                color: Theme.textSecondary
                pixelSize: Theme.fontSm
                ground: Theme.bgElevated
                // Whole: the credit beside it is what gives way.
                Layout.minimumWidth: implicitWidth
                Layout.alignment: Qt.AlignBaseline
            }
            CoAuthorLine {
                id: mateLine
                records: hoverCard.mates
                plain: true
                // The card's face, so the cut mark is drawn on it (`LineText.ground`).
                ground: Theme.bgElevated
                Layout.alignment: Qt.AlignBaseline
                // The message sets the card's width; the credit line takes what is left and elides. `preferredWidth` 0
                // keeps a crowd of names out of the card's size hint; the minimum (the names' need, capped at
                // `messageMinW`) keeps a one-line subject's card wide enough to credit somebody. Not a `maximumWidth`
                // cut: a commit crediting many still stretches the card. Not a flat `messageMinW` floor: short names
                // leave blank room.
                Layout.fillWidth: true
                Layout.preferredWidth: 0
                Layout.minimumWidth: Math.min(Metrics.messageMinW, mateLine.implicitWidth)
            }
        }
    }
}
