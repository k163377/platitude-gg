pragma ComponentBehavior: Bound

import QtQuick

// What a headless run asks of the strip: what the tabs came out as, and the pointer a run does not have. Every answer
// is read off the items the view actually built, so a binding that came apart answers with what is on screen.
QtObject {
    id: probe

    /// The strip's own list, laid out here only on being asked to (`settleStrip`).
    required property ListView view

    /// Makes the strip answer for the model as it stands. A view responds to its model once per frame, so between a
    /// row leaving the model and the next polish `itemAtIndex` still answers by the old indices: with row 0 gone,
    /// index 0 is still the closed tab's item. A reader that walks the items right after a change asks for this
    /// first; a sampler waiting for the strip to change on its own (`tab-drag`) need not.
    function settleStrip() {
        probe.view.forceLayout()
    }

    /// How many rows have an item standing for them — whether the walks below answer for the whole strip or only
    /// the part the layout has caught up with.
    function tabItemCount() {
        let n = 0
        for (let i = 0; i < probe.view.count; i++)
            if (probe.view.itemAtIndex(i))
                n++
        return n
    }

    /// Whether any tab is standing in `path` — titles cannot tell, as every demo worktree has the same name. The
    /// worktree, not the repository: the one that moves when a tab is stood elsewhere (`TabItem.worktree_path`).
    function hasTabPath(path) {
        for (let i = 0; i < probe.view.count; i++) {
            const tab = probe.view.itemAtIndex(i)
            if (tab && tab.worktree_path === path)
                return true
        }
        return false
    }

    function tabPaths() {
        let paths = []
        for (let i = 0; i < probe.view.count; i++) {
            const tab = probe.view.itemAtIndex(i)
            if (tab)
                paths.push(tab.worktree_path)
        }
        return paths.join(",")
    }

    /// What each tab says after its name — the worktree it stands in, or nothing for the repository's own
    /// (`TabItemDelegate.worktree_name`). A picture cannot tell a name dropped for want of room from none, hence
    /// `tabTreeWidths`.
    function tabTrees() {
        let names = []
        for (let i = 0; i < probe.view.count; i++) {
            const tab = probe.view.itemAtIndex(i)
            names.push(tab ? tab.worktree_name : "")
        }
        return names.join(",")
    }

    /// How wide the worktree-name run came out on each tab, in strip order — 0 where none is drawn.
    function tabTreeWidths() {
        let widths = []
        for (let i = 0; i < probe.view.count; i++) {
            const tab = probe.view.itemAtIndex(i)
            widths.push(tab ? Math.round(tab.treeW) : 0)
        }
        return widths.join(",")
    }

    /// The name each tab came out with, in strip order (`PGG_AUTO_ACT=tab-name`).
    function tabTitles() {
        let names = []
        for (let i = 0; i < probe.view.count; i++) {
            const tab = probe.view.itemAtIndex(i)
            names.push(tab ? tab.title : "")
        }
        return names.join(",")
    }

    /// The path a tab was opened with, spelled the way the strip has it (`PGG_AUTO_ACT=open-again`).
    function tabPathAt(index) {
        const tab = probe.view.itemAtIndex(index)
        return tab ? tab.worktree_path : ""
    }

    /// Every tab's width, in strip order (`PGG_AUTO_ACT=tab-widths`).
    function tabWidths() {
        let widths = []
        for (let i = 0; i < probe.view.count; i++) {
            const tab = probe.view.itemAtIndex(i)
            widths.push(tab ? Math.round(tab.width) : 0)
        }
        return widths.join(",")
    }

    /// How many tabs draw none of their own name (`TabItemDelegate.nameKept`). Must be zero: the strip shares out room
    /// down to three characters at each end and scrolls past that (`TabStrip.settleTitleCap`). Counted, because a
    /// crushed name and a short one photograph as the same narrow tab with a mark in it.
    function tabNamesCrushed() {
        let n = 0
        for (let i = 0; i < probe.view.count; i++) {
            const tab = probe.view.itemAtIndex(i)
            if (tab && !tab.nameKept)
                n++
        }
        return n
    }

    /// The pointer, set down on the tab at `index` (`PGG_AUTO_ACT=tab-mark`).
    function pointAtTab(index) {
        const tab = probe.view.itemAtIndex(index)
        if (tab)
            tab.pointed = true
    }

    /// Which tabs have their mark out, in strip order.
    function tabMarks() {
        let marks = []
        for (let i = 0; i < probe.view.count; i++) {
            const tab = probe.view.itemAtIndex(i)
            marks.push(tab ? tab.markShown : 0)
        }
        return marks.join(",")
    }
}
