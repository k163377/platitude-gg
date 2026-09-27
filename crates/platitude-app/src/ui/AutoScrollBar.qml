import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

// A scroll bar pinned visible while its view overflows, the policy computed from content size: the style's AsNeeded
// re-derives it from transient view state and has been seen dropping the bar around model swaps. The comparison has a
// pixel of slack — text heights are fractional, and a strict `>` stands a bar on a view with nothing to scroll.
//
// `z: 1`: an overlay pinned over the rows (`GraphHeadPin`'s band, `HeadPinRow`) is a later child of the view and
// would otherwise cover the bar.
ScrollBar {
    id: bar

    /// The view this bar answers for: by default the `Flickable` it is attached to. A bar on a `ScrollView` is parented
    /// to the view, so its user names the `contentItem` (`DescriptionBox` / `MessageEditor`) — left to the style, that
    /// bar stands only under the pointer. The two sideways bars leave this null.
    property Flickable view: parent as Flickable

    /// Whether the idle step is drawn as less opacity. `PaneScrollBar` clears it: its opaque slab names a colour.
    property bool dimsItself: true

    /// The arrows at the two ends of an upright bar (デザイン規約 §スクロールバーの矢印): one thumb across (Chromium's
    /// Fluent bar, measured), each end the arrow's ink and above and below it the air the thumb keeps from the bar's
    /// free side (`leftPadding`) — a thin, faint bar holds its arrows close. A sideways bar has none. `PaneScrollBar`
    /// hands in its own span.
    property real arrowSpan: bar.orientation === Qt.Vertical ? bar.availableWidth : 0
    readonly property int arrowLength: bar.arrowSpan > 0
                                       ? Math.ceil(bar.arrowSpan * upArrow.tallShare + 2 * bar.leftPadding) : 0
    property bool arrowsBuried: false
    /// The thumb's resting ink, one step up on the arrow being held and on nothing else: Chromium lights only the part
    /// in use (measured: a thumb in the hand leaves the arrows as they are).
    property color arrowInk: bar.palette.mid
    property color arrowHeldInk: bar.palette.dark
    /// The style's thumb stands at three quarters (Fusion's `active` state); the arrows are drawn as it is.
    property real arrowOpacity: 0.75
    /// The bar's length, read off its view — the bar's own height feeds its implicit height through the padding. A
    /// bar that ends short of its view names its own (`DescriptionBox`).
    property real lengthAlong: bar.view ? bar.view.height : 0
    /// A bar shorter than its two ends squeezes each to half its length, the arrows with them, and has no thumb once
    /// nothing is left between them (Chromium's Fluent bar, measured on boxes 10–140px tall: the arrows never go).
    readonly property int arrowEnd: Math.min(bar.arrowLength, Math.floor(bar.lengthAlong / 2))
    readonly property bool hasTrack: bar.arrowLength === 0 || bar.lengthAlong - 2 * bar.arrowEnd >= 1

    topPadding: bar.arrowLength > 0 ? bar.arrowEnd : 2
    bottomPadding: bar.arrowLength > 0 ? bar.arrowEnd : 2
    Binding {
        target: bar.contentItem
        property: "visible"
        value: bar.hasTrack
    }

    /// Which arrow is held: -1 the upper, 1 the lower, 0 neither.
    property int arrowHeld: 0
    /// Whether the held arrow is running on (the pause is over, and the hand has stayed on the arrow).
    property bool arrowRunning: false
    /// Which way the held track sends: -1 up (pressed above the thumb), 1 down, 0 not held. Only an upright bar with a
    /// view pages; a sideways bar keeps the style's jump to the press.
    property int trackHeld: 0
    /// Where the hand holds the track, in the bar's coordinates: the run stops as the thumb reaches it.
    property real trackAt: 0
    /// Whether the held track is running on (the pause is over, the thumb short of the hand, the hand on the bar).
    property bool trackRunning: false
    readonly property bool trackWaiting: trackWait.running
    /// The page a press on the track sends (`Metrics.trackPageShare` of the view, whole pixels as Chromium's).
    readonly property real trackPage: bar.view ? Math.floor(bar.view.height * Metrics.trackPageShare) : 0
    /// When the run last sent, for the next tick's share: the timer's beat slips under load, the speed must not.
    property real runAt: 0
    /// The glide a press's step rides. A surface that glides its wheel notches hands its own in (`WheelGlide`), so a
    /// notch after a step adds to it and every hand that halts the surface's glide halts the step as well
    /// (rules-refs/app-ui.md「ホイールの 1 ノッチは送られる、跳ばない」); any other bar rides one of its own.
    property var stepGlide: ownGlide
    /// Whether a press's step is still gliding.
    readonly property bool stepping: bar.stepGlide.sending

    /// Where the view may stand: its own `clampY` where it keeps one (margins, a room of its own), its extent otherwise.
    function limitY(y) {
        const v = bar.view
        if (typeof v.clampY === "function")
            return v.clampY(y)
        const top = v.originY - v.topMargin
        return Math.max(top, Math.min(y, Math.max(top, v.originY + v.contentHeight + v.bottomMargin - v.height)))
    }
    /// A press on an arrow, `dir` -1 up and 1 down: one step, glided, and the pause before a run. The handlers below
    /// and a run both come in here (verify-ui implement.md).
    function pressArrow(dir) {
        bar.arrowHeld = dir
        bar.arrowRunning = false
        bar.stepGlide.sendTo(bar.limitY(bar.stepGlide.at + dir * Metrics.arrowStep))
        runWait.restart()
    }
    /// The hand let go: the run stops where it is. A step still gliding lands.
    function releaseArrow() {
        bar.arrowHeld = 0
        bar.arrowRunning = false
        runWait.stop()
    }
    /// The hand slid off the held arrow: the run stops and does not come back until the next press (Chromium,
    /// measured).
    function pointerOnArrow(on) {
        if (on || bar.arrowHeld === 0)
            return
        bar.arrowRunning = false
        runWait.stop()
    }
    function startRun() {
        bar.runAt = Date.now()
        bar.arrowRunning = true
    }
    function runTick() {
        const now = Date.now()
        const share = (now - bar.runAt) / 1000
        bar.runAt = now
        if (bar.trackRunning)
            bar.trackTick(share)
        else
            bar.view.contentY = bar.limitY(bar.view.contentY + bar.arrowHeld * Metrics.arrowRepeatSpeed * share)
    }

    /// The thumb's two ends in the bar's coordinates, as drawn (`visual…` carries the style's minimum size).
    function thumbTop() {
        return bar.topPadding + bar.visualPosition * bar.availableHeight
    }
    function thumbBottom() {
        return bar.thumbTop() + bar.visualSize * bar.availableHeight
    }
    /// Whether the thumb has reached the hand on the held track, from the side it pages towards.
    function thumbAtHand() {
        return bar.trackHeld > 0 ? bar.thumbBottom() >= bar.trackAt : bar.thumbTop() <= bar.trackAt
    }
    /// A press on the track at `y`: one page, glided, towards the press, and the pause before a run — or nothing, if
    /// the press is on the thumb, whose own drag answers it (the handler hands it on). Chromium's Fluent bar, measured.
    function pressTrack(y) {
        if (y >= bar.thumbTop() && y <= bar.thumbBottom())
            return false
        bar.trackHeld = y < bar.thumbTop() ? -1 : 1
        bar.trackAt = y
        bar.trackRunning = false
        bar.stepGlide.sendTo(bar.limitY(bar.stepGlide.at + bar.trackHeld * bar.trackPage))
        trackWait.restart()
        return true
    }
    function releaseTrack() {
        bar.trackHeld = 0
        bar.trackRunning = false
        trackWait.stop()
    }
    /// The hand moved on the held track: off the bar the run stops and does not come back until the next press; along
    /// it, the run aims at the hand and stops once the thumb is there (Chromium, measured).
    function pointerOnTrack(inside, y) {
        if (bar.trackHeld === 0)
            return
        if (inside)
            bar.trackAt = y
        if (!inside || bar.thumbAtHand()) {
            bar.trackRunning = false
            trackWait.stop()
        }
    }
    function startTrackRun() {
        if (bar.thumbAtHand())
            return
        bar.runAt = Date.now()
        bar.trackRunning = true
    }
    /// Pages twenty a second (the arrow's beat); the last one only as far as puts the thumb's far end on the hand.
    function trackTick(share) {
        const v = bar.view
        const perSecond = Metrics.arrowRepeatSpeed / Metrics.arrowStep
        v.contentY = bar.limitY(v.contentY + bar.trackHeld * bar.trackPage * perSecond * share)
        if (!bar.thumbAtHand())
            return
        const past = bar.trackHeld > 0 ? bar.thumbBottom() - bar.trackAt : bar.trackAt - bar.thumbTop()
        const travel = bar.availableHeight * (1 - bar.visualSize)
        const reach = v.contentHeight + v.topMargin + v.bottomMargin - v.height
        if (travel > 0)
            v.contentY = bar.limitY(v.contentY - bar.trackHeld * past * reach / travel)
        bar.trackRunning = false
    }

    WheelGlide {
        id: ownGlide
        view: bar.view
    }
    Timer {
        id: runWait
        interval: Metrics.arrowRepeatDelayMs
        onTriggered: bar.startRun()
    }
    Timer {
        id: trackWait
        interval: Metrics.arrowRepeatDelayMs
        onTriggered: bar.startTrackRun()
    }
    Timer {
        interval: 16
        repeat: true
        running: bar.arrowRunning || bar.trackRunning
        onTriggered: bar.runTick()
    }

    ScrollArrow {
        id: upArrow
        x: bar.leftPadding
        width: bar.availableWidth
        height: bar.arrowEnd
        visible: bar.arrowEnd > 0
        buried: bar.arrowsBuried
        span: bar.arrowSpan * bar.arrowEnd / Math.max(1, bar.arrowLength)
        ink: bar.arrowHeld === -1 ? bar.arrowHeldInk : bar.arrowInk
        opacity: bar.arrowOpacity
    }
    ScrollArrow {
        x: bar.leftPadding
        y: bar.height - bar.arrowEnd
        width: bar.availableWidth
        height: bar.arrowEnd
        visible: bar.arrowEnd > 0
        down: true
        buried: bar.arrowsBuried
        span: bar.arrowSpan * bar.arrowEnd / Math.max(1, bar.arrowLength)
        ink: bar.arrowHeld === 1 ? bar.arrowHeldInk : bar.arrowInk
        opacity: bar.arrowOpacity
    }
    // The track between the arrows: a press off the thumb pages (`pressTrack`); one on the thumb is handed on to the
    // bar's own drag. Declared before the arrows' areas; none of them takes hover (below: the bar takes none).
    //
    // `preventStealing` on all three: the bar is a child of its view, which filters its children's presses and takes a
    // moving hand for a drag of its rows — the thumb keeps its grab by itself, these areas have to say so.
    MouseArea {
        y: bar.arrowEnd
        width: bar.width
        height: Math.max(0, bar.height - 2 * bar.arrowEnd)
        enabled: bar.view !== null && bar.orientation === Qt.Vertical && bar.hasTrack
        preventStealing: true
        onPressed: mouse => mouse.accepted = bar.pressTrack(mouse.y + bar.arrowEnd)
        onReleased: bar.releaseTrack()
        onCanceled: bar.releaseTrack()
        onPositionChanged: mouse => bar.pointerOnTrack(containsMouse, mouse.y + bar.arrowEnd)
    }
    // The whole width of the bar takes the press, as the thumb's does. Hover stays off (below: the bar takes none).
    MouseArea {
        width: bar.width
        height: bar.arrowEnd
        enabled: bar.arrowEnd > 0
        preventStealing: true
        onPressed: bar.pressArrow(-1)
        onReleased: bar.releaseArrow()
        onCanceled: bar.releaseArrow()
        onContainsMouseChanged: bar.pointerOnArrow(containsMouse)
    }
    MouseArea {
        y: bar.height - bar.arrowEnd
        width: bar.width
        height: bar.arrowEnd
        enabled: bar.arrowEnd > 0
        preventStealing: true
        onPressed: bar.pressArrow(1)
        onReleased: bar.releaseArrow()
        onCanceled: bar.releaseArrow()
        onContainsMouseChanged: bar.pointerOnArrow(containsMouse)
    }

    /// Whether the reader is inside the range this bar sends, from a handler on the view — an ancestor of the rows, so
    /// it leaves them their hover (rules-refs/app-ui.md「行に重ねる面の `HoverHandler` は祖先が持つ」). The two sideways
    /// bars have no view and are handed their pane's answer; a headless run writes this property too.
    ///
    /// Read through a guard: a pointer handler's `parent` is its QObject parent, so with no view the handler is left
    /// to the JS heap and collected, and the binding would read a destroyed object.
    property bool inArea: viewHover ? viewHover.hovered : false
    HoverHandler {
        id: viewHover
        parent: bar.view
    }

    /// Whether the bar is in the hand: the thumb, an arrow, or the track.
    readonly property bool held: bar.pressed || bar.arrowHeld !== 0 || bar.trackHeld !== 0
    /// Whether the reader is here: pointing into the range, or holding the bar (a drag may take the pointer
    /// anywhere). Not `activeFocus` — a list keeps it for the rest of the session after one click.
    readonly property bool attended: bar.inArea || bar.held

    /// The window's hover stops while this is held (`Hand.heldBar`): an arrow or the track runs the rows past a still
    /// hand just as a drag does. Every way a hold ends clears it here: a bar hidden or disabled mid-drag loses the grab
    /// and reports `pressed` false, and an arrow's or the track's press is cancelled.
    onHeldChanged: {
        if (bar.held)
            Hand.heldBar = bar
        else if (Hand.heldBar === bar)
            Hand.heldBar = null
    }
    // A bar taken down in the middle of a drag (its tab closed from the keyboard) must not go on being held.
    Component.onDestruction: {
        if (Hand.heldBar === bar)
            Hand.heldBar = null
    }

    /// Whether this bar has been sent since the reader arrived: raised by sending, lowered by leaving, no timer
    /// (デザイン規約 §QML 実装ルール のバーの明るさ).
    property bool lit: false
    onAttendedChanged: {
        if (!bar.attended)
            bar.lit = false
    }
    onVisibleChanged: {
        if (!bar.visible) {
            bar.lit = false
            bar.releaseArrow()
            bar.releaseTrack()
        }
    }
    /// Says the view has just been sent (wired below for a flickable; `GraphLaneBar` / `DiffCodeScroll` call it).
    /// Only while somebody is here: a load growing the graph and `shiftRows` move the view too, and would leave it lit.
    function moved() {
        if (bar.attended)
            bar.lit = true
    }

    /// What the bar is painting: full ink while it is being used, a step down once it is not.
    readonly property bool bright: bar.lit || bar.held

    /// Hover is left to the view. Taking it breaks two ways: a `Flickable` drops its own handler's hover over a
    /// hover-taking child (a hole in `inArea` the width of the bar), and the bar's `hovered` latches after a press.
    /// Named, not inherited from `useHoverEffects` (rules-refs/app-ui.md の `useHoverEffects` の行).
    hoverEnabled: false

    // `contentY`, not `position`: rows arriving below the reader move `position` without the view going anywhere.
    Connections {
        target: bar.view
        function onContentYChanged() { bar.moved() }
    }

    z: 1
    policy: view && view.contentHeight + view.topMargin + view.bottomMargin > view.height + 1
            ? ScrollBar.AlwaysOn : ScrollBar.AsNeeded
    /// Drawn exactly while it has somewhere to go: the style's fade-out (450ms + 200ms) left a full-height bar over a
    /// box that fits. Its `active && size < 1` term stays out — the policy above is the whole answer.
    visible: policy === ScrollBar.AlwaysOn

    /// The idle share and the timing: 規約 §スクロールバー / §QML 実装ルール のバーの明るさ.
    ///
    /// `gone` stands first and the transition names its `to`: a sideways bar hidden part way down would otherwise
    /// come back mid-fall and finish it in view. Only leaving `lit` on screen is animated.
    opacity: bar.dimsItself ? 0.3 : 1
    states: [
        State { name: "gone"; when: !bar.visible },
        State {
            name: "lit"
            when: bar.bright
            PropertyChanges { bar.opacity: 1 }
        }
    ]
    transitions: Transition {
        from: "lit"
        to: ""
        NumberAnimation { target: bar; property: "opacity"; duration: 400 }
    }
}
