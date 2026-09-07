pragma ComponentBehavior: Bound

import QtQuick

// What the headless run is allowed to ask of the strip: what the tabs came out as, and the one input a run has no
// pointer for. Every answer is read off the items the view actually built — the output side, so a binding that came
// apart answers with what is on screen rather than with what was asked for (rules-refs/app-ui.md).
//
// Not the strip's own for the reason the hand is not (`TabCarry`): the strip is about laying tabs out, and this is
// about reporting on them. A `QtObject`: nothing here draws, and nothing here needs a child.
QtObject {
    id: probe

    /// The strip's own list. This reads it, and lays it out only on being asked to (`settleStrip`).
    required property ListView view

    /// Makes the strip answer for the model as it stands. A view responds to its model once per frame, so between a
    /// row leaving the model and the next polish `itemAtIndex` still answers by the indices the items had: with row 0
    /// gone, `count` is one and index 0 is still the item of the tab that was closed — which reads as the wrong tab
    /// having gone, on a strip that only needed a frame (measured: `middle-close 0` on the container said
    /// `gone=false` with the closed repository as the one still open). A reader that walks the items right after a
    /// change asks for this first; the samplers that wait for the strip to change on its own (`tab-drag`) need not.
    function settleStrip() {
        probe.view.forceLayout()
    }

    /// How many of the strip's rows have an item standing for them. `tabPaths`, `tabTitles` and `hasTabPath` walk
    /// those items, so this is what says whether their answer is the whole strip or only the part of it the layout has
    /// caught up with.
    function tabItemCount() {
        let n = 0
        for (let i = 0; i < probe.view.count; i++)
            if (probe.view.itemAtIndex(i))
                n++
        return n
    }

    /// Whether any tab in the strip was opened on `path`. The closed tab's title cannot say it went — every demo
    /// working tree is called the same thing — and the path is the only thing that can.
    function hasTabPath(path) {
        for (let i = 0; i < probe.view.count; i++) {
            const tab = probe.view.itemAtIndex(i)
            if (tab && tab.repo_path === path)
                return true
        }
        return false
    }

    /// The paths rather than the titles — every demo repository is called the same thing, and a strip of one name
    /// proves nothing.
    function tabPaths() {
        let paths = []
        for (let i = 0; i < probe.view.count; i++) {
            const tab = probe.view.itemAtIndex(i)
            if (tab)
                paths.push(tab.repo_path)
        }
        return paths.join(",")
    }

    /// The name each tab came out with, in the order they sit in (`PG_AUTO_ACT=tab-name`). Read off the tabs rather
    /// than off the model: what the strip settled on is only worth anything where it is what the strip is drawing.
    function tabTitles() {
        let names = []
        for (let i = 0; i < probe.view.count; i++) {
            const tab = probe.view.itemAtIndex(i)
            names.push(tab ? tab.title : "")
        }
        return names.join(",")
    }

    /// The path a tab was opened with, spelled the way the strip has it (`PG_AUTO_ACT=open-again`).
    function tabPathAt(index) {
        const tab = probe.view.itemAtIndex(index)
        return tab ? tab.repo_path : ""
    }

    /// Every tab's width, in the order they sit in (`PG_AUTO_ACT=tab-widths`).
    function tabWidths() {
        let widths = []
        for (let i = 0; i < probe.view.count; i++) {
            const tab = probe.view.itemAtIndex(i)
            widths.push(tab ? Math.round(tab.width) : 0)
        }
        return widths.join(",")
    }

    /// How many tabs are drawing none of their own name — the mark standing where the name was
    /// (`TabItemDelegate.nameKept`). Nothing but zero is a layout the strip is allowed to reach: it hands the run out
    /// down to a floor of three characters at each end and scrolls rather than cut past it
    /// (`TabStrip.settleTitleCap`). Counted rather than looked at, because the picture of a crushed name and the
    /// picture of a short one are the same narrow tab with a mark in it.
    function tabNamesCrushed() {
        let n = 0
        for (let i = 0; i < probe.view.count; i++) {
            const tab = probe.view.itemAtIndex(i)
            if (tab && !tab.nameKept)
                n++
        }
        return n
    }

    /// The pointer, set down on the tab at `index` — the half no headless run can reach any other way
    /// (`PG_AUTO_ACT=tab-mark`).
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
