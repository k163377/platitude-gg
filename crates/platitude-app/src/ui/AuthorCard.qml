pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Who this commit belongs to, read off the author's name in the details pane: the address that tells two people of the
// same name apart, and — when the commit was not put here by the person who wrote it, or not at the moment they wrote
// it — the second person and the two moments.
//
// An ordinary commit is one block and nothing else: the same shape a co-author gets in `CoAuthorCard`, because it is
// the same fact (a person, a face, an address). Nothing marks the ordinary case (デザイン規約 §状態).
//
// A popup, for the reason the ref list and the co-author card are: anything declared
// inside the pane's column would be clipped by it and painted under the list below.
AppCard {
    id: authorCard

    property string authorName: ""
    property string authorEmail: ""
    property int authorFace: 0
    property string authorFaceUrl: ""
    property double authoredAt: 0
    property string committerName: ""
    property string committerEmail: ""
    property int committerFace: 0
    property string committerFaceUrl: ""
    property double committedAt: 0
    /// Somebody other than the author put this commit here.
    property bool committerDiffers: false
    /// ...or they put it here at another moment than it was written.
    property bool timeDiffers: false
    /// How wide this card may stand. Neither a name nor an address has a length worth trusting — an address may be
    /// 254 characters — and a popup clamps its *position* to the window, never its width, so without a cap a long one
    /// simply runs off the edge; what is over the cap wraps. **The owner measures the room against the window**
    /// (`DetailsAuthorCards.takeRoom`).
    property real maxWidth: 0
    /// What is left of it for a row's own words, once the frame's padding and the blocks' inset are out.
    readonly property real rowCap: authorCard.maxWidth > 0
        ? authorCard.maxWidth - 2 * authorCard.padding - 2 * Theme.spaceSm : Number.MAX_VALUE

    /// When the two acts are worth naming, both are named — the moment is what the second person's line is for, and a
    /// commit written and committed in the same second still has two lines to say so.
    readonly property bool actsShown: authorCard.committerDiffers || authorCard.timeDiffers
    readonly property string wroteWord: qsTr("authored")
    readonly property string putWord: qsTr("committed")
    readonly property var wroteAct: ({ word: authorCard.wroteWord, stamp: Words.stamp(authorCard.authoredAt) })
    readonly property var putAct: ({ word: authorCard.putWord, stamp: Words.stamp(authorCard.committedAt) })
    /// The lines under the author's name. **The same commit, put here later by the same hand, carries both** — one
    /// person, two moments, and nobody else to name.
    readonly property var authorActs: !authorCard.actsShown
        ? []
        : authorCard.committerDiffers ? [authorCard.wroteAct]
                                      : [authorCard.wroteAct, authorCard.putAct]
    /// ...and when somebody else put it here, the moment they did stands under *their* name.
    readonly property var committerActs: authorCard.committerDiffers ? [authorCard.putAct] : []
    // The stamps line up under one another, so the two moments can be compared at a glance
    // (`AppMenu.codeColW` shares a column the same way). Measured off labels — `TextMetrics` comes out
    // a few pixels short of what a Label actually takes.
    readonly property real wordColW: Math.max(wroteMetric.implicitWidth, putMetric.implicitWidth)
    CardText {
        id: wroteMetric
        visible: false
        text: authorCard.wroteWord
        pixelSize: Theme.fontSm
    }
    CardText {
        id: putMetric
        visible: false
        text: authorCard.putWord
        pixelSize: Theme.fontSm
    }

    padding: Theme.spaceXs
    // The card sits against the underlined name: the pointer has to be able to walk down into it without
    // leaving both.
    margins: 0
    // The pointer walks into this one and reads it. The blocks here accept no hover today; giving `AppCard` both halves
    // is what keeps that an implementation detail.
    tracksPointer: true
    contentPointed: contentHover.hovered
    // Every gap in this card is a place a selection can start — the inset the blocks keep, the step under a name, the
    // room beside a stamp (規約 §hover のツールチップ).
    textContent: cardBody

    /// One person: face, name, address, and when their part happened. Laid out from the start —
    /// see `CoAuthorCard` for what moves when a hover resizes its own target.
    component PersonBlock: Item {
        id: block
        required property string name
        required property string address
        required property int face
        required property string faceUrl
        /// What this person did, and when — `{ word, stamp }` to a line. An empty list drops them, which is the
        /// ordinary commit's whole story.
        required property var acts
        required property real wordWidth
        required property real cap

        readonly property bool hasAddress: block.address !== ""
        readonly property bool hasAct: block.acts.length > 0
        readonly property real textLeft: Theme.spaceSm + Theme.iconMd + Theme.spaceXs
        readonly property real textCap: block.cap - Theme.iconMd - Theme.spaceXs
        /// How tall the face-and-name line came out. A row's worth ordinarily; more when a name longer than the cap
        /// wrapped into a second line — **and it wraps**, because this card is where a name that
        /// the pane's own row had to cut goes to be read in full and taken away (規約 §hover のツールチップ).
        readonly property real headHeight: Math.max(Theme.rowHeight, rowContent.implicitHeight)

        // Only what is drawn is measured: a column with no line in it still stands at the text inset, and letting
        // that into the maximum makes an ordinary one-person card as wide as the act line it is not showing.
        implicitWidth: Math.max(rowContent.implicitWidth,
                                block.hasAddress ? address.x - Theme.spaceSm + address.width : 0,
                                block.hasAct ? actLines.x - Theme.spaceSm + actLines.width : 0)
                       + 2 * Theme.spaceSm
        // A block ends `spaceXs` under its own last line — what follows it is another person.
        implicitHeight: block.headHeight
                        + (block.hasAddress ? address.height : 0)
                        + (block.hasAct ? Theme.spaceXs + actLines.height : 0)
        width: implicitWidth
        height: implicitHeight

        RowLayout {
            id: rowContent
            x: Theme.spaceSm
            y: (block.headHeight - height) / 2
            spacing: Theme.spaceXs
            IdentIcon {
                code: block.face
                imageUrl: block.faceUrl
                width: Theme.iconMd
                height: Theme.iconMd
                Layout.alignment: Qt.AlignVCenter
            }
            CardText {
                text: block.name
                color: Theme.textPrimary
                pixelSize: Theme.fontMd
                Layout.maximumWidth: block.textCap
                Layout.alignment: Qt.AlignVCenter
            }
        }
        // Under the name, indented past the face so the two read as one person.
        CardText {
            id: address
            visible: block.hasAddress
            width: Math.min(implicitWidth, block.textCap)
            x: block.textLeft
            y: block.headHeight - Theme.spaceXs
            text: block.address
            color: Theme.textSecondary
            pixelSize: Theme.fontSm
        }
        // A step under the address: the address belongs to the name over it, and the moments are their own fact.
        // Nothing stands between the moments — two stamps are read against one another.
        Column {
            id: actLines
            visible: block.hasAct
            x: block.textLeft
            y: address.y + (block.hasAddress ? address.height : 0) + Theme.spaceXs
            Repeater {
                model: block.acts
                delegate: Row {
                    required property var modelData
                    spacing: Theme.spaceXs
                    CardText {
                        width: block.wordWidth
                        text: parent.modelData.word
                        color: Theme.textSecondary
                        pixelSize: Theme.fontSm
                    }
                    CardText {
                        text: parent.modelData.stamp
                        color: Theme.textSecondary
                        pixelSize: Theme.fontSm
                    }
                }
            }
        }
    }

    contentItem: Column {
        id: cardBody
        HoverHandler {
            id: contentHover
        }
        PersonBlock {
            name: authorCard.authorName
            address: authorCard.authorEmail
            face: authorCard.authorFace
            faceUrl: authorCard.authorFaceUrl
            acts: authorCard.authorActs
            wordWidth: authorCard.wordColW
            cap: authorCard.rowCap
        }
        PersonBlock {
            visible: authorCard.committerDiffers
            name: authorCard.committerName
            address: authorCard.committerEmail
            face: authorCard.committerFace
            faceUrl: authorCard.committerFaceUrl
            acts: authorCard.committerActs
            wordWidth: authorCard.wordColW
            cap: authorCard.rowCap
        }
    }
}
