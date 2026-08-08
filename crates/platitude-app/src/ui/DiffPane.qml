pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// Center pane, diff mode: one file's unified diff with per-file,
// per-hunk and per-line staging affordances, plus image / binary
// previews. Which file is shown and what staging means is the owner's
// business — this pane reports intents.
Rectangle {
    id: diffPane

    required property var diffModel
    // Whether the shown diff is a working-tree file (stageable).
    property bool fromWorkTree: false
    // Whether it is the staged side (flips the affordance wording).
    property bool staged: false
    /// Whether the file is one git stopped on. `git add` still runs, but
    /// what it means there is "this is dealt with", not "this goes in the
    /// next commit", and the words follow that.
    property bool conflicted: false
    // A write is running: staging buttons disable.
    property bool busy: false
    /// Whether this diff has pieces worth naming. A file the repository is
    /// seeing for the first time has none: its one hunk is the whole file,
    /// so `Stage hunk` would be the header's `Stage file` said a second
    /// time in a smaller voice, and the `+` on every row would be that
    /// same word again once per line (デザイン規約 §diff の中のステージ).
    /// What stays is the one word in the header.
    readonly property bool partial: diffPane.fromWorkTree
                                    && !diffPane.diffModel.isNewFile

    signal closeRequested()
    /// The side being read has nothing left in it — everything that was
    /// here has been staged (or unstaged, or thrown away), so what the
    /// pane is standing on is gone.
    signal nothingLeft()
    /// Stage or unstage the whole file (direction follows `staged`).
    signal stageFileRequested()
    /// Stage or unstage one hunk (line < 0) or one line of it.
    signal stageSelectionRequested(int hunk, int line)
    /// Stage or unstage the lines picked by hand — all of them, in one
    /// write. The owner reads them back with `chosenPairs()`.
    signal stageChosenRequested()

    // ---- the lines picked by hand ----------------------------------
    // A click on a changed line takes it or puts it back; the heading of
    // the hunk it belongs to then names the count instead of the hunk
    // (デザイン規約 §diff の中のステージ). Held here rather than on the
    // rows because a delegate is recycled the moment its line scrolls off,
    // and keyed `<hunk>:<line>` because that pair is what a patch is
    // addressed by.
    property var chosenLines: ({})
    property int chosenCount: 0
    function lineChosen(hunk, line) {
        return diffPane.chosenLines[hunk + ":" + line] === true
    }
    /// How many lines of one hunk are picked — what its heading says.
    function chosenIn(hunk) {
        const head = hunk + ":"
        let n = 0
        for (const key in diffPane.chosenLines)
            if (key.indexOf(head) === 0)
                n++
        return n
    }
    function toggleLine(hunk, line) {
        // A fresh object every time: the rows follow this property, and
        // assigning the same one back changes nothing to follow.
        const key = hunk + ":" + line
        const next = ({})
        for (const k in diffPane.chosenLines)
            next[k] = true
        if (next[key] === true)
            delete next[key]
        else
            next[key] = true
        diffPane.chosenLines = next
        diffPane.chosenCount = Object.keys(next).length
    }
    function clearLines() {
        diffPane.chosenLines = ({})
        diffPane.chosenCount = 0
    }
    /// The picked lines as `[hunk, line]` pairs, in the order a patch
    /// wants them — for the owner to hand to the bridge.
    function chosenPairs() {
        const out = []
        for (const key in diffPane.chosenLines) {
            const cut = key.indexOf(":")
            out.push([parseInt(key.substring(0, cut)),
                      parseInt(key.substring(cut + 1))])
        }
        out.sort((a, b) => a[0] === b[0] ? a[1] - b[1] : a[0] - b[0])
        return out
    }
    /// Throw one hunk away. Offered on the unstaged side only — the
    /// staged side unstages first. No question comes before it: the
    /// heading's own button is held down (デザイン規約 §その他の操作).
    signal discardHunkRequested(int hunk)

    /// Automation: hold the heading's discard button to its end. The
    /// hunk is the first one, which is the one every smoke run acts on.
    function completeHold() {
        const row = diffList.itemAtIndex(0)
        if (row && row.discardButton)
            row.discardButton.completeHold()
    }

    // ---- automation: the tools a line only shows under the pointer ----
    // Hover cannot be injected on Windows (CLAUDE.md), so the squares a
    // line puts out — stage this line, throw it away — have no headless
    // way to be seen. These name a row as though the pointer were on it,
    // the way `ref-list` enters where the hover timer would. Nothing in
    // the app writes them: the pointer is the only other way in.
    property int hoverHunk: -1
    property int hoverLine: -1
    function showLineTools(hunk, line) {
        diffPane.hoverHunk = hunk
        diffPane.hoverLine = line
    }
    /// Automation: pick the first `count` changed lines of a hunk, the way
    /// a click on each of them would.
    function chooseLines(hunk, count) {
        diffPane.clearLines()
        let taken = 0
        for (let i = 0; i < diffList.count && taken < count; i++) {
            const row = diffList.itemAtIndex(i)
            if (row && row.hunk === hunk
                    && (row.kind === "add" || row.kind === "del")) {
                diffPane.toggleLine(hunk, row.line)
                taken++
            }
        }
        return taken
    }

    // ---- the view's place in a diff that is about to be rebuilt -------
    // A partial write ends by reading the file again, and the answer
    // arrives as a whole new list. Without this the view would come back
    // at the top, which on a long diff loses the place being worked
    // through — the same restore `GraphPane.shiftRows` does after a graph
    // swap, and delayed for the same reason (`contentHeight` is still the
    // old one on the frame the rows land).
    property real heldY: -1
    function holdScroll() {
        diffPane.heldY = diffList.contentY
    }
    function restoreScroll() {
        if (diffPane.heldY >= 0)
            placeTimer.restart()
    }
    // The wait is the graph's (`Metrics.anchorDelayMs`, and `shiftRows`
    // learned it the same way): on the frame the rows land the list has
    // not laid them out yet, so `contentHeight` is still the old one and
    // the clamp below would take the view to the top instead of back to
    // its place.
    Timer {
        id: placeTimer
        interval: Metrics.anchorDelayMs
        onTriggered: {
            const want = diffPane.heldY
            diffPane.heldY = -1
            diffList.contentY = Math.max(0, Math.min(want, diffList.contentHeight
                                                     - diffList.height))
            if (AppBackend.autoAct !== "")
                AppBackend.report("diff_place " + Math.round(diffList.contentY))
        }
    }
    /// Automation: read the view away from the top, so that a rebuild can
    /// be seen to put it back where it was.
    function scrollTo(y) {
        diffList.contentY = y
    }

    // ---- the side that ran out --------------------------------------
    // A write empties the diff it was made in as soon as the last of it
    // goes over to the other side, and an empty frame with a live
    // `Stage file` in it is a pane standing on nothing. Only a rebuild
    // asks the question — opening a file that has nothing to show is a
    // different story and not one a close would explain.
    property bool closeWhenEmpty: false
    Connections {
        target: diffPane.diffModel
        // Sent once the swap is complete, so the count is the new one.
        function onChanged() {
            if (!diffPane.closeWhenEmpty || diffPane.diffModel.loading)
                return
            diffPane.closeWhenEmpty = false
            // A picture is content even where there are no rows to count.
            if (diffList.count === 0 && !diffPane.diffModel.isBinary
                    && diffPane.diffModel.previewKind === "")
                diffPane.nothingLeft()
        }
    }
    /// The first line of a hunk that a partial write can act on. A hunk's
    /// lines are numbered through the context it carries, so line 0 is
    /// usually a line that is not part of the change at all — selecting
    /// one builds a patch with nothing in it, which core refuses ("the
    /// selected part is no longer in its diff"). A pointer never has this
    /// problem: it picks its line by being over it, and the squares only
    /// come out on the lines that changed. -1 when the hunk has none.
    function firstChangedLine(hunk) {
        for (let i = 0; i < diffList.count; i++) {
            const row = diffList.itemAtIndex(i)
            if (row && row.hunk === hunk
                    && (row.kind === "add" || row.kind === "del"))
                return row.line
        }
        return -1
    }

    color: Theme.bgSurface
    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: Theme.headerHeight
            color: Theme.bgElevated
            // The hairline every pane header closes with (see PaneHeader).
            // This is the band it was written for: the row under it is a
            // hunk heading of the same colour.
            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: Theme.borderWidth
                color: Theme.borderSubtle
                z: 1
            }
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: Theme.spaceSm
                anchors.rightMargin: Theme.spaceSm
                spacing: Theme.spaceSm
                Label {
                    text: qsTr("DIFF · %1").arg(diffPane.diffModel.title)
                    font.pixelSize: Theme.fontSm
                    font.weight: Font.DemiBold
                    color: Theme.textSecondary
                    elide: Text.ElideMiddle
                    Layout.fillWidth: true
                }
                // The file's own word, and the loudest thing in the pane:
                // the pair colour the hunks and the file rows use, a step
                // up in size, and lit whether or not the pointer is near
                // (デザイン規約 §diff の中のステージ). It is the only
                // standing colour word in the view — the hunks' wait for
                // the pointer — which is what puts the scopes back in
                // order: staging a file is the larger of the two.
                ActionButton {
                    visible: diffPane.fromWorkTree
                    // The same `git add` on a conflicted file is not a
                    // staging at all — it is how git is told the conflict
                    // has been dealt with, so the word says that instead
                    // (デザイン規約 §diff の中のステージ).
                    text: diffPane.conflicted ? qsTr("Mark resolved")
                          : diffPane.staged ? qsTr("Unstage file")
                                            : qsTr("Stage file")
                    tone: diffPane.staged ? Theme.diffRemovedFg
                                          : Theme.diffAddedFg
                    enabled: !diffPane.busy
                    ToolTip.visible: hovered
                    ToolTip.delay: Metrics.tipDelayMs
                    ToolTip.text: diffPane.conflicted
                        ? qsTr("Takes the file as it stands now")
                        : diffPane.staged
                        ? qsTr("Unstage the whole file at once")
                        : qsTr("Stage the whole file at once")
                    onActivated: diffPane.stageFileRequested()
                }
                HoverToolButton {
                    implicitWidth: Theme.iconLg
                    implicitHeight: Theme.iconLg
                    padding: 0
                    contentItem: Item {
                        NavIcon {
                            anchors.centerIn: parent
                            width: Theme.iconMd
                            height: Theme.iconMd
                            kind: "close"
                            tint: Theme.textPrimary
                        }
                    }
                    onClicked: diffPane.closeRequested()
                }
            }
        }
        // No question bar here: the only thing this pane throws away is a
        // hunk, and that is held down on the hunk's own heading
        // (デザイン規約 §その他の操作).
        // -- content preview: binaries summarized by size, images
        //    rendered (added = After only, deleted = Before only,
        //    modified = both).
        Label {
            visible: diffPane.diffModel.previewKind === "binary"
                     || (diffPane.diffModel.isBinary
                         && diffPane.diffModel.previewKind === "")
            Layout.margins: Theme.spaceSm
            Layout.fillWidth: true
            elide: Text.ElideRight
            text: {
                const oldS = diffPane.diffModel.previewOldSize
                const newS = diffPane.diffModel.previewNewSize
                if (oldS !== "" && newS !== "")
                    return qsTr("Binary file · %1 → %2").arg(oldS).arg(newS)
                if (newS !== "")
                    return qsTr("Binary file · %1").arg(newS)
                if (oldS !== "")
                    return qsTr("Binary file removed · was %1").arg(oldS)
                return qsTr("Binary file — no text diff")
            }
            color: Theme.textMuted
        }
        RowLayout {
            visible: diffPane.diffModel.previewKind === "image"
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: Theme.spaceSm
            spacing: Theme.spaceSm
            ImagePreviewCell {
                Layout.fillWidth: true
                Layout.fillHeight: true
                label: qsTr("Before · %1").arg(diffPane.diffModel.previewOldSize)
                url: diffPane.diffModel.previewOldUrl
                sizeText: diffPane.diffModel.previewOldSize
            }
            ImagePreviewCell {
                Layout.fillWidth: true
                Layout.fillHeight: true
                label: qsTr("After · %1").arg(diffPane.diffModel.previewNewSize)
                url: diffPane.diffModel.previewNewUrl
                sizeText: diffPane.diffModel.previewNewSize
            }
        }
        ListView {
            id: diffList
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: diffPane.diffModel
            reuseItems: true
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: AutoScrollBar {}
            // The rows the write asked for have landed: put the view back
            // where it was reading. The empty half of the swap is not it —
            // a reset shows up here as a count of zero first.
            onCountChanged: if (count > 0) diffPane.restoreScroll()
            // An image with no text rows hands its space to the preview
            // (SVG edits keep both).
            visible: diffPane.diffModel.previewKind !== "image"
                     || count > 0
            delegate: Rectangle {
                id: diffRow
                required property string kind
                required property int old_no
                required property int new_no
                required property string text
                required property int hunk
                required property int line
                width: diffList.width
                height: Theme.rowHeight
                color: kind === "add" ? Theme.diffAddedBg
                       : kind === "del" ? Theme.diffRemovedBg
                       : kind === "hunk" ? Theme.diffHunkHeaderBg
                       : "transparent"
                // Nothing marks a row any more: what a hunk's discard
                // takes is the hunk its heading sits on, and the button is
                // in that heading. Reached from outside for the smoke run,
                // which holds it the way a hand does.
                readonly property alias discardButton: discardHunkButton
                /// Picked by hand — this line goes with the next write.
                readonly property bool picked:
                    diffPane.partial
                    && diffPane.lineChosen(diffRow.hunk, diffRow.line)
                    && (diffRow.kind === "add" || diffRow.kind === "del")
                /// The pointer is on this hunk's heading, so the whole
                /// hunk lights: the heading's two words act on exactly
                /// these rows, and this is what says so
                /// (デザイン規約 §diff の中のステージ). Only the heading
                /// does it — a pointer resting on a line is reading, not
                /// aiming at the hunk.
                readonly property bool inAimedHunk:
                    diffPane.partial && diffPane.hoverLine < 0
                    && diffPane.hoverHunk === diffRow.hunk
                /// How many of this hunk's lines are picked (heading rows).
                readonly property int pickedInHunk:
                    diffPane.partial && diffRow.kind === "hunk"
                    ? diffPane.chosenIn(diffRow.hunk) : 0
                // The hunk under the pointer, and every line picked by
                // hand, wear the wash a row anywhere else in the app wears
                // under the pointer. A picked line also carries the mark
                // at its head — the diff's own colours own the row's
                // ground, so the choice cannot be shown by filling it.
                Rectangle {
                    anchors.fill: parent
                    color: Theme.bgHover
                    visible: diffRow.picked || diffRow.inAimedHunk
                }
                Rectangle {
                    width: Theme.spaceXs
                    height: parent.height
                    color: Theme.accent
                    visible: diffRow.picked
                }
                Row {
                    anchors.fill: parent
                    spacing: 0
                    Label {
                        width: 42
                        height: parent.height
                        verticalAlignment: Text.AlignVCenter
                        text: diffRow.old_no >= 0 ? diffRow.old_no : ""
                        horizontalAlignment: Text.AlignRight
                        rightPadding: Theme.spaceXs
                        color: Theme.textMuted
                        font.family: Theme.monoFamily
                        font.pixelSize: Theme.fontSm
                    }
                    Label {
                        width: 42
                        height: parent.height
                        verticalAlignment: Text.AlignVCenter
                        text: diffRow.new_no >= 0 ? diffRow.new_no : ""
                        horizontalAlignment: Text.AlignRight
                        rightPadding: Theme.spaceXs
                        color: Theme.textMuted
                        font.family: Theme.monoFamily
                        font.pixelSize: Theme.fontSm
                    }
                    Label {
                        width: parent.width - 84
                        height: parent.height
                        verticalAlignment: Text.AlignVCenter
                        text: diffRow.text
                        elide: Text.ElideRight
                        font.family: Theme.monoFamily
                        font.pixelSize: Theme.fontMd
                        color: diffRow.kind === "add" ? Theme.diffAddedFg
                               : diffRow.kind === "del" ? Theme.diffRemovedFg
                               : diffRow.kind === "hunk" ? Theme.diffHunkHeaderFg
                               : diffRow.kind === "meta" ? Theme.textMuted
                               : Theme.textPrimary
                    }
                }
                MouseArea {
                    id: lineHover
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.LeftButton
                    // Off entirely on a diff with no pieces in it: with
                    // nothing to aim at, a row lighting up under the
                    // pointer would be an offer that is not there.
                    enabled: diffPane.partial
                    // A heading under the pointer lights its own hunk, and
                    // says so through the same pair the automation hook
                    // writes — one answer to "which hunk is being aimed
                    // at", whichever way the pointer got there.
                    onContainsMouseChanged: {
                        if (diffRow.kind !== "hunk")
                            return
                        if (containsMouse) {
                            diffPane.hoverHunk = diffRow.hunk
                            diffPane.hoverLine = -1
                        } else if (diffPane.hoverHunk === diffRow.hunk) {
                            diffPane.hoverHunk = -1
                        }
                    }
                    // One click, one line: a changed line joins what the
                    // next write takes, or leaves it. Anywhere else in the
                    // diff puts the whole choice down — the rows carrying
                    // it are on screen, so there is nothing to lose track
                    // of (デザイン規約 §diff の中のステージ).
                    onClicked: {
                        if (diffRow.kind === "add" || diffRow.kind === "del")
                            diffPane.toggleLine(diffRow.hunk, diffRow.line)
                        else if (diffRow.kind !== "hunk")
                            diffPane.clearLines()
                    }
                }
                // Under the pointer, or named as if it were (see above).
                readonly property bool underPointer:
                    lineHover.containsMouse
                    || (diffPane.hoverHunk === diffRow.hunk
                        && diffPane.hoverLine === diffRow.line)
                // Hunk-level staging. The row carries the hunk index the
                // patch builder needs, so what is staged is exactly what
                // is shown — and so is what is thrown away. Absent on a
                // diff with no pieces in it (see `partial`).
                Row {
                    visible: diffPane.partial && diffRow.kind === "hunk"
                    anchors.right: parent.right
                    anchors.rightMargin: Theme.spaceSm
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: Theme.spaceXs
                    // Only the unstaged side has a piece to throw away: on
                    // the staged side the button beside this one puts the
                    // hunk back where it can be.
                    //
                    // Held, not asked about (デザイン規約 §長押し): the
                    // button sits in the hunk's own heading, so what it
                    // takes is the thing it is standing on, and a bar
                    // coming down over the diff to say so is machinery a
                    // hunk does not need.
                    //
                    // Shaped like the held row of a right-click menu, not
                    // like the toolbar's framed button: this one sits in a
                    // line of other words rather than in a row of other
                    // buttons, and a frame around one word in a heading
                    // reads as a box that has come loose. The mark says it
                    // is held, and the hold fills the words' own ground
                    // edge to edge.
                    ActionButton {
                        id: discardHunkButton
                        visible: !diffPane.staged
                        text: qsTr("Discard hunk")
                        font.pixelSize: Theme.fontSm
                        // Asleep until the pointer is on this heading
                        // (デザイン規約 §diff の中のステージ). The mark is
                        // what still says this one is held rather than
                        // clicked — the colour is saying something else.
                        tone: diffRow.underPointer ? Theme.danger
                                                   : Theme.textSecondary
                        holdTone: Theme.danger
                        holdMs: Metrics.holdMs
                        enabled: !diffPane.busy
                        onHeld: diffPane.discardHunkRequested(diffRow.hunk)
                    }
                    // The same pair of colours the file rows put on their
                    // own `+` and `−`: staging is the green half of the
                    // gesture and unstaging the red one, and the heading
                    // should not name them in a different voice than the
                    // list does — but it says it at the volume of a
                    // heading that is not being pointed at. At rest both
                    // words wear `textSecondary`, which is the colour the
                    // `@@` beside them already has, so the whole heading
                    // reads as one grey line until the pointer arrives
                    // (デザイン規約 §diff の中のステージ). A file diff
                    // carries 2 hunks at the middle and 8 at the ninetieth
                    // percentile — measured over 52 files — so leaving
                    // them all lit puts 2 to 4 coloured words on screen
                    // against the header's one.
                    ActionButton {
                        // With lines picked out of this hunk the word
                        // names them instead: what is about to be written
                        // is the choice, not the hunk. A choice is a
                        // standing state, so the word is lit for as long
                        // as it stands — the pointer is what wakes the
                        // heading, but a choice keeps it awake.
                        // Spelled out rather than left to `%n`: with no
                        // translation loaded Qt keeps the source string as
                        // it stands, and `2 line(s)` on a button is a
                        // placeholder that shipped.
                        text: diffRow.pickedInHunk === 0
                              ? (diffPane.staged ? qsTr("Unstage hunk")
                                                 : qsTr("Stage hunk"))
                              : diffRow.pickedInHunk === 1
                                ? (diffPane.staged ? qsTr("Unstage 1 line")
                                                   : qsTr("Stage 1 line"))
                                : (diffPane.staged
                                   ? qsTr("Unstage %1 lines").arg(diffRow.pickedInHunk)
                                   : qsTr("Stage %1 lines").arg(diffRow.pickedInHunk))
                        font.pixelSize: Theme.fontSm
                        tone: !diffRow.underPointer && diffRow.pickedInHunk === 0
                              ? Theme.textSecondary
                              : diffPane.staged ? Theme.diffRemovedFg
                                                : Theme.diffAddedFg
                        enabled: !diffPane.busy
                        onActivated: {
                            if (diffRow.pickedInHunk > 0)
                                diffPane.stageChosenRequested()
                            else
                                diffPane.stageSelectionRequested(diffRow.hunk, -1)
                        }
                    }
                }
                // The mark a changed line puts out for the hand: `+` where
                // a click takes the line into the staging area and `−`
                // where it takes it back out, in the pair of colours that
                // gesture wears everywhere else (デザイン規約 §diff の中の
                // ステージ). It names the *direction* of the write, not
                // what the line did — an added and a deleted line are both
                // staged by the same `+`. No frame: a box around a mark
                // this size reads as a control that came loose from the
                // toolbar, and the ground it needs is the one the pointer
                // brings with it.
                Rectangle {
                    visible: diffPane.partial
                             && (diffRow.underPointer || diffRow.picked)
                             && (diffRow.kind === "add"
                                 || diffRow.kind === "del")
                    x: Theme.spaceXs
                    anchors.verticalCenter: parent.verticalCenter
                    width: Theme.iconMd
                    height: Theme.iconMd
                    radius: Theme.radiusSm
                    color: stageLineHover.containsMouse ? Theme.bgHover
                                                        : "transparent"
                    ToolTip.visible: stageLineHover.containsMouse
                    ToolTip.delay: Metrics.tipDelayMs
                    ToolTip.text: diffPane.staged ? qsTr("Unstage this line")
                                                  : qsTr("Stage this line")
                    NavIcon {
                        anchors.centerIn: parent
                        width: Theme.iconSm
                        height: Theme.iconSm
                        kind: diffPane.staged ? "minus" : "plus"
                        tint: diffPane.staged ? Theme.diffRemovedFg
                                              : Theme.diffAddedFg
                    }
                    MouseArea {
                        id: stageLineHover
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: diffPane.toggleLine(diffRow.hunk,
                                                       diffRow.line)
                    }
                }
                // No square for throwing one line away. A line can be
                // staged on its own because staging loses nothing — the
                // line stays on disk either way — but the smallest thing
                // that can be thrown away is a hunk (デザイン規約
                // §その他の操作): a bare `×` has no words to say what it
                // takes, and a control that must be held has to say it.
            }
        }
    }
}
