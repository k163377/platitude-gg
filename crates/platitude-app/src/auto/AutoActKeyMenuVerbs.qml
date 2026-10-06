pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// The menu key on each surface that answers it (デザイン規約 §メニュー のキーボード). The keyboard goes there by the
/// door a hand's press takes, then the run enters the key's own handler — or, on the graph, Qt's request: with no place
/// (Windows' Shift+F10), or with one (xcb's menu key at the pointer, which the window turns away). Each says whether a
/// menu stood, and whether it stood under its target's left end. Built by `AutoActDriver`.
// `Item` because `QtObject` has no default property to hold the timer below.
Item {
    id: acts

    /// `var`: naming its type would be a circle — the driver is the file that builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var graphModel: driver.graphModel
    readonly property var workingTree: driver.workingTree
    readonly property var unstagedModel: driver.unstagedModel
    readonly property var graphPane: driver.graphPane
    readonly property var wipPane: driver.wipPane
    readonly property var diffPane: driver.diffPane
    readonly property var navProbe: driver.navProbe

    /// Runs `act` if it is this family's, and says whether it was. `key-menu <surface>`: `graph` (the row under
    /// HEAD's), `graph:request` / `graph:pointer` (Qt's request instead of the key), `files:<path>` (an unstaged file's
    /// row), `diff:<path>` (a drag over that file's unstaged diff), `nav:<kind>:<name>` (a row of the left menu).
    function run(act, arg) {
        if (act !== "key-menu")
            return false
        const cut = arg.indexOf(":")
        keyMenuTimer.surface = cut > 0 ? arg.substring(0, cut) : arg
        keyMenuTimer.rest = cut > 0 ? arg.substring(cut + 1) : ""
        keyMenuTimer.begin()
        return true
    }

    // ---- the press that brings the keyboard ------------------------------------------------------------------------
    /// Puts the press in, once what it lands on is there; says whether it went in.
    function press(surface, rest) {
        if (surface === "graph") {
            acts.page.activateRow(acts.graphModel.oidAt(acts.headRow() + 1))
            // The door every press inside the pane takes (`GraphList.takeKeyboard`).
            acts.graphPane.takeKeyboard()
            return true
        }
        if (surface === "files") {
            acts.page.showWip()
            const row = acts.wipPane.filesWalk.rowFor("unstaged", rest)
            if (!row)
                return false
            // The row's own click: it chooses the file and brings the keyboard (`WipBucketPane`).
            row.leftClick(Qt.NoModifier)
            return true
        }
        if (surface === "diff")
            return acts.pressDiff(rest)
        if (surface === "nav") {
            const list = acts.navList(rest)
            const index = acts.navIndex(rest)
            if (!list || index < 0)
                return false
            list.positionViewAtIndex(index, ListView.Contain)
            list.forceLayout()
            // The row's own click (`NavList.clickRow`), which brings the keyboard (`NavList.takeKeyboard`).
            return list.clickRow(index)
        }
        Harness.report("key_menu surface=" + surface + " unknown")
        return true
    }
    function headRow() {
        return acts.graphModel.rowOf(acts.workingTree.headOid)
    }
    /// The diff opens first, then a drag from the head of the first line past the first removed one, then the hand
    /// arrives on the text (`DiffPane.handArrived`, the wheel's and a press's door for the keyboard).
    property bool diffAsked: false
    property int diffLastRow: -1
    function pressDiff(path) {
        if (!acts.diffAsked) {
            acts.page.showWip()
            acts.page.toggleDiff("unstaged", path, acts.unstagedModel.origOf(path))
            acts.diffAsked = true
            return false
        }
        // The rows built, too: the first removed line is looked for among them (`DiffPane.firstRemovedRow`).
        if (acts.diffPane.diffModel.loading || acts.diffPane.view.count < 3 || !acts.diffPane.view.itemAtIndex(2))
            return false
        // Past the first removed line where there is one, so the selection holds both sides; two rows otherwise.
        const removed = acts.diffPane.firstRemovedRow()
        acts.diffLastRow = Math.min(acts.diffPane.view.count - 1, removed >= 0 ? removed + 1 : 2)
        acts.diffPane.pickText(0, 1, 0, acts.diffLastRow, acts.driver.pastLineEnd)
        acts.diffPane.handArrived()
        return true
    }
    /// `<kind>:<name>`: the section's list, and the row showing that name.
    function navList(rest) {
        const cut = rest.indexOf(":")
        return cut > 0 ? acts.navProbe.listOf(rest.substring(0, cut)) : null
    }
    function navIndex(rest) {
        const list = acts.navList(rest)
        return list ? list.sectionModel.rowOfName(rest.substring(rest.indexOf(":") + 1)) : -1
    }

    // ---- the keyboard arrived ------------------------------------------------------------------------------------
    function landed(surface, rest) {
        if (surface === "graph")
            return acts.graphPane.view.activeFocus && acts.graphPane.view.currentIndex === acts.headRow() + 1
        if (surface === "files") {
            const row = acts.wipPane.filesWalk.rowFor("unstaged", rest)
            // The click also opened the file's diff (`RepoPage.toggleDiff`), a git subprocess away: what the run
            // photographs and records is the pane with its rows (`DiffPane.diffSettled`).
            return Awaited.all("key_menu", {
                "keyboard": !!row && row.ListView.view.activeFocus,
                "diff": acts.page.diffKey === "unstaged:" + rest && acts.diffPane.diffSettled()
            })
        }
        if (surface === "diff")
            return acts.diffPane.view.activeFocus && acts.diffPane.diffModel.selHasNew
        if (surface === "nav") {
            const list = acts.navList(rest)
            return !!list && list.activeFocus
        }
        return true
    }

    // ---- the key ------------------------------------------------------------------------------------------------
    /// Asks, the way `rest` names on the graph (`key`, `request`, `pointer`) and through the key elsewhere, and reads
    /// where the menu stood against the foot of the target's left end. The target is read after the asking: the
    /// surface brings it on screen first.
    function ask(surface, rest) {
        let via = "key"
        let menu = null
        let foot = null
        if (surface === "graph") {
            const view = acts.graphPane.view
            via = rest === "" ? "key" : rest
            if (via === "key") {
                acts.graphPane.menuFromKeys()
            } else {
                // The window's own handler, which every request reaches that nothing above took
                // (`Main.keyMenuAsked`): the window's origin for one with no place, the pointer's point for xcb's —
                // here the middle of the list.
                view.Window.window.keyMenuAsked(via === "request" ? Qt.point(0, 0)
                                                : view.mapToItem(null, view.width / 2, view.height / 2))
            }
            menu = acts.driver.commitMenu
            const item = view.itemAtIndex(view.currentIndex)
            foot = item ? item.mapToItem(null, 0, Theme.graphRowHeight) : null
        } else if (surface === "files") {
            acts.wipPane.menuFromKeys()
            menu = acts.driver.fileMenu
            const row = acts.wipPane.filesWalk.rowFor("unstaged", rest)
            foot = row ? row.mapToItem(null, 0, Theme.rowHeight) : null
        } else if (surface === "diff") {
            acts.diffPane.menuFromKeys()
            menu = acts.driver.diffRowMenu.menu
            // The drag ended on its last row, past the line's end: the foot of that row, wherever along it.
            const item = acts.diffPane.view.itemAtIndex(acts.diffLastRow)
            foot = item ? item.mapToItem(null, 0, item.height) : null
        } else if (surface === "nav") {
            const list = acts.navList(rest)
            list.menuFromKeys()
            menu = acts.driver.refMenu
            const item = list.itemAtIndex(acts.navIndex(rest))
            foot = item ? item.mapToItem(null, 0, item.lineHeight) : null
        }
        const opened = menu !== null && menu.visible
        const at = opened ? menu.parent.mapToItem(null, menu.x, menu.y) : null
        // The diff's x is the end's place along the line, read off the layout; only its row is the run's to know, and
        // that the card stands right of the row's head.
        const under = opened && foot !== null && Math.abs(at.y - foot.y) <= 1
                      && (surface === "diff" ? at.x > foot.x : Math.abs(at.x - foot.x) <= 1)
        Harness.report("key_menu on=" + surface + " via=" + via + " opened=" + opened + " under=" + under
                          + " at=" + (at ? Math.round(at.x) + "," + Math.round(at.y) : "none")
                          + " foot=" + (foot ? Math.round(foot.x) + "," + Math.round(foot.y) : "none"))
    }

    // Presses, waits for the keyboard to land (and on the file list for the diff the click opened), then asks. No
    // ceiling: a target that never lands is ended by the watchdog.
    SampleTimer {
        id: keyMenuTimer
        property string surface: ""
        property string rest: ""
        property bool pressed: false
        function begin() {
            keyMenuTimer.pressed = false
            acts.diffAsked = false
            keyMenuTimer.start()
        }
        onTriggered: {
            if (!keyMenuTimer.pressed) {
                keyMenuTimer.pressed = acts.press(keyMenuTimer.surface, keyMenuTimer.rest)
                return
            }
            if (!acts.landed(keyMenuTimer.surface, keyMenuTimer.rest))
                return
            keyMenuTimer.stop()
            acts.ask(keyMenuTimer.surface, keyMenuTimer.rest)
            acts.driver.complete()
        }
    }
}
