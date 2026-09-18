pragma ComponentBehavior: Bound

import QtQuick

// How a run is handed out among tab names — and nothing else. No font, no items, no window: what comes in is a list
// of widths somebody else measured and the run they are to fit in, and what goes out is the two answers the strip
// lays out by (デザイン規約 §ウィンドウの縁 の譲る順).
//
// **Split off from the measuring for what it lets be asked.** A strip that narrowed the wrong tabs has to be caught
// at a count where the run makes it narrow them, so every case used to cost a window and a repository per tab; here
// the run is a number, so the whole table — none, one, a mixture, the floor, the ceiling, either side of each
// boundary — is asked in one process (`tests/qml/tst_tabwidths.qml`). What a window is still needed for is the
// other half: that the widths going in are a real font's and the answers coming out reach the tabs
// (`PGG_AUTO_ACT=tab-widths`).
//
// The costs are `TabMetrics`'s to measure and are pushed in, not read from here: this object owns no number of its
// own, so the one copy of each stays where the reasoning for it is written down.
QtObject {
    id: tabShare

    /// The shortest a name is ever cut to, and the length it stops being helped along at — both read off the font
    /// (`TabMetrics.titleMinW` / `titleEaseFullW`), so both are **pushed** rather than bound: `advanceWidth` is a
    /// method and a binding on one freezes at the default font's answer (rules-refs/app-ui.md §FontMetrics).
    /// `TabMetrics.settle` is what pushes them, which is why nothing else may call [`settle`] directly.
    property real minW: 0
    property real easeFullW: 0
    /// What a tab spends either side of its name, and the ways the run is cut for one name's ceiling — the same
    /// numbers `TabMetrics` holds, required so that an instance standing without them fails to load rather than
    /// answering off a default nobody chose.
    required property real padL
    required property real markRoomFull
    required property real markRoomMin
    required property int runParts
    required property real easeShare

    /// The ceiling on one name: that share of the run, cut into at most one part per tab, and floored at the width a
    /// name still says something at (`TabMetrics.titleRunParts` carries the reasoning).
    function ceiling(run, tabs) {
        const parts = Math.max(1, Math.min(tabShare.runParts, tabs))
        return Math.max(tabShare.minW, Math.floor(run / parts))
    }

    /// The air a name of this width is given on top of itself — half of what it falls short by, held to the cap
    /// (`TabMetrics.titleEase`).
    function ease(naturalW, cap, easeW) {
        return Math.round(Math.max(0, Math.min(easeW, cap) - naturalW) * tabShare.easeShare)
    }

    /// Which side of that air is set down on the mark's (`TabMetrics.easeRight`).
    function easeRight(air, markRoom) {
        return Math.max(air / 2, Math.min(air, tabShare.markRoomFull - markRoom))
    }

    /// The whole pass, in the order the band gives things up (デザイン規約 §ウィンドウの縁): the room the marks stand
    /// in first — all of it, off every tab at once — and only then the names, the longest giving way last; below
    /// `minW` the strip scrolls.
    ///
    /// `nat` is every name at its natural width, in any order (this makes its own sorted copy — the caller's list is
    /// read in the strip's own order afterwards). Whole pixels throughout: a strip sized off fractional widths comes
    /// out a pixel over the run it was told to fit in, which is a strip that scrolls when nothing is out of room.
    ///
    /// `wantNames` is what the names would take with nothing cut and nothing capping the air either — the strip adds
    /// its own furniture to it and asks for the sum (`TabStrip.tabsWantWidth`). Asked at the names' own width because
    /// the ceiling is a share of the run, so an ask carrying it would be read out of the run that ask is about to be
    /// answered with.
    function settle(nat, run) {
        const maxW = tabShare.ceiling(run, nat.length)
        const easeW = Math.min(tabShare.easeFullW, maxW)
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
        // The room the marks stand in is what a crowded strip takes back first, and **every tab gives up the same
        // amount of it** (同§): taking it from the tabs that are short of run would stand the marks at a different
        // distance from each tab's edge, which is a row of marks nobody lined up.
        const room = Math.floor((run - eased - capped - nat.length * tabShare.padL) / nat.length)
        const markRoom = Math.max(tabShare.markRoomMin, Math.min(tabShare.markRoomFull, room))
        // What is left over once the marks have stood down as far as this run makes them. Against the room they
        // actually got: costed at the full room while standing in less, the strip leaves the difference on every tab
        // unspent and cuts names it had the run for (measured).
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

    /// The widest every name may be drawn and still leave room for all of them: the max-min share of what the run has
    /// left once each tab has its near step and `markRoom` for its mark. The longest names give way and come out
    /// equal; the ones already under their share keep their own width (同§). `sorted` comes in ascending, and
    /// `ceilingW` is the answer when they all fit — the ceiling is what caps the strip, or a tab whose name happens
    /// to be short would cap the ones beside it.
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
