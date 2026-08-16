pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// What is in the working tree, by kind, on the graph's uncommitted row.
// Conflicts lead: what is stopped is the thing to see first, and the rest
// is the order the file list would put them in.
RowLayout {
    id: tallies

    required property int conflicted
    required property int added
    required property int modified
    required property int deleted
    required property int renamed
    required property int copied

    // One kind of change and how many rows of it the file list holds. The
    // mark is the same ChangeIcon those rows carry, and a kind with
    // nothing in it takes no seat (デザイン規約 §無効).
    component Tally: RowLayout {
        id: tally
        required property string code
        required property int count
        visible: tally.count > 0
        // Nothing between the mark and its number: the seat below is the
        // ink's width, so what the eye measures is already the mark's own
        // air (デザイン規約 §余白).
        spacing: 0
        // The seat is the ink, not the box. Drawn to the box, `!` would
        // stand five pixels from its own number while `+` stood two, and
        // neither would belong to it.
        Item {
            Layout.preferredWidth: tallyMark.inkWidth
            Layout.preferredHeight: Theme.iconXs
            ChangeIcon {
                id: tallyMark
                anchors.centerIn: parent
                change: tally.code
                // A step under `iconSm`: this mark stands beside a digit
                // of its own rather than beside a word, and at `iconSm` it
                // measured 8px against the digit's 6 (規約 §寸法).
                width: Theme.iconXs
                height: Theme.iconXs
                // The grid shrinks and the line has to shrink with it, or
                // the mark carries more weight than the digit beside it
                // (app-ui.md §語の隣に立つ印).
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
