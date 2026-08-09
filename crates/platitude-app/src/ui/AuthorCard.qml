pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// Who this commit belongs to, read off the author's name in the details
// pane: the address that tells two people of the same name apart, and —
// when the commit was not put here by the person who wrote it, or not at
// the moment they wrote it — the second person and the two moments.
//
// An ordinary commit is one block and nothing else: the same shape a
// co-author gets in `CoAuthorCard`, because it is the same fact (a
// person, a face, an address). Nothing marks the ordinary case
// (デザイン規約 §状態).
//
// A popup rather than an item under the name for the reason the ref list
// and the co-author card are: anything declared inside the pane's column
// would be clipped by it and painted under the list below.
Popup {
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
    /// How wide a name or address may run before it is elided. Neither
    /// has a length worth trusting — an address may be 254 characters —
    /// and a popup clamps its *position* to the window, never its width,
    /// so without a cap a long one simply runs off the edge. The owner
    /// sets it from the pane the card opens over.
    property real maxRowWidth: 0
    readonly property real rowCap:
        authorCard.maxRowWidth > 0 ? authorCard.maxRowWidth : Number.MAX_VALUE

    /// When the two acts are worth naming, both are named — the moment
    /// is what the second person's line is for, and a commit written and
    /// committed in the same second still has two lines to say so.
    readonly property bool actsShown:
        authorCard.committerDiffers || authorCard.timeDiffers
    readonly property string wroteWord: qsTr("authored")
    readonly property string putWord: qsTr("committed")
    // The stamps line up under one another, so the two moments can be
    // compared at a glance rather than read (`AppMenu.codeColW` shares a
    // column the same way). Measured off labels rather than
    // `TextMetrics`, which comes out a few pixels short of what a Label
    // actually takes.
    readonly property real wordColW: Math.max(wroteMetric.implicitWidth,
                                              putMetric.implicitWidth)
    Label {
        id: wroteMetric
        visible: false
        text: authorCard.wroteWord
        font.pixelSize: Theme.fontSm
    }
    Label {
        id: putMetric
        visible: false
        text: authorCard.putWord
        font.pixelSize: Theme.fontSm
    }

    padding: Theme.spaceXs
    // Nothing stands between the underlined name and this: the pointer
    // has to be able to walk down into it without leaving both.
    margins: 0
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    /// Whether the pointer is over this — on the padding band the
    /// background covers, or on the content. Two handlers, because the
    /// background and the content are siblings, not parent and child:
    /// "handlers are passive" only holds down a subtree, so the moment
    /// anything in the content takes the hover, the background's handler
    /// reads false (measured on RefListPopup's rows, 2026-08-09). The
    /// blocks here accept no hover today; the pair is what keeps that an
    /// implementation detail rather than a load-bearing fact.
    readonly property bool pointerInside:
        insideHover.hovered || contentHover.hovered

    // On the background, not the content: the content stops at the
    // padding, so a pointer walking in over the card's own border is
    // over neither it nor the name that opened it, and the card closes
    // in that 4px band (2026-08-09 report, CoAuthorCard).
    background: Rectangle {
        color: Theme.bgElevated
        radius: Theme.radiusMd
        border.color: Theme.borderDefault
        border.width: Theme.borderWidth
        HoverHandler {
            id: insideHover
        }
    }

    /// One person: face, name, address, and the moment their part
    /// happened. Laid out from the start, never on hover — see
    /// `CoAuthorCard` for what moves when a hover resizes its own target.
    component PersonBlock: Item {
        id: block
        required property string name
        required property string address
        required property int face
        required property string faceUrl
        /// What this person did, and when. Empty word = the line is not
        /// drawn, which is the ordinary commit's whole story.
        required property string word
        required property string stamp
        required property real wordWidth
        required property real cap

        readonly property bool hasAddress: block.address !== ""
        readonly property bool hasAct: block.word !== ""
        readonly property real textLeft: Theme.spaceSm + Theme.iconMd
                                         + Theme.spaceXs
        readonly property real textCap: block.cap - Theme.iconMd
                                        - Theme.spaceXs

        // Only what is drawn is measured: an invisible Row still knows
        // how wide its labels are, and letting that into the maximum
        // makes an ordinary one-person card as wide as the act line it
        // is not showing.
        implicitWidth: Math.max(rowContent.implicitWidth,
                                block.hasAddress
                                ? address.x - Theme.spaceSm + address.width : 0,
                                block.hasAct
                                ? act.x - Theme.spaceSm + act.width : 0)
                       + 2 * Theme.spaceSm
        implicitHeight: Theme.rowHeight
                        + (block.hasAddress ? Theme.fontSmLine : 0)
                        + (block.hasAct ? Theme.fontSmLine : 0)
        width: implicitWidth
        height: implicitHeight

        RowLayout {
            id: rowContent
            x: Theme.spaceSm
            y: (Theme.rowHeight - height) / 2
            spacing: Theme.spaceXs
            IdentIcon {
                code: block.face
                imageUrl: block.faceUrl
                width: Theme.iconMd
                height: Theme.iconMd
                Layout.alignment: Qt.AlignVCenter
            }
            Label {
                text: block.name
                color: Theme.textPrimary
                font.pixelSize: Theme.fontMd
                elide: Text.ElideRight
                Layout.maximumWidth: block.textCap
                Layout.alignment: Qt.AlignVCenter
            }
        }
        // Under the name, indented past the face so the two read as one
        // person.
        Label {
            id: address
            visible: block.hasAddress
            width: Math.min(implicitWidth, block.textCap)
            elide: Text.ElideRight
            x: block.textLeft
            y: Theme.rowHeight - Theme.spaceXs
            text: block.address
            color: Theme.textSecondary
            font.pixelSize: Theme.fontSm
        }
        Row {
            id: act
            visible: block.hasAct
            spacing: Theme.spaceXs
            x: block.textLeft
            y: address.y + (block.hasAddress ? Theme.fontSmLine : 0)
            Label {
                width: block.wordWidth
                text: block.word
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
            Label {
                text: block.stamp
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
        }
    }

    contentItem: Column {
        // The content's half of `pointerInside` — see the property.
        HoverHandler {
            id: contentHover
        }
        PersonBlock {
            name: authorCard.authorName
            address: authorCard.authorEmail
            face: authorCard.authorFace
            faceUrl: authorCard.authorFaceUrl
            word: authorCard.actsShown ? authorCard.wroteWord : ""
            stamp: Qt.formatDateTime(new Date(authorCard.authoredAt * 1000),
                                     "yyyy-MM-dd HH:mm")
            wordWidth: authorCard.wordColW
            cap: authorCard.rowCap
        }
        PersonBlock {
            visible: authorCard.committerDiffers
            name: authorCard.committerName
            address: authorCard.committerEmail
            face: authorCard.committerFace
            faceUrl: authorCard.committerFaceUrl
            word: authorCard.actsShown ? authorCard.putWord : ""
            stamp: Qt.formatDateTime(new Date(authorCard.committedAt * 1000),
                                     "yyyy-MM-dd HH:mm")
            wordWidth: authorCard.wordColW
            cap: authorCard.rowCap
        }
        // The same commit, put here later by the same hand: one person,
        // two moments. The line belongs under them rather than in a
        // block of its own — there is nobody else to name.
        Item {
            visible: authorCard.timeDiffers && !authorCard.committerDiffers
            implicitWidth: lateAct.x - Theme.spaceSm + lateAct.width
                           + 2 * Theme.spaceSm
            implicitHeight: visible ? Theme.fontSmLine : 0
            width: implicitWidth
            height: implicitHeight
            Row {
                id: lateAct
                spacing: Theme.spaceXs
                x: Theme.spaceSm + Theme.iconMd + Theme.spaceXs
                Label {
                    width: authorCard.wordColW
                    text: authorCard.putWord
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontSm
                }
                Label {
                    text: Qt.formatDateTime(
                              new Date(authorCard.committedAt * 1000),
                              "yyyy-MM-dd HH:mm")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontSm
                }
            }
        }
    }
}
