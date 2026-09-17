pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// Where a file list's arrows stand, and when the middle pane is asked to read what they walked onto (規約 §diff のファイル一覧).
//
// Shared by the two lists that open diffs — the commit's CHANGES and the working tree's — which answer the same two
// questions through the same two model slots (`stepFile` / `origOf`). Nothing here draws: every line is a step of the
// walk, a write to the view's position, or the beat that decides when a step is read. Its own visibility is the pane's:
// it sits inside it.
Item {
    id: walk

    /// The lists being walked, top to bottom, each `{ view, model }`. One for a commit's changed files; one per bucket
    /// for the working tree's, which are three lists the arrows cross between (規約 §diff のファイル一覧).
    required property var sides
    /// The one list a walk of one is over — everything that is not the step itself reads the lot of them.
    readonly property var view: walk.sides[0].view
    readonly property var model: walk.sides[0].model
    /// Which of the lists is showing the file the walk is standing on; -1 while none is. Asked of the rows on screen,
    /// because those are what a walk moves over. A walk over one list is over the list it was given — telling them
    /// apart is a question only the working tree's three raise.
    function sideHolding() {
        if (walk.sides.length === 1)
            return 0
        for (let i = 0; i < walk.sides.length; i++)
            if (walk.sides[i].model.rowOfFileIn(walk.atBucket, walk.atPath) >= 0)
                return i
        return -1
    }

    /// The file the middle pane is reading, handed down by the pane. Whoever moved the diff there, this is where the
    /// next walk sets off from — and empty means nothing is being read. The bucket is empty for a commit's changed
    /// files, which sit in none.
    required property string readBucket
    required property string readPath
    /// Where the walk stands: the read file until an arrow moves it, and the row ahead of the reading for as long as a
    /// held key runs (the reading catches up when the hand comes off — `noteStep`).
    property string atBucket: ""
    property string atPath: ""
    /// Where the next walk sets off from: **the file being read** (デザイン規約 §diff のファイル一覧).
    /// Empty with no diff up, and then the arrows move no file — there is no row for a step to be a step from, and the
    /// arrows are not how a first one is opened. Both lists answer the same way because both lights mean the same
    /// thing: the working tree's goes with the reading it was opened by, so no list is left lighting a row the arrows
    /// would have to answer for with nothing on screen (`WipPane.onReadKeyChanged`).
    readonly property string fromKey:
        walk.readPath !== "" ? walk.readBucket + ":" + walk.readPath : ""
    onFromKeyChanged: {
        const cut = walk.fromKey.indexOf(":")
        walk.atBucket = cut < 0 ? "" : walk.fromKey.substring(0, cut)
        walk.atPath = cut < 0 ? "" : walk.fromKey.substring(cut + 1)
    }

    /// The arrows moved onto another file. Raised on every press: the light in the list follows the key at the key's
    /// own rate.
    signal stepped(string bucket, string path)
    /// Read this file. Raised where the run set off and once the hand has come off the key — **twice in a run**:
    /// a step carries a `git diff` with it, and a held arrow repeats faster than that (規約 §diff のファイル一覧, the same beat
    /// as §矢印で履歴を辿る).
    signal landed(string bucket, string path, string origPath)

    /// Moves the walk one file (`way` = ∓1) and answers whether it moved. The keys and the automation hook both come
    /// through here — a headless run cannot inject a keystroke, so the step has to be callable as well as pressable
    /// (verify-ui).
    ///
    /// Refused while the list is off screen: the two file lists swap with each other, and the arrows belong to the
    /// one on screen (規約 §矢印で履歴を辿る「画面から退いたペインはキーボードを手放す」). Refused as well with nothing being read
    /// (`fromKey`) — there is no row for a step to be a step from, and the arrows are not how a first one is opened.
    ///
    /// Answering `false` at either end is how it stops: the model has nowhere to send it and the key
    /// goes unaccepted.
    ///
    /// `held` says the key was already down when this step arrived (`KeyEvent.isAutoRepeat`); it changes nothing about
    /// where the walk goes, only when the file is read (`noteStep`).
    function stepFile(way, held) {
        if (!walk.visible || !walk.view.visible || walk.atPath === "")
            return false
        let side = walk.sideHolding()
        if (side < 0)
            return false
        let record = walk.sides[side].model.stepFile(walk.atBucket, walk.atPath, way)
        // Out of this bucket's list and into the next one's: the heading between them is something walking goes past
        // (規約 §diff のファイル一覧), and so is a bucket holding no files at all.
        while (record === "") {
            side += way < 0 ? -1 : 1
            if (side < 0 || side >= walk.sides.length)
                return false
            record = walk.sides[side].model.edgeFile(way)
        }
        // `<row>\u{1e}<bucket>\u{1e}<path>`, the path last because it is the one field git lets hold the separator.
        const sep = String.fromCharCode(30)
        const first = record.indexOf(sep)
        const second = record.indexOf(sep, first + 1)
        walk.atBucket = record.substring(first + 1, second)
        walk.atPath = record.substring(second + 1)
        // As little as will do — the row stepped onto is brought inside the viewport and nothing else moves. Asked of
        // the view: a file list has folder rows in it that a walk goes past, so the arithmetic the graph's walk does
        // would land on the wrong pixel here. Uncentred: a file list holds no reading position of its own to protect,
        // and the lit row is where the hand just pressed (規約 §diff のファイル一覧).
        walk.sides[side].view.positionViewAtIndex(Number(record.substring(0, first)), ListView.Contain)
        walk.stepped(walk.atBucket, walk.atPath)
        walk.noteStep(held)
        return true
    }
    /// Automation: the name of the row the list is painting as lit, read off the rectangle
    /// (verify-ui). The first of them where several are lit — a walk stands on one, but the working
    /// tree's list can have a whole Ctrl-clicked choice up.
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
    /// Automation: how many of the rows on screen are painted as lit. **The count is the half a name cannot say**: a
    /// list whose only light is the reading wears exactly one while a file is open and none while none is, and a list
    /// lighting all of its rows still answers `litPath` with the right name at the top (verify-ui).
    ///
    /// **Asked of `litNow`.** That key above is the row's own path, which is the very thing such a
    /// list has failed to read — counted through it, a wholly lit list comes to none (observed).
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
    /// Automation: the row for one file, once the list has built it — `itemAtIndex` answers null until then, and a
    /// click aimed at nothing latches a wait that never ends (app-ui.md §UI 自動化の因果性). The bucket tells apart the two
    /// rows of a file changed on both sides; empty matches either, which is every row of a commit's list.
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

    // Let the keyboard go when the list is taken off the screen — Qt leaves active focus on an item it has just made
    // invisible and the keys go on arriving there (規約 §矢印で履歴を辿る, and the hole `DiffRowWalk` closes on the other side of
    // the same swap).
    onVisibleChanged: {
        if (walk.visible)
            return
        for (let side = 0; side < walk.sides.length; side++)
            walk.sides[side].view.focus = false
    }

    /// Books the reading of the file stepped onto. A press is read at once — a single one has to answer inside the
    /// interaction budget — and a step taken with the key still down only pushes the settle back. So a held arrow is
    /// read exactly twice: where it set off, and where it stopped. The shape, the timer and why the repeat has to name
    /// itself are the graph walk's (`GraphRowWalk.noteStep`).
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
    /// Whether a step went by unread: one taken with the key down, or one that came while the settle was running.
    /// Without it the settle behind a single press would read the same file twice.
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
