pragma ComponentBehavior: Bound

import QtQuick

// How a run is handed out among tab names, and nothing else: widths measured elsewhere and a run go in, the strip's
// layout answers come out. No font and no items, so the whole condition table runs in one process
// (`tests/qml/tst_tabwidths.qml`); `tab-widths` covers that real widths go in and the answers reach the tabs.
//
// **The order the strip gives way in**, short of room — it stops short of the grab run (`TabStrip`'s `grabRun`) and
// narrows; narrowed all the way, it scrolls sideways:
//   1. **The room kept for the `✕`** right of the name, by every tab alike, down to the tab's near step; past it name
//      and mark overlap (`TabItemDelegate`'s fade). The tabs thin with every name whole: cutting names first leaves
//      blanks once they can no longer be read. Alike so the marks stand the same distance in from every right edge.
//   2. **The linked worktree's run** (`木 名前`, `splitName`), per tab: the share the name leaves over is that run's;
//      short of it the folder name is cut in the middle, and below the mark's width the run goes whole. It is gone
//      before a letter of the name is cut — a tab is read for which repository; where it stands comes next (user
//      decision — ask before changing). A tab with no such run (in the repository's own worktree) goes from 1 to 3.
//   3. **The names**, the longest first (`widest`): an even split would narrow `ui` too and take width from what has
//      none to give. Those giving way come out at one width; under `TabMetrics.titleMinChars` the strip scrolls.
// The window's floor follows this order: it is the tightest the band packs, so it counts the marks' room at its
// minimum (`TabStrip.tabStripFloorW`). The state badges give way at the same time, not after (`TopBar`'s tab strip).
//
// Owns no number: the costs are `TabMetrics`'s, pushed in.
QtObject {
    id: tabShare

    /// The shortest a name is cut to and the width easing stops at. Read off the font, so pushed
    /// (`TabMetrics.titleEaseFullW`) by `TabMetrics.settle` — production reaches [`settle`] only through it.
    property real minW: 0
    property real easeFullW: 0
    /// `TabMetrics`'s costs; required, so an instance without them fails to load rather than answering off a default.
    required property real padL
    required property real markRoomFull
    required property real markRoomMin
    required property int runParts
    required property real easeShare

    /// The ceiling on one name: a share of the run, cut into at most one part per tab, floored at `minW`
    /// (`TabMetrics.titleRunParts`).
    function ceiling(run, tabs) {
        const parts = Math.max(1, Math.min(tabShare.runParts, tabs))
        return Math.max(tabShare.minW, Math.floor(run / parts))
    }

    /// The air a name of this width is given on top of itself (`TabMetrics.titleEase`).
    function ease(naturalW, cap, easeW) {
        return Math.round(Math.max(0, Math.min(easeW, cap) - naturalW) * tabShare.easeShare)
    }

    /// How much of that air goes on the mark's side (`TabMetrics.easeRight`).
    function easeRight(air, markRoom) {
        return Math.max(air / 2, Math.min(air, tabShare.markRoomFull - markRoom))
    }

    /// How one tab spends the cap between its two runs: the repository's name, then the linked worktree's
    /// (`TabTreeMark`). The name is served first (step 2 of the order in this file's header), so the worktree's run is
    /// gone before a letter of the name is cut.
    ///
    /// Under `treeFloor` (`TabTreeMark.floorWidth`) the run goes whole, mark and all: a worktree name without its mark
    /// reads as the repository's name carrying on.
    ///
    /// A tab that drops the run comes out narrower than its share by under that floor, left as band at the strip's
    /// end. That only happens while `cap` is above the name's own width, so no name is cut for it.
    function splitName(titleNat, treeNat, treeFloor, cap) {
        const titleW = Math.min(titleNat, cap)
        const left = cap - titleW
        const treeW = treeNat > 0 && left >= treeFloor ? Math.min(left, treeNat) : 0
        return { titleW: titleW, treeW: treeW }
    }

    /// The whole pass, in the order the band gives things up (this file's header): the marks' room first, off
    /// every tab at once, then the names, the longest giving way first; below `minW` the strip scrolls.
    ///
    /// `nat` is every name's natural width, in any order. Whole pixels throughout: fractional widths come out a pixel
    /// over the run, and the strip scrolls with nothing out of room.
    ///
    /// `wantNames` is what the names take with nothing cut and the air uncapped (`TabStrip.tabsWantWidth` adds the
    /// furniture). Not capped: the ceiling is a share of the run, so an ask carrying it would move with its answer.
    function settle(nat, run) {
        const maxW = tabShare.ceiling(run, nat.length)
        const easeW = Math.min(tabShare.easeFullW, maxW)
        // The easing's air is a fixed cost, like the mark's room: the cap bites on long names and the air goes to short
        // ones, so costing both off one number would count twice. The pass costs the air at `maxW`, the cap being still
        // unknown here; only the tab clamps it to the cap (`TabItemDelegate.titleEase`).
        let eased = 0
        let names = 0
        let capped = 0
        let wantEase = 0
        for (const w of nat) {
            eased += tabShare.ease(w, maxW, easeW)
            names += w
            capped += Math.min(w, maxW)
            wantEase += tabShare.ease(w, tabShare.easeFullW, tabShare.easeFullW)
        }
        const wantNames = names + wantEase + nat.length * (tabShare.padL + tabShare.markRoomFull)
        if (nat.length === 0) {
            return {
                minW: tabShare.minW,
                maxW: maxW,
                easeW: easeW,
                markRoom: tabShare.markRoomFull,
                cap: maxW,
                wantNames: wantNames
            }
        }
        // Every tab gives up the same mark room (step 1): taking it only from tabs short of run stands the marks at
        // different distances from their edges.
        const room = Math.floor((run - eased - capped - nat.length * tabShare.padL) / nat.length)
        const markRoom = Math.max(tabShare.markRoomMin, Math.min(tabShare.markRoomFull, room))
        // Shared out against the mark room actually given: costing the full room leaves the difference unspent on every
        // tab and cuts names the run had room for.
        const share = tabShare.widest(nat.slice().sort((a, b) => a - b), run, eased, markRoom, maxW)
        return {
            minW: tabShare.minW,
            maxW: maxW,
            easeW: easeW,
            markRoom: markRoom,
            cap: Math.max(tabShare.minW, Math.min(share, maxW)),
            wantNames: wantNames
        }
    }

    /// The widest every name may be drawn with all of them fitting: the max-min share of the run left after the easing
    /// and each tab's `padL` and `markRoom`. The longest give way and come out equal; shorter ones keep their width.
    /// `sorted` is ascending; `ceilingW` answers when all fit, so short names do not set the cap for the rest.
    function widest(sorted, run, eased, markRoom, ceilingW) {
        let left = run - eased - sorted.length * (tabShare.padL + markRoom)
        for (let i = 0; i < sorted.length; i++) {
            const share = Math.floor(left / (sorted.length - i))
            if (sorted[i] > share)
                return share
            left -= sorted[i]
        }
        return ceilingW
    }
}
