pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The worktree's changes by kind, on the graph's uncommitted row. Conflicts first; the rest in the file list's
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
        // No gap: the mark is a sign before the count (デザイン規約 §アイコンの席 の修飾), in the middle of its seat,
        // whose air parts it from the count — but the conflicts' `!`, drawn for the face's `!`, stands as that
        // character with no seat, set to its count as ↑↓ are (§アイコンの席 の字の代替): in the cloud's seat the sliver
        // would stand off its count. Its box is the count's line, so the two share a baseline.
        spacing: 0
        ChangeIcon {
            id: tallyMark
            Layout.preferredWidth: tallyMark.wordCell
            Layout.preferredHeight: tallyCount.implicitHeight
            change: tally.code
            set: "sign"
            characterStandIn: tallyMark.conflict
            size: tallyCount.font.pixelSize
            wordBase: tallyCount.baselineOffset
            width: tallyMark.wordCell
            height: tallyCount.implicitHeight
        }
        Label {
            id: tallyCount
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
