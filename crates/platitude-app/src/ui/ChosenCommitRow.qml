pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui

// One commit of a choice, as the right pane lists it: face on the graph's lane, summary cut to one line, and the hash
// plate. The only press the row takes is a Ctrl one, which drops the commit; its words are fields
// (デザイン規約 §複数のコミットを選ぶ).
Item {
    id: commitRow

    /// One row as `GraphModel.chosenRows` names it (`oid`, `sha8`, `subject`, `body`, `author`, `authorTime`, `avatar`,
    /// `avatarUrl`, `mates`, and `laneAbove` / `laneBelow` — "", "solid" or "dashed", デザイン規約 §複数のコミットを選ぶ).
    required property var modelData
    /// The card the page has out, by target key (`RowHoverHost.cardKey`); the row it is of keeps its band while the
    /// card stands over it.
    property string cardKey: ""
    /// How far the face moves down from the row's middle to its summary's capitals (デザイン規約 §アイコンの席). One
    /// figure for the whole list (`DetailsPane.chosenFaceDrop`), read off the window's face: a summary set in a
    /// fallback face has a box of its own, and a face following it would part the lane's dashes at that row's seams.
    property real faceDrop: 0

    /// The names the card's opener reads off a row, whichever list it is in (`RowHoverHost.openRowCard`).
    readonly property string oid_hex: commitRow.modelData.oid
    readonly property string subject: commitRow.modelData.subject
    readonly property string body: commitRow.modelData.body
    readonly property string author: commitRow.modelData.author
    readonly property double author_time: commitRow.modelData.authorTime
    readonly property var co_authors: commitRow.modelData.mates
    /// Always -1: the index a card carries is a graph row, and this list's own numbering would pick a commit at random
    /// (`RepoPage.activateRow` looks the commit up when given -1).
    readonly property int index: -1
    /// Where the pointer is along the row, so the card opens under the hand (0 for a run that asks with no hand).
    readonly property real pointerX: Math.max(0, commitRow.pointerRowX)
    /// The row cut the message to one line, so its card shows it whole (`RowHoverHost.openRowCard`).
    readonly property bool wholeMessage: true

    /// The pane's hand (`HandWatch`), which the row reads its light and its ask off by geometry each time the hand or
    /// the rows move: a row the choice brought under a still hand is the row under it, and what it asks for waits on
    /// the seat for the hand (as the graph's rows, `GraphRowDelegate.syncHand`).
    property HandWatch paneHand: null
    /// Where along the row the hand is, or -1 for "not on this row".
    property real pointerRowX: -1
    readonly property int handMoves: commitRow.paneHand !== null ? commitRow.paneHand.moves : 0
    onHandMovesChanged: commitRow.syncHand()
    readonly property int handShifts: commitRow.paneHand !== null ? commitRow.paneHand.shifts : 0
    onHandShiftsChanged: commitRow.syncHand()
    // The row's place in the list changed with the list where it was — the choice changing: the rows moved under the
    // hand, and the hand is told so by the row that had it or has it now (`GraphRowDelegate.placeMoved`).
    onYChanged: commitRow.placeMoved()
    function syncHand() {
        const hand = commitRow.paneHand
        if (hand === null || !hand.known || !commitRow.visible || commitRow.pooled) {
            commitRow.pointerRowX = -1
            return
        }
        // Inside the list's own box: a row kept built past its edge lies under the pane's other parts.
        const view = commitRow.ListView.view
        if (view) {
            const v = view.mapFromItem(null, hand.at.x, hand.at.y)
            if (v.x < 0 || v.y < 0 || v.x >= view.width || v.y >= view.height) {
                commitRow.pointerRowX = -1
                return
            }
        }
        const p = commitRow.mapFromItem(null, hand.at.x, hand.at.y)
        commitRow.pointerRowX = p.x >= 0 && p.y >= 0 && p.x < commitRow.width && p.y < commitRow.height ? p.x : -1
    }
    function placeMoved() {
        const hand = commitRow.paneHand
        if (hand === null || !commitRow.visible || commitRow.pooled)
            return
        // A hand the rows cannot hear — in a card over them, elsewhere — is told all the same (`GraphRowDelegate`).
        if (!hand.known) {
            hand.rowsMoved()
            return
        }
        const p = commitRow.mapFromItem(null, hand.at.x, hand.at.y)
        const over = p.x >= 0 && p.y >= 0 && p.x < commitRow.width && p.y < commitRow.height
        if (over !== (commitRow.pointerRowX >= 0))
            hand.rowsMoved()
    }
    /// The hand is on the row: the page's seat is told as it comes and goes (`RowHoverHost.pointRow`), and counts
    /// the rest (規約 §hover のツールチップ「補足は待ってから開く」).
    readonly property bool handOn: commitRow.pointerRowX >= 0
    onHandOnChanged: commitRow.hoverRequested(commitRow, commitRow.handOn)
    // A row pooled is off screen; one handed another record is another row: what the hand found on it does not carry
    // over, and nothing lights or opens until the hand moves — and what it had out goes (`rewritten`). Pooled, it
    // stays `visible` where it last stood (`GraphRowDelegate.pooled`).
    property bool pooled: false
    ListView.onPooled: {
        commitRow.pooled = true
        commitRow.pointerRowX = -1
    }
    ListView.onReused: commitRow.pooled = false
    onOid_hexChanged: {
        commitRow.pointerRowX = -1
        commitRow.rewritten(commitRow)
    }
    /// This row now shows another commit: the card out for the one before has nothing to stand on
    /// (`RowHoverHost.rowGone`).
    signal rewritten(var row)

    signal hoverRequested(var row, bool inside)
    signal copyRequested(string text)
    /// A Ctrl press takes this commit back out of the choice.
    signal dropRequested(string oidHex)
    /// Whether a press is the row's own: only a Ctrl one is. A refused press goes on down to the words and the hand
    /// under them, which keeps a plain drag a drag over the text.
    function takesPress(modifiers) {
        return (modifiers & Qt.ControlModifier) !== 0
    }
    /// The click that follows a press the row took, asked the same question so an entry here cannot skip it. Named so
    /// a run presses where a hand does (verify-ui §壊れない動詞の実装と反復).
    function leftClick(modifiers) {
        if (commitRow.takesPress(modifiers))
            commitRow.dropRequested(commitRow.oid_hex)
    }

    // A reused row is handed another record, lanes and all.
    onModelDataChanged: lane.requestPaint()
    width: ListView.view ? ListView.view.width : 0
    // The graph's row, so the faces stand as far apart as its nodes and the lane between them is as long.
    height: Theme.graphRowHeight

    /// The row is wearing the hover wash — under the hand, or under the card it put out.
    readonly property bool lit: commitRow.handOn || commitRow.cardKey === "card:chosen:" + commitRow.oid_hex
    Rectangle {
        anchors.fill: parent
        color: Theme.bgHover
        visible: commitRow.lit
    }

    // The hand that drags over the words from the air around them. Declared first so it lies beneath, and a press
    // reaches it only where no field took one (規約 §右のペインの字は掴める).
    SweepPad {
        id: sweepHand
        anchors.fill: parent
        content: block
    }
    /// Automation only: the sweep as a hand makes it, and what it came away with (verify-ui).
    readonly property alias sweep: sweepHand

    Item {
        id: block
        anchors.fill: parent
        anchors.leftMargin: Theme.spaceXs
        // The scroll bar's gutter: the bar is opaque, and a row reaching under it would hide its hash.
        anchors.rightMargin: Theme.navBarGutter

        // The graph's lane through the faces, at its weight and dash, under the face as the graph's is under its node.
        // One colour, not the lanes': a choice mixes lanes, and the row carries none of them.
        InkCanvas {
            id: lane
            /// A dashed link shows only in the gap between two faces, so its dashes are laid from that gap: as many
            /// whole ones as fit, the air left over split between the two ends. Laid from anywhere else, the gap
            /// catches a dash cut by a face, or a dash from each row meeting at the seam as one stroke. `top` is where
            /// the gap starts in this row's frame (every row stands its face alike — `faceDrop` — so the gap is the
            /// row less a face); the half drawn is `from`–`to`.
            function dashes(ctx, top, from, to) {
                const dash = Metrics.laneDash[0] * Metrics.laneStroke
                const space = Metrics.laneDash[1] * Metrics.laneStroke
                const gap = lane.height - face.height
                const count = Math.floor((gap + space) / (dash + space))
                const lead = Math.floor((gap - count * dash - (count - 1) * space) / 2)
                for (let i = 0; i < count; i++) {
                    const y0 = Math.max(from, top + lead + i * (dash + space))
                    const y1 = Math.min(to, top + lead + i * (dash + space) + dash)
                    if (y1 > y0)
                        ctx.fillRect(lane.width / 2 - Metrics.laneStroke / 2, y0, Metrics.laneStroke, y1 - y0)
                }
            }
            x: face.x + (face.width - lane.width) / 2
            width: Metrics.nodeIcon
            height: parent.height
            onHeightChanged: lane.requestPaint()
            onPaint: {
                const ctx = getContext("2d")
                ctx.clearRect(0, 0, width, height)
                const cx = width / 2
                const cy = face.y + face.height / 2
                // Whole pixels, so the dashes stand sharp wherever the face's capitals put it.
                const gapTop = Math.round(cy + face.height / 2)
                ctx.strokeStyle = Theme.textSecondary
                ctx.fillStyle = Theme.textSecondary
                ctx.lineWidth = Metrics.laneStroke
                const above = commitRow.modelData.laneAbove
                const below = commitRow.modelData.laneBelow
                if (above === "solid") {
                    ctx.beginPath()
                    ctx.moveTo(cx, 0)
                    ctx.lineTo(cx, cy)
                    ctx.stroke()
                } else if (above === "dashed") {
                    lane.dashes(ctx, gapTop - height, 0, cy)
                }
                if (below === "solid") {
                    ctx.beginPath()
                    ctx.moveTo(cx, cy)
                    ctx.lineTo(cx, height)
                    ctx.stroke()
                } else if (below === "dashed") {
                    lane.dashes(ctx, gapTop, cy, height)
                }
            }
        }
        IdentIcon {
            id: face
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            // A picture heading the subject, on its capitals' middle (デザイン規約 §アイコンの席).
            anchors.verticalCenterOffset: commitRow.faceDrop
            code: commitRow.modelData.avatar
            imageUrl: commitRow.modelData.avatarUrl
            onYChanged: lane.requestPaint()
        }
        // No parent row under the plate: that is a question for the pane that shows one commit.
        HashPlate {
            id: plate
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            sha8: commitRow.modelData.sha8
            fullSha: commitRow.modelData.oid
            // A step under the summary beside it (デザイン規約 §複数のコミットを選ぶ).
            shaColor: Theme.textMuted
            shaSize: Theme.fontCode
            onCopyRequested: text => commitRow.copyRequested(text)
        }
        LineText {
            id: subjectLine
            anchors.left: face.right
            // The face's air on its right, as the row's inner step on its left (デザイン規約 §アイコンの席「④ の席」—
            // a face's seat is `spaceXs` | face | `spaceXs`).
            anchors.leftMargin: Theme.spaceXs
            anchors.right: plate.left
            anchors.rightMargin: Theme.spaceSm
            anchors.verticalCenter: parent.verticalCenter
            // A summary is read from the left; the whole of it stays in the field for a drag.
            cutAt: "end"
            // The two layers this row paints, in that order — the mark's opaque patch in the wrong colour is a box
            // around the `…`.
            ground: Theme.bgSurface
            groundOverlay: commitRow.lit ? Theme.bgHover : "transparent"
            text: commitRow.subject
        }
    }

    // Declared last, so the Ctrl press it takes is taken before the words can select on it; every other press is
    // refused and goes on down as if this were not here. Deaf to hover, which would take the pointer from the fields
    // and the row's handler (rules-refs/app-ui.md「行に重ねる面の `HoverHandler` は祖先が持つ」).
    MouseArea {
        id: dropHand
        anchors.fill: parent
        hoverEnabled: false
        // The row keeps the drag it is handed (rules-refs/app-ui.md「行は渡されたドラッグを手放さない」).
        preventStealing: true
        onPressed: mouse => { mouse.accepted = commitRow.takesPress(mouse.modifiers) }
        onClicked: mouse => commitRow.leftClick(mouse.modifiers)
    }
    /// Automation only: that the hand stands, since a run enters `leftClick` itself and would pass with the area gone.
    readonly property bool dropStands: dropHand.enabled && dropHand.width === commitRow.width
                                       && dropHand.height === commitRow.height

    /// Automation only: hover cannot be injected, so a run enters where the hand's arrival would (verify-ui).
    function askCard() {
        commitRow.hoverRequested(commitRow, true)
    }
    /// Whether the face has what it will paint and the words have been laid out — a shot of this list waits on both.
    function rowReady() {
        return face.pictureReady() && subjectLine.width > 0
    }
    /// Whether the summary was cut — read back, since the mark is too small to judge on a picture.
    readonly property alias summaryCut: subjectLine.clipped
}
