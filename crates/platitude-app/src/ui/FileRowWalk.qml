pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// Where a file list's arrows stand, and when the middle pane is asked to
// read what they walked onto (規約 §diff のファイル一覧).
//
// Shared by the two lists that open diffs — the commit's CHANGES and the
// working tree's — which answer the same two questions through the same two
// model slots (`stepFile` / `origOf`). Nothing here draws: every line is a
// step of the walk, a write to the view's position, or the beat that decides
// when a step is read. Its own visibility is the pane's: it sits inside it.
Item {
    id: walk

    /// The list being walked, and the model behind it.
    required property var view
    required property var model

    /// The file the middle pane is reading, handed down by the pane. Whoever
    /// moved the diff there, this is where the next walk sets off from — and
    /// empty means nothing is being read. The bucket is empty for a commit's
    /// changed files, which sit in none.
    required property string readBucket
    required property string readPath
    /// Where the walk stands: the read file until an arrow moves it, and the
    /// row ahead of the reading for as long as a held key runs (the reading
    /// catches up when the hand comes off — `noteStep`).
    property string atBucket: ""
    property string atPath: ""
    readonly property string readKey: walk.readBucket + ":" + walk.readPath
    onReadKeyChanged: {
        walk.atBucket = walk.readBucket
        walk.atPath = walk.readPath
    }

    /// The arrows moved onto another file. Raised on every press: the light
    /// in the list follows the key at the key's own rate.
    signal stepped(string bucket, string path)
    /// Read this file. Raised where the run set off and once the hand has
    /// come off the key — **never once per press**: a step carries a
    /// `git diff` with it, and a held arrow repeats faster than that
    /// (規約 §diff のファイル一覧, the same beat as §矢印で履歴を辿る).
    signal landed(string bucket, string path, string origPath)

    /// Moves the walk one file (`way` = ∓1) and answers whether it moved.
    /// The keys and the automation hook both come through here — a headless
    /// run cannot inject a keystroke, so the step has to be callable as well
    /// as pressable (verify-ui).
    ///
    /// Refused while the list is off screen: the two file lists swap with
    /// each other, and one nobody can see must not answer arrows
    /// (規約 §矢印で履歴を辿る「画面から退いたペインはキーボードを手放す」).
    /// Refused as well with nothing being read — there is no file for a step
    /// to be a step from, and the arrows are not how a first one is opened.
    ///
    /// Answering `false` at either end is how it stops rather than wraps:
    /// the model has nowhere to send it and the key goes unaccepted.
    function stepFile(way) {
        if (!walk.visible || !walk.view.visible || walk.atPath === "")
            return false
        const record = walk.model.stepFile(walk.atBucket, walk.atPath, way)
        if (record === "")
            return false
        // `<row>\u{1e}<bucket>\u{1e}<path>`, the path last because it is the
        // one field git lets hold the separator.
        const sep = String.fromCharCode(30)
        const first = record.indexOf(sep)
        const second = record.indexOf(sep, first + 1)
        walk.atBucket = record.substring(first + 1, second)
        walk.atPath = record.substring(second + 1)
        // As little as will do — the row stepped onto is brought inside the
        // viewport and nothing else moves. Asked of the view rather than
        // worked out from a row height: the working tree's list has bucket
        // headings between its rows, so the arithmetic the graph's walk does
        // would land on the wrong pixel there. No centring case either: a
        // file list holds no reading position of its own to protect, and the
        // lit row is where the hand just pressed (規約 §diff のファイル一覧).
        walk.view.positionViewAtIndex(Number(record.substring(0, first)),
                                      ListView.Contain)
        walk.stepped(walk.atBucket, walk.atPath)
        walk.noteStep()
        return true
    }
    /// Automation: the name of the row the list is painting as lit, read off
    /// the rectangle rather than off the condition behind it (verify-ui).
    /// The first of them where several are lit — a walk stands on one, but
    /// the working tree's list can have a whole Ctrl-clicked choice up.
    function litPath() {
        for (let i = 0; i < walk.view.count; i++) {
            const row = walk.view.itemAtIndex(i)
            if (row && row.litKey !== undefined && row.litKey !== "")
                return row.litKey
        }
        return ""
    }
    /// Automation: the row for one file, once the list has built it —
    /// `itemAtIndex` answers null until then, and a click aimed at nothing
    /// latches a wait that never ends (app-ui.md §UI 自動化の因果性). The
    /// bucket tells apart the two rows of a file changed on both sides; empty
    /// matches either, which is every row of a commit's list.
    function rowFor(bucket, path) {
        for (let i = 0; i < walk.view.count; i++) {
            const row = walk.view.itemAtIndex(i)
            if (row && row.walkKey === path
                    && (bucket === "" || row.bucket === bucket))
                return row
        }
        return null
    }

    // Let the keyboard go when the list is taken off the screen — Qt leaves
    // active focus on an item it has just made invisible and the keys go on
    // arriving there (規約 §矢印で履歴を辿る, and the hole `DiffRowWalk`
    // closes on the other side of the same swap).
    onVisibleChanged: {
        if (!walk.visible)
            walk.view.focus = false
    }

    /// Books the reading of the file stepped onto. The first step of a run is
    /// read at once — a single press has to answer inside the interaction
    /// budget — and the ones behind it only push the settle back. So a held
    /// arrow is read exactly twice: where it set off, and where it stopped.
    /// The shape, and the timer, are the graph walk's (`GraphRowWalk`).
    function noteStep() {
        if (stepTimer.running) {
            walk.stepPending = true
            stepTimer.restart()
            return
        }
        walk.stepPending = false
        walk.landStep()
        stepTimer.restart()
    }
    /// Whether a step went by unread while the settle was running. Without
    /// it the settle behind a single press would read the same file twice.
    property bool stepPending: false
    function landStep() {
        if (walk.atPath !== "")
            walk.landed(walk.atBucket, walk.atPath,
                        walk.model.origOf(walk.atPath))
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
