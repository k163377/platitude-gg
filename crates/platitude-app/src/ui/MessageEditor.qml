import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude.ui

// The commit message pair: a prominent summary box over a dimmer description box, the two of them one block of one
// height (デザイン規約 §コミットメッセージの 2 つの枠). The commit editor and the commit details pane both write and read messages through
// this same pair, so the two cannot drift apart.
//
// The pair's height is held as one number that the layout splits: the description fills what the summary leaves, so
// the total is exact whatever the font's line height rounds to. One line of summary, the gap, and five lines of
// description at rest; a summary that wraps takes its extra lines out of the description down to two, and past that
// the block itself grows — the summary's own cap is what stops that.
//
// What stands around the pair is the pane's business and comes in as numbers (`listHeight` / `blockRoom` /
// `blockHeight`): only the pane knows what a file list or an author card is. The answers go back out under the names
// the panes have always said them under — the automation hooks read the panes, and the panes forward (verify-ui スキル).
ColumnLayout {
    id: editor

    // ---- what the pane says about the boxes -------------------------
    /// Whether the boxes refuse typing (the details pane on a commit that cannot be rewritten from here; the commit
    /// editor never refuses).
    property bool readOnly: false
    /// Why they refuse, in one line ("" when they do not). A box that refuses typing without saying why reads as
    /// broken.
    property string blockedTip: ""
    /// Stands in for the pointer on the summary box, so its read-only tooltip can be photographed — hover cannot be
    /// injected (verify-ui スキル).
    property bool summaryPointedAt: false
    /// What the empty boxes say when there is a message standing behind them: a stopped merge already has one, and
    /// leaving the boxes empty commits exactly what they are showing (デザイン規約 §進行中の操作から出る). Empty falls
    /// back to the two words the boxes are otherwise named by.
    property string standingSubject: ""
    property string standingBody: ""

    // ---- the mark a reader arrives at -------------------------------
    /// A reader was sent to these boxes from somewhere that could not hold the whole message — the graph row's hover
    /// card, whose note is the way here (デザイン規約 §hover のツールチップ). The pair says which it is until the
    /// reader's next press: they were moved without asking, and what they were moved to has to be findable at a
    /// glance. **Only the pane that is sent to sets it** — the commit editor is where the hand already is.
    ///
    /// **The mark is the frames' own colour** — both boxes already carry a border, so a resting window gains no ink
    /// for it, and the two frames take the pair as one block.
    property bool attention: false
    function callAttention() { editor.attention = true }
    function dropAttention() { editor.attention = false }

    // ---- what the pane says about its own geometry ------------------
    /// The pane's file list height. The list is the one thing under the block that gives, and two rows is where it
    /// stops being a list — so what it has past those rows is the whole of what this pair may borrow (デザイン規約
    /// §コミットメッセージの 2 つの枠).
    property real listHeight: 0
    /// How much of the pane the block holding this pair may take. Only the pane knows what else stands in that block
    /// and above it, so the bound is the pane's — the two panes' definitions differ.
    property real blockRoom: 0
    /// What that block is asking for (its column's implicitHeight), measured against `blockRoom` — see `descOwed`
    /// on why a laid-out height is a ring.
    property real blockHeight: 0

    // ---- what the pair holds ----------------------------------------
    readonly property string subjectText: summaryArea.text
    readonly property string bodyText: descBox.text
    /// The read-only tooltip is up. Reported through the ToolTip's own visible — the output side, so a cut binding
    /// cannot read as green.
    readonly property bool summaryTipShown: summaryArea.ToolTip.visible
    /// What the description box paints — the smoke hooks report the painted side
    /// (app-ui.md).
    readonly property color descriptionColor: descBox.textColor
    readonly property bool descriptionFocused: descBox.focused
    /// Whether a caret is in either box — the pane reads it as "somebody is in here writing", which is when the row
    /// that saves has to be on screen (規約 §コミットメッセージの 2 つの枠).
    readonly property bool anyFocused: summaryArea.activeFocus || descBox.focused

    // -- the message pair is one block of one height --
    /// What the summary box needs for its own lines, capped — nothing in git bounds a summary, and one pasted paragraph
    /// grew this box to 650px, which pushed the description off the pane and left the author row over the window's own
    /// footer (measured at a 2,000-byte subject). Ceiled, so the box is at least as tall as the text inside it —
    /// that is the difference between a scroll bar and no scroll bar.
    readonly property real summaryNeed:
        Math.min(Math.ceil(summaryArea.implicitHeight) + Theme.spaceSm,
                 Theme.messageMaxHeight)
    /// The description gives up its lines to a wrapping summary down to two, and no further.
    readonly property real descFloor: 2 * Theme.fontMdLine + Theme.spaceSm
    /// The pair's resting height: one summary line and five description lines, with the gap between them.
    readonly property real pairRest:
        Theme.fontLgLine + Theme.spaceSm + Theme.spaceXs + Theme.messageMaxHeight
    /// What the pair is laid out at. Past the point where the description has given its last line, the block itself
    /// grows — the summary's own cap is what stops that.
    readonly property real pairBase:
        Math.max(editor.pairRest,
                 editor.summaryNeed + Theme.spaceXs + editor.descFloor)
    /// What the description is allotted at rest: the rest of the block.
    readonly property real descRest:
        editor.pairBase - editor.summaryNeed - Theme.spaceXs

    // -- what the pane lends the description box --
    //
    // The box carries the pull and the grip (`DescriptionBox`); the bound is the pane's, and arrives as `listHeight`:
    // the file list is the one thing that gives, and two rows is where it stops being a list — so what is left above
    // that is the whole of the room.
    readonly property real descRoom:
        Math.max(0, editor.listHeight - 2 * Theme.rowHeight)
    /// The far side of the same measure: how far past the bound the pane already is, which is what the hand has to give
    /// back. The list is the only thing that gives, so it hits zero and stops answering while the block's column keeps
    /// growing past the pane's edge — the second term is that overflow, and without it the give-back stalls at the last
    /// 48 pixels the list still had (measured: the command log opening under a pulled-open box).
    ///
    /// Measured against the room the block is *allowed* (`blockRoom`): the block's own height follows what the box
    /// does, so reading a laid-out height back here would put the box and the layout in a ring — the box grows, the
    /// height it is compared to is still last frame's, the box is told it owes the difference, and it gives back
    /// everything it just took (measured: the grip did nothing at all, `cap` never left 120).
    readonly property real descOwed:
        Math.max(0, 2 * Theme.rowHeight - editor.listHeight)
        + Math.max(0, editor.blockHeight - editor.blockRoom)
    /// Whether the grip is refusing a pull, and where the hand is while it does (scene coordinates). The page draws the
    /// badge — see `RepoPage` on why it cannot be drawn in the box.
    readonly property alias descRefuses: descBox.gripRefused
    readonly property alias descPoint: descBox.gripPoint
    readonly property bool descGrips: descBox.grips
    readonly property real descHeight: descBox.boxHeight
    readonly property real descWants: descBox.wants
    readonly property real descCap: descBox.cap
    /// How many rows the file list is left with, which is the bound the pull stops at. Rounded: the layout hands out
    /// fractions and what this is about is rows.
    readonly property int descListRows:
        Math.round(editor.listHeight / Theme.rowHeight)
    /// Whether the block is taller than the room it was given — anything in it below the fold. What a pulled-open box
    /// pushes past the pane's edge is exactly what the block ends up scrolling by, so this is also the answer to "has
    /// the box given back what it owes". Said out loud because a headless run cannot see a scroll bar and the shot
    /// frames alike either way (`PGG_AUTO_ACT=window-floor wip`).
    readonly property bool blockScrolls:
        editor.blockHeight > editor.blockRoom + 1
    readonly property bool descKeeps: !editor.blockScrolls
    /// What the pane lays the pair out at: the block, plus whatever the grip has pulled. The pane binds this to
    /// `Layout.preferredHeight` on the instance — a layout's own implicitHeight is the engine's to write, so a binding
    /// there loses to the next recompute and the description box collapses to its 0 implicit height (measured).
    readonly property real pairHeight: editor.pairBase + descBox.extra

    /// A wheel neither box could use, in pixels. The boxes cover most of the block they stand on, so whoever owns that
    /// block moves it by this — without it the surface under the boxes cannot be reached by wheel at all (observed).
    signal wheelPastEnd(real pixels)

    /// Swap in a different message. A pull, and a reading position, belong to the message they were made on — the next
    /// one opens at its own rest height and its own first line. The summary box needs the same caret treatment for the
    /// same reason (see DescriptionBox.resetForNewMessage): a subject long enough to scroll otherwise opens on its last
    /// line.
    function setMessage(subject, description) {
        summaryArea.text = subject
        descBox.text = description
        summaryArea.cursorPosition = 0
        editor.summaryPinned = true
        editor.summaryToTop()
        descBox.resetForNewMessage()
    }
    /// Held at its first line until the reader moves it, the way the description box is held
    /// (`DescriptionBox.pinnedTop`) and for the same reason: assigning `text` leaves the caret at the end, and a
    /// `TextArea` scrolls the flickable under it to keep the caret in view — so a subject longer than the box opens on
    /// its **last** line. A single assignment races that scroll and loses when the layout settles late (measured on a
    /// 2,000-character subject: the box opened on `終端`). Pinning states the intent.
    property bool summaryPinned: false
    function unpinSummary() {
        editor.summaryPinned = false
    }
    Connections {
        target: summaryView.contentItem
        enabled: editor.summaryPinned
        function onContentYChanged() {
            if (editor.summaryPinned && summaryView.contentItem.contentY !== 0)
                summaryView.contentItem.contentY = 0
        }
        function onContentHeightChanged() {
            if (editor.summaryPinned)
                summaryView.contentItem.contentY = 0
        }
    }
    /// Write the texts alone — a revert, or the smoke hook that types into the boxes — the caret, the scroll and
    /// the pull stay put.
    function setTexts(subject, description) {
        summaryArea.text = subject
        descBox.text = description
    }
    /// Escape was pressed in one of the boxes. **The draft goes**, and nothing asks (デザイン規約 §コミットメッセージ
    /// の 2 つの枠): Escape is the reader saying so, and the text it drops was never committed. The caret goes with it,
    /// so the row that saves comes down too.
    signal escaped()
    function leaveBoxes() {
        summaryArea.focus = false
        descBox.dropCaret()
        editor.escaped()
    }
    function summaryToTop() {
        if (summaryView.contentItem)
            summaryView.contentItem.contentY = 0
    }
    /// The same two steps for the summary as the description box makes on its own text (`DescriptionBox.rollBy`): its
    /// own scroll while a pasted paragraph past the shared cap leaves text to move, the block once there is not.
    function rollSummary(dy) {
        // A wheel over the box is the reader moving it: whatever the pin was holding, they have taken over.
        editor.unpinSummary()
        const flick = summaryView.contentItem
        const pixels = dy / 120 * (Metrics.wheelRows * Theme.fontMdLine)
        const max = Math.max(0, flick.contentHeight - flick.height)
        const next = Math.max(0, Math.min(max, flick.contentY - pixels))
        if (Math.abs(next - flick.contentY) > 0.5) {
            flick.contentY = next
            return
        }
        editor.wheelPastEnd(pixels)
    }
    /// Smoke hook: the caret in the description box, the way a click in it puts it there.
    function focusDescription() {
        descBox.takeCaret()
    }
    // -- smoke hooks, forwarded to the box --
    function growDescription(dy) { descBox.grow(dy) }
    function pullDescriptionPast(down) { descBox.pullPast(down) }
    /// Smoke hook: the wheel over the description box, through the one door a notch comes in by
    /// (`DescriptionBox.rollBy` — the flickable inside a `ScrollView` is not interactive, so this is the only thing
    /// that moves this text).
    function rollDescription(dy) { descBox.rollBy(dy) }
    /// Smoke hook: the reader inside the box, which is what keeps its bar bright — and the one thing a headless run
    /// cannot do, since hover is not injectable (`AutoScrollBar.inArea`, verify-ui).
    function holdDescriptionBar(on) { descBox.bar.inArea = on }
    /// How much ink is on the box's own bar, and how far the text stands (verify-ui).
    readonly property real descriptionBarInk: descBox.bar.opacity
    readonly property real descriptionAt: descBox.textAt

    spacing: Theme.spaceXs

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: editor.summaryNeed
        color: Theme.bgBase
        radius: Theme.radiusMd
        border.color: editor.attention ? Theme.accent : Theme.borderDefault
        border.width: Theme.borderWidth
        // The band the inset below leaves between this frame and the words. Declared first, so it is under the view
        // and reaches only a press the text did not take (`SweepBand`).
        SweepBand {
            anchors.fill: parent
            field: summaryArea
        }
        ScrollView {
            id: summaryView
            anchors.fill: parent
            anchors.margins: Theme.spaceXs
            // Both boxes are inset by the one value, lay their own bar out, and refuse the sideways one — see
            // `DescriptionBox` for why each is so.
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            ScrollBar.vertical: AutoScrollBar {
                view: summaryView.contentItem
                x: summaryView.width - width
                y: summaryView.topPadding
                height: summaryView.availableHeight
            }
            // ScrollView keeps its Flickable private -- reach it once it exists. Interaction off, for the reason the
            // description box carries: the flickable answering the same wheel as the handler moved the text twice.
            Component.onCompleted: {
                contentItem.boundsBehavior = Flickable.StopAtBounds
                contentItem.interactive = false
            }
            SummaryArea {
                id: summaryArea
                readOnly: editor.readOnly
                placeholderText: editor.readOnly ? ""
                                 : editor.standingSubject !== "" ? editor.standingSubject
                                 : qsTr("Commit summary")
                // A caret put in the box is the other way the reader takes it over (`DescriptionBox` does the same).
                onActiveFocusChanged: if (summaryArea.activeFocus) editor.unpinSummary()
                Keys.onEscapePressed: event => {
                    editor.leaveBoxes()
                    event.accepted = true
                }
                ToolTip.visible: (hovered || editor.summaryPointedAt) && editor.blockedTip !== ""
                ToolTip.delay: Metrics.tipDelayMs
                ToolTip.text: editor.blockedTip
                WheelHandler {
                    acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
                    onWheel: event => editor.rollSummary(event.angleDelta.y)
                }
            }
        }
    }
    // Always shown, even empty, and taking whatever the summary above it left of the block.
    DescriptionBox {
        id: descBox
        readOnly: editor.readOnly
        placeholderText: editor.readOnly ? ""
                         : editor.standingBody !== "" ? editor.standingBody
                         : qsTr("Description")
        border.color: editor.attention ? Theme.accent : Theme.borderSubtle
        restHeight: editor.descRest
        room: editor.descRoom
        owed: editor.descOwed
        onWheelPastEnd: pixels => editor.wheelPastEnd(pixels)
        onEscaped: editor.leaveBoxes()
    }
}
