pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// Where a file list's arrows stand, and when the middle pane is asked to read what they walked onto (the arrows' shared
// rules: 規約 §矢印で履歴を辿る). Shared by the commit's CHANGES and the worktree's lists, through the same two model
// slots (`stepFile` / `origOf`). Its visibility is the pane's.
//
// Both lists answer alike (user decision — ask before changing): the same gesture answering differently per surface
// makes the reader learn a rule per surface. The file being read lights its row (`FileRowDelegate`), and Up / Down
// move that light, the diff following, as the graph's arrows move the selection the right pane follows (user
// decision). Only the file being read moves (user decision): the light means "being read", so with the diff closed
// there is no row to move in either list. Folder rows are stepped over (no diff to read), and only rows on screen are
// walked — not inside a closed folder, not what a filter hides (`NavSectionModel::step`). The worktree's buckets are
// one line to the arrows, conflicts → unstaged → staged, empty ones passed by. The keyboard stays where the press
// landed: a hand that pressed a row is in that list (規約 §矢印で履歴を辿る「最後に触った場所が居場所」).
Item {
    id: walk

    /// The lists being walked, top to bottom, each `{ view, model }`: one for a commit's changed files, one per bucket
    /// for the worktree's.
    required property var sides
    readonly property var view: walk.sides[0].view
    readonly property var model: walk.sides[0].model
    /// Which of the lists holds the file the walk is standing on, -1 while none does.
    function sideHolding() {
        if (walk.sides.length === 1)
            return 0
        for (let i = 0; i < walk.sides.length; i++)
            if (walk.sides[i].model.rowOfFileIn(walk.atBucket, walk.atPath) >= 0)
                return i
        return -1
    }

    /// The file the middle pane is reading, handed down by the pane; empty when nothing is. The bucket is empty for a
    /// commit's changed files. Handed in, not bound to `atBucket` / `atPath`, which this walk writes: a binding there
    /// would break on the first step.
    required property string readBucket
    required property string readPath
    /// Where the walk stands: the read file until an arrow moves it, and ahead of the reading while a held key runs
    /// (`noteStep`).
    property string atBucket: ""
    property string atPath: ""
    /// Where the next walk sets off from: **the file being read** (the header says why). Empty with no
    /// diff up, and then the arrows move no file.
    readonly property string fromKey:
        walk.readPath !== "" ? walk.readBucket + ":" + walk.readPath : ""
    onFromKeyChanged: {
        const cut = walk.fromKey.indexOf(":")
        walk.atBucket = cut < 0 ? "" : walk.fromKey.substring(0, cut)
        walk.atPath = cut < 0 ? "" : walk.fromKey.substring(cut + 1)
    }

    /// The arrows moved onto another file — every press, so the light follows the key at its own rate.
    signal stepped(string bucket, string path)
    /// Read this file — where a held run set off and where the hand came off the key: each step brings a `git diff`
    /// (`keyStepSettleMs`, 規約 §矢印で履歴を辿る).
    signal landed(string bucket, string path, string origPath)

    /// Moves the walk one file (`way` = ∓1) and answers whether it moved — `false` at either end leaves the key
    /// unaccepted. The keys and the automation hook both come through here (verify-ui). Refused off screen
    /// (規約 §矢印で履歴を辿る「画面から退いたペインはキーボードを手放す」) and with nothing being read (`fromKey`).
    ///
    /// `held` (`KeyEvent.isAutoRepeat`) changes only when the file is read (`noteStep`).
    function stepFile(way, held) {
        if (!walk.visible || !walk.view.visible || walk.atPath === "")
            return false
        let side = walk.sideHolding()
        if (side < 0)
            return false
        // The next file is the model's answer, never the view's: a `ListView` holds no off-screen rows.
        let landing = walk.sides[side].model.stepFile(walk.atBucket, walk.atPath, way)
        // Into the next bucket's list, past empty buckets (the header). Nowhere to land is nothing
        // (`encode::Landing`).
        while (!landing) {
            side += way < 0 ? -1 : 1
            if (side < 0 || side >= walk.sides.length)
                return false
            landing = walk.sides[side].model.edgeFile(way)
        }
        walk.atBucket = landing.bucket
        walk.atPath = landing.path
        // As little as will do, never centred: the lit row is where the hand just pressed, unlike the graph's walk from
        // a row out of sight (規約 §矢印で履歴を辿る). Asked of the view: the graph walk's row
        // arithmetic (row height × row) would miss here, where folder rows are walked past and the worktree's lists
        // stand bucket bands between their rows.
        walk.sides[side].view.positionViewAtIndex(landing.row, ListView.Contain)
        walk.stepped(walk.atBucket, walk.atPath)
        walk.noteStep(held)
        return true
    }
    /// Automation: the name of the first row the list paints as lit (a Ctrl-clicked choice can light several).
    function litPath() {
        for (let side = 0; side < walk.sides.length; side++) {
            const view = walk.sides[side].view
            for (let i = 0; i < view.count; i++) {
                const row = view.itemAtIndex(i)
                if (row && row.litKey !== undefined && row.litKey !== "")
                    return row.litKey
            }
        }
        return ""
    }
    /// Automation: how many rows on screen are painted as lit — a wholly lit list still answers `litPath` with the
    /// right name at the top. **Asked of `litNow`**, not `litKey`: a list lit throughout because its rows lost their
    /// path counts none through the key.
    function litRows() {
        let lit = 0
        for (let side = 0; side < walk.sides.length; side++) {
            const view = walk.sides[side].view
            for (let i = 0; i < view.count; i++) {
                const row = view.itemAtIndex(i)
                if (row && row.litNow === true)
                    lit++
            }
        }
        return lit
    }
    /// Automation: the row for one file once the list has built it, else null. The bucket tells apart the two rows of
    /// a file changed on both sides; empty matches either.
    function rowFor(bucket, path) {
        for (let side = 0; side < walk.sides.length; side++) {
            const view = walk.sides[side].view
            for (let i = 0; i < view.count; i++) {
                const row = view.itemAtIndex(i)
                if (row && row.walkKey === path && (bucket === "" || row.bucket === bucket))
                    return row
            }
        }
        return null
    }

    // Let the keyboard go off screen — Qt leaves active focus on an item it has just made invisible (規約 §矢印で履歴を辿る).
    onVisibleChanged: {
        if (walk.visible)
            return
        for (let side = 0; side < walk.sides.length; side++)
            walk.sides[side].view.focus = false
    }

    /// A press is read at once; a step with the key still down only pushes the settle back — so a held arrow is read
    /// where it set off and where it stopped (the graph walk's shape, `GraphRowWalk.noteStep`): each read is one
    /// `git diff`. Read through `landed`, its own signal and not the click's: a click is a toggle (`fileActivated` →
    /// `RepoPage.toggleDiff`), and the settle's second read would close the diff.
    function noteStep(held) {
        if (held || stepTimer.running) {
            walk.stepPending = true
            stepTimer.restart()
            return
        }
        walk.stepPending = false
        walk.landStep()
        stepTimer.restart()
    }
    /// A step went by unread — without it the settle behind a single press would read the same file twice.
    property bool stepPending: false
    function landStep() {
        if (walk.atPath !== "")
            walk.landed(walk.atBucket, walk.atPath, walk.model.origOf(walk.atPath))
    }
    Timer {
        id: stepTimer
        interval: Metrics.keyStepSettleMs
        onTriggered: {
            if (!walk.stepPending)
                return
            walk.stepPending = false
            walk.landStep()
        }
    }
}
