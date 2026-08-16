pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// One bucket heading, with the empty unstaged heading standing above it
// where that is the seat an empty unstaged bucket would take.
//
// A section with no rows has no heading of its own, so an emptied bucket
// has to be put back by hand — and neither of the two may float to an end
// of the list, because the order is conflicts, then unstaged, then staged
// (デザイン規約 §その他の操作). The seat immediately above STAGED is that
// order's answer for an empty unstaged bucket wherever the conflicts are,
// which is why this rides on the staged heading rather than on the list's
// own header.
Item {
    id: band

    /// Which bucket this heads: `conflicts` / `unstaged` / `staged`.
    /// Named `section` because that is what the list injects.
    required property string section
    required property real listWidth
    required property var repoTab
    required property var workTree
    /// How many rows the whole list has. On a clean tree it is 0 and no
    /// heading stands at all — a lone `(0)` would head nothing.
    required property int total

    /// Which buckets this band is heading, as the scene has it — the
    /// automation walks the list's own children for these rather than
    /// re-deriving the conditions above, which would report the input
    /// (app-ui.md §UI 自動化の因果性).
    readonly property bool headsUnstaged:
        band.visible && (emptyUnstaged.visible || band.section === "unstaged")
    readonly property bool headsStaged:
        band.visible && band.section === "staged"
    /// Automation: press the whole-bucket button of the heading for
    /// `section`, where a hand would press it.
    function moveAll(section) {
        if (section === "unstaged" && emptyUnstaged.visible)
            return emptyUnstaged.moveAll()
        if (section !== band.section)
            return false
        return own.moveAll()
    }

    width: band.listWidth
    // A band the list is not showing takes no room, the way each heading
    // in it does: a hidden item with a height leaves a stripe of ground
    // behind the last row.
    height: band.visible ? emptyUnstaged.height + own.height : 0

    WipBucketHeader {
        id: emptyUnstaged
        section: "unstaged"
        listWidth: band.listWidth
        repoTab: band.repoTab
        workTree: band.workTree
        visible: band.section === "staged" && band.total > 0
                 && band.workTree.unstagedCount
                    + band.workTree.untrackedCount === 0
    }
    WipBucketHeader {
        id: own
        y: emptyUnstaged.height
        section: band.section
        listWidth: band.listWidth
        repoTab: band.repoTab
        workTree: band.workTree
    }
}
