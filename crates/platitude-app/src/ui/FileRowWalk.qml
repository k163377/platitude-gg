pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// Where a file list's arrows stand, and when the middle pane is asked to read what they walked onto (規約 §diff のファイル一覧).
// Shared by the commit's CHANGES and the worktree's lists, through the same two model slots (`stepFile` /
// `origOf`). Its visibility is the pane's.
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
    /// commit's changed files.
    required property string readBucket
    required property string readPath
    /// Where the walk stands: the read file until an arrow moves it, and ahead of the reading while a held key runs
    /// (`noteStep`).
    property string atBucket: ""
    property string atPath: ""
    /// Where the next walk sets off from: **the file being read** (デザイン規約 §diff のファイル一覧). Empty with no
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
    /// Read this file — where a held run set off and where the hand came off the key (規約 §diff のファイル一覧).
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
        let landing = walk.sides[side].model.stepFile(walk.atBucket, walk.atPath, way)
        // Into the next bucket's list, past empty buckets (規約 §diff のファイル一覧). Nowhere to land is nothing
        // (`encode::Landing`).
        while (!landing) {
            side += way < 0 ? -1 : 1
            if (side < 0 || side >= walk.sides.length)
                return false
            landing = walk.sides[side].model.edgeFile(way)
        }
        walk.atBucket = landing.bucket
        walk.atPath = landing.path
        // As little as will do, never centred (規約 §diff のファイル一覧). Asked of the view: the graph walk's row
        // arithmetic would miss here, where folder rows are walked past.
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
    /// where it set off and where it stopped (the graph walk's shape, `GraphRowWalk.noteStep`).
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
