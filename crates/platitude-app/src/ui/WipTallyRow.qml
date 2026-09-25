pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The working tree's changes by kind, on the graph's uncommitted row. Conflicts first; the rest in the file list's
// order.
RowLayout {
    id: tallies

    required property int conflicted
    required property int added
    required property int modified
    required property int deleted
    required property int renamed
    required property int copied

    // One kind of change and its count, marked with the rows' ChangeIcon; an empty kind takes no seat (デザイン規約
    // §無効).
    component Tally: RowLayout {
        id: tally
        required property string code
        required property int count
        visible: tally.count > 0
        // No gap: the seat below is the ink's width, so the mark's own air is the gap (デザイン規約 §余白).
        spacing: 0
        // The seat is the ink (デザイン規約 §未コミット行が名乗るもの).
        Item {
            Layout.preferredWidth: tallyMark.inkWidth
            Layout.preferredHeight: Theme.iconXs
            ChangeIcon {
                id: tallyMark
                anchors.centerIn: parent
                change: tally.code
                // A step under `iconSm`: paired with a digit into one unit (規約 §寸法).
                width: Theme.iconXs
                height: Theme.iconXs
                // The stroke shrinks with the grid, or the mark outweighs the digit beside it
                // (rules-refs/app-ui.md「語の隣に立つ印は `iconSm`、席はインクに引く」).
                stroke: Metrics.iconStroke * Theme.iconXs / Theme.iconMd
            }
        }
        Label {
            leftPadding: Theme.spaceXs / 2
            text: tally.count
            color: tallyMark.tint
            font.pixelSize: Theme.fontSm
        }
    }

    spacing: Theme.spaceSm
    Tally {
        code: "UU"
        count: tallies.conflicted
    }
    Tally {
        code: "A"
        count: tallies.added
    }
    Tally {
        code: "M"
        count: tallies.modified
    }
    Tally {
        code: "D"
        count: tallies.deleted
    }
    Tally {
        code: "R"
        count: tallies.renamed
    }
    Tally {
        code: "C"
        count: tallies.copied
    }
}
