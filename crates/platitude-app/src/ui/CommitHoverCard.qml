pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// What a graph row says when the pointer rests on it: the message in
// full, and the facts the row's three columns do not carry — who wrote
// it, when, and whoever they credited.
//
// A card rather than a `ToolTip`: a tooltip is a string laid out in one
// block, its ground is the Fusion default from outside this theme, and
// it appears wherever the style decides — which is what made the old one
// feel detached from the pointer (P3-確認事項 §B).
//
// It answers and offers nothing: nothing in it opens anything further.
// Whoever wants the addresses behind the credited names, or the message
// as an editable field, has the details pane a click away — a preview
// that grows its own second popup is a preview asking to be read like a
// pane (規約 §co-author の表示).
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

    /// The pointer is over the card itself — on the padding band the
    /// background covers, or on the content. Two handlers, because the
    /// background and the content are siblings, not parent and child:
    /// the moment anything in the content takes the hover, the
    /// background's handler reads false (measured on RefListPopup's
    /// rows, 2026-08-09).
    readonly property bool pointerInside:
        insideHover.hovered || contentHover.hovered

    /// What the credit line was actually given, and whether the names
    /// ran past it. Read by the headless runs, which cannot see an
    /// ellipsis and cannot measure a card from a PNG (0 when the commit
    /// credits nobody).
    readonly property real creditWidth: mateLine.visible ? mateLine.width : 0
    readonly property bool creditCut: mateLine.visible && mateLine.clipped

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
        // The content's half of `pointerInside` — see the property.
        HoverHandler {
            id: contentHover
        }
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
        // Date, and beside it whoever the message credits — written out
        // and comma separated, because this card is the preview and the
        // details pane is where a commit is read in full (規約
        // §co-author の表示).
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
                plain: true
                Layout.alignment: Qt.AlignVCenter
                // The message sets this card's width; the credit line
                // takes what is left of it and elides. `preferredWidth`
                // 0 is what keeps the names out of the card's own size
                // hint — a crowd would otherwise widen the card past the
                // message it belongs to — and the minimum is the floor
                // under that: what the names need, but never more than a
                // message column's worth, so a one-line subject still
                // opens wide enough to credit somebody without leaving
                // empty room when it already was (規約 §co-author の表示).
                Layout.fillWidth: true
                Layout.preferredWidth: 0
                Layout.minimumWidth: Math.min(Metrics.messageMinW,
                                              mateLine.implicitWidth)
            }
        }
    }
}
