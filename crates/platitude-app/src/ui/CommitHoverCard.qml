pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// What a graph row says when the pointer rests on it: the message in
// full, and the facts the row's three columns do not carry — who wrote
// it, when, and whoever they credited.
//
// A card rather than a `ToolTip` because a tooltip is a string: it can
// hold neither a face nor a line that is itself hoverable. It is also
// the only shape that can be put where the row is — an attached ToolTip
// appears wherever the style decides, which is what made the old one
// feel detached from the pointer (P3-確認事項 §B).
//
// Owned by the page, not the delegate: rows are recycled the moment they
// scroll off, and a popup parented to one goes with it.
Popup {
    id: hoverCard

    property string subject: ""
    property string body: ""
    property string author: ""
    property double atime: 0
    /// Packed co-author records for the line beside the date.
    property string mates: ""
    /// How wide the message may run before it wraps. The owner sets it
    /// from the pane the card opens over — there is no token for it
    /// because it is not a fixed size, it is a share of what is there.
    property real textWidth: 0
    /// How tall either of the two wrapping fields may grow before it
    /// elides, from the same share of the same pane. A message has no
    /// length git enforces, and a card that grows with one runs off the
    /// screen and takes its own footer with it: measured at a 2,000-byte
    /// subject, the date and the credit line ended below the window. This
    /// card is the preview — the details pane is where a message is read
    /// in full — so the two fields stop and say so with an ellipsis.
    property real textHeight: 0

    /// The pointer is over the card itself.
    readonly property alias pointerInside: insideHover.hovered
    /// The co-author stretch, so whoever owns the list that opens off it
    /// can put that list under the stretch rather than under the card.
    readonly property alias matesAnchor: mateLine
    /// ...or over the co-author stretch inside it, which opens its own.
    signal matesPointed(bool inside)
    /// Full weight on that stretch's rule while its card is up.
    property bool matesLit: false

    padding: Theme.spaceSm
    margins: Theme.spaceXs
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    // On the background, so the padding band counts as being inside —
    // see CoAuthorCard for what happens when it does not.
    background: Rectangle {
        color: Theme.bgElevated
        radius: Theme.radiusMd
        border.color: Theme.borderDefault
        border.width: Theme.borderWidth
        HoverHandler {
            id: insideHover
        }
    }

    contentItem: ColumnLayout {
        spacing: Theme.spaceXs
        Label {
            Layout.fillWidth: true
            Layout.maximumWidth: hoverCard.textWidth
            Layout.maximumHeight: hoverCard.textHeight
            text: hoverCard.subject
            color: Theme.textPrimary
            font.pixelSize: Theme.fontMd
            font.weight: Font.DemiBold
            wrapMode: Text.Wrap
            elide: Text.ElideRight
        }
        Label {
            visible: hoverCard.body !== ""
            Layout.fillWidth: true
            Layout.maximumWidth: hoverCard.textWidth
            Layout.maximumHeight: hoverCard.textHeight
            text: hoverCard.body
            color: Theme.textSecondary
            font.pixelSize: Theme.fontMd
            wrapMode: Text.Wrap
            elide: Text.ElideRight
        }
        // The same width the message is held to. Nothing in git bounds an
        // author's name either, and this one line was the only field here
        // outside the share: a name of a couple of hundred characters
        // widened the whole card past the pane it opens over (measured
        // 1,031px over a 775px pane), because a Popup is as wide as its
        // widest child no matter what the others were told.
        Label {
            Layout.fillWidth: true
            Layout.maximumWidth: hoverCard.textWidth
            text: hoverCard.author
            color: Theme.textPrimary
            font.pixelSize: Theme.fontMd
            elide: Text.ElideRight
        }
        // Date, and beside it whoever the message credits — the same
        // pairing the details pane uses, drawn by the same component
        // (規約 §co-author の表示).
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceSm
            Label {
                text: Qt.formatDateTime(new Date(hoverCard.atime * 1000),
                                        "yyyy-MM-dd HH:mm")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontSm
            }
            CoAuthorLine {
                id: mateLine
                packed: hoverCard.mates
                lit: hoverCard.matesLit
                nameWidth: hoverCard.textWidth
                Layout.alignment: Qt.AlignVCenter
                onPointerChanged: inside => hoverCard.matesPointed(inside)
            }
            Item { Layout.fillWidth: true }
        }
    }
}
