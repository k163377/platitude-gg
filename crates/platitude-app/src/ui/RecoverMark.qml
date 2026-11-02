import QtQuick
import platitude.ui

// The discard log's mark for an entry or a part (`RecoverEntries`): what kind of thing it took, in its section's
// tint, and where (user decision — ask before changing):
// - here alone: the kind's mark
// - a remote's alone: for a tag, the tag a step down with the cloud on its shoulder (`ShoulderBadge`) — the cloud
//   alone would not say tag, and the pair below means something else; a remote's branch is the cloud itself, REMOTES'
//   mark
// - here and on a remote at once: the kind over the cloud across a slash — two things taken, two marks — a fraction
//   like `ActionButtonSeat`'s in `iconXs` halves, stood up: taller than wide, short of the row at either end, and as
//   wide as the seat lets it be — centred on the seat as a lone mark is, it keeps air to the row's edge and to the
//   words' first letter, so every row's words start in one column. The one mark whose seat is wider than
//   `MarkSeat`'s one width (a special design)
//
// Two pairs in a row put the cloud of the one above over the kind of the one below, in the same columns — the pair is
// narrow — so a pair stays inside its row, with air left between the two.
//
// The band's lines are one part each, so they never pair: of them, only a remote's tag wears the shoulder cloud.
Item {
    id: mark

    /// `NavIcon`'s kind, and its tint.
    property string kind: ""
    property color tint: Theme.textSecondary
    /// The cloud on the shoulder: what was taken was a remote's.
    property bool badged: false
    /// The kind over the cloud: taken here and on a remote at once.
    property bool paired: false
    /// The pixel size of the words the mark heads: a lone mark is drawn to their letters, the box's middle their
    /// capitals' middle (デザイン規約 §アイコンの席 の文頭記号) — the caller stands the box's middle there.
    property real size: Theme.fontMd
    /// The pair's box: the kind at its top-left corner, the cloud at its bottom-right. As wide as the seat lets it be:
    /// the ink keeps a pixel or more of air to the row's edge and before the words (`RecoverPane`'s row, the box
    /// centred on the seat).
    readonly property int pairWidth: Theme.iconSm + Theme.spaceXs
    readonly property int pairHeight: Theme.iconLg
    /// The slash, end to end, as long as `ActionButtonSeat`'s and at its weight — lighter than the halves, being
    /// neither of them.
    readonly property real slashReach: Theme.iconSm + Theme.spaceXs / 2
    readonly property real slashStroke: Metrics.iconStroke * Theme.iconSm / 16

    implicitWidth: mark.paired ? mark.pairWidth : Theme.iconSm
    implicitHeight: mark.paired ? mark.pairHeight : Theme.iconSm

    // A step taller than the box, on the box's middle: the branch and the tree run past the capitals both ways and
    // come out as tall as the box, and a canvas only the box's height cuts what lands past it.
    NavIcon {
        visible: !mark.paired
        width: Theme.iconSm
        height: Theme.iconSm + Theme.spaceXs
        anchors.verticalCenter: parent.verticalCenter
        set: "lead"
        size: mark.size
        wordBase: (Theme.iconSm + Theme.spaceXs) / 2 + WordFace.capitalMiddle(mark.size)
        kind: mark.kind
        tint: mark.tint
        ShoulderBadge {
            visible: mark.badged
            kind: "remote"
            tint: Theme.textSecondary
            // On the box's shoulder, not the taller canvas's.
            anchors.topMargin: -Theme.spaceXs / 2
        }
    }
    // The halves at opposite corners of the pair's box across the slash. The box keeps the pair's size whether or not
    // the mark is paired, so a row handed another entry shows a slash drawn once at that size.
    Item {
        visible: mark.paired
        width: mark.pairWidth
        height: mark.pairHeight
        NavIcon {
            x: 0
            y: 0
            width: Theme.iconXs
            height: Theme.iconXs
            kind: mark.kind
            tint: mark.tint
        }
        // Drawn, not a glyph — a font's `/` keeps its own slant and sits on its baseline — through the box's middle at
        // 45°, as far across as up. Square to the line between the halves' centres (`ActionButtonSeat`'s) it would lie
        // almost flat in a box this tall.
        InkCanvas {
            anchors.fill: parent
            onPaint: {
                const ctx = getContext("2d")
                const half = mark.slashReach / 2 / Math.SQRT2
                ctx.clearRect(0, 0, width, height)
                ctx.strokeStyle = Theme.textSecondary
                ctx.lineWidth = mark.slashStroke
                ctx.lineCap = "round"
                ctx.beginPath()
                ctx.moveTo(width / 2 - half, height / 2 + half)
                ctx.lineTo(width / 2 + half, height / 2 - half)
                ctx.stroke()
            }
        }
        NavIcon {
            x: parent.width - width
            y: parent.height - height
            width: Theme.iconXs
            height: Theme.iconXs
            kind: "remote"
            tint: Theme.textSecondary
        }
    }
}
