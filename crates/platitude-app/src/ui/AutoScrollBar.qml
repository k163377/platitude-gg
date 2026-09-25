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

    /// Whether the reader is here: pointing into the range, or holding the thumb (a drag may take the pointer
    /// anywhere). Not `activeFocus` — a list keeps it for the rest of the session after one click.
    readonly property bool attended: bar.inArea || bar.pressed

    /// The window's hover stops while this is held (`Hand.heldBar`). Every way a hold ends clears it here: a bar hidden
    /// or disabled mid-drag loses the grab and reports `pressed` false.
    onPressedChanged: {
        if (bar.pressed)
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
        if (!bar.visible)
            bar.lit = false
    }
    /// Says the view has just been sent (wired below for a flickable; `GraphLaneBar` / `DiffCodeScroll` call it).
    /// Only while somebody is here: a load growing the graph and `shiftRows` move the view too, and would leave it lit.
    function moved() {
        if (bar.attended)
            bar.lit = true
    }

    /// What the bar is painting: full ink while it is being used, a step down once it is not.
    readonly property bool bright: bar.lit || bar.pressed

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
