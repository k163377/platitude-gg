pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The card under the author's name in the details pane (デザイン規約 §author の hover).
//
// A popup: anything declared inside the pane's column would be clipped by it and painted under the list below.
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
    /// How wide this card may stand; what is over it wraps. A popup clamps its position to the window, never its
    /// width, so without a cap a long address runs off the edge. The owner measures it (`DetailsAuthorCards.takeRoom`).
    property real maxWidth: 0
    /// What is left of it for a row's own words, once the frame's padding and the blocks' inset are out.
    readonly property real rowCap: authorCard.maxWidth > 0
        ? authorCard.maxWidth - 2 * authorCard.padding - 2 * Theme.spaceSm : Number.MAX_VALUE

    /// Either difference names both acts, even in the same second (デザイン規約 §author の hover).
    readonly property bool actsShown: authorCard.committerDiffers || authorCard.timeDiffers
    readonly property string wroteWord: qsTr("authored")
    readonly property string putWord: qsTr("committed")
    readonly property var wroteAct: ({ word: authorCard.wroteWord, stamp: Words.stamp(authorCard.authoredAt) })
    readonly property var putAct: ({ word: authorCard.putWord, stamp: Words.stamp(authorCard.committedAt) })
    /// The lines under the author's name — both of them when nobody else put the commit here.
    readonly property var authorActs: !authorCard.actsShown
        ? []
        : authorCard.committerDiffers ? [authorCard.wroteAct]
                                      : [authorCard.wroteAct, authorCard.putAct]
    /// ...and when somebody else put it here, the moment they did stands under *their* name.
    readonly property var committerActs: authorCard.committerDiffers ? [authorCard.putAct] : []
    // Measured off hidden labels: `TextMetrics` comes out a few pixels short of what a Label takes.
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
    // Flush against the underlined name, so the pointer walks down into it without leaving both.
    margins: 0
    // Both halves though no block takes hover yet (rules-refs/app-ui.md「中身がまだ hover を受けないカードにも置く」).
    tracksPointer: true
    contentPointed: contentHover.hovered
    // Every gap in this card is a place a selection can start (規約 §hover のツールチップ).
    textContent: cardBody

    /// One person: face, name, address, and when their part happened. Laid out from the start (why: `CoAuthorCard`).
    component PersonBlock: Item {
        id: block
        required property string name
        required property string address
        required property int face
        required property string faceUrl
        /// What this person did, and when — `{ word, stamp }` to a line; empty for none.
        required property var acts
        required property real wordWidth
        required property real cap

        readonly property bool hasAddress: block.address !== ""
        readonly property bool hasAct: block.acts.length > 0
        readonly property real textLeft: Theme.spaceSm + Theme.iconMd + Theme.spaceXs
        readonly property real textCap: block.cap - Theme.iconMd - Theme.spaceXs
        /// Over a row where a long name wrapped — it wraps: this card is where a name the pane cut is read in full
        /// (規約 §hover のツールチップ).
        readonly property real headHeight: Math.max(Theme.rowHeight, rowContent.implicitHeight)

        // Only what is drawn is measured: a hidden line still has a width, and would widen an ordinary card.
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
        // One step under the address, none between the stamps (デザイン規約 §author の hover).
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
