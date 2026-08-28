import QtQuick
import QtQuick.Controls.Fusion

// A scroll bar pinned visible while its view overflows. The default AsNeeded policy re-derives visibility from
// transient view state and has been seen dropping the bar entirely around model swaps, so the policy is computed from
// content size instead. Attached to a Flickable, the bar's parent is the view itself. The comparison carries a pixel of
// slack. Text heights are fractional -- a font whose line box is 23.5 makes a two-line item 47.0 inside a frame laid
// out at 47 -- and a strict `>` turns a rounding remainder no eye can see into a bar down the side of a view that has
// nothing to scroll (2026-08-15 ユーザー報告). A real overflow is a line of text, never a fraction of one, so nothing that
// should scroll is lost by this.
//
// **A bar with somewhere to go is always drawn; what changes is how brightly** (デザイン規約 §QML 実装ルール のバーの明るさ).
// Full while the reader is sending this view, three tenths of that once they have left the range it sends. Taking the
// bar away instead is not wanted: over a ground this dark it is the only thing on screen saying where the reading
// stands, and a bar that has to be waited out has no wait that is right for both reading and leaving.
//
// **Above anything else pinned to the view's frame.** An overlay laid over the rows (`GraphHeadPin`'s band, the
// sidebar's `HeadPinRow`) is a child of the view like this bar is, and is built later, so at equal z it lands on top
// and takes the bar with it — which is where the reader is in two thousand rows (2026-08-22 ユーザー報告). One step up
// is all it takes, and it is the right way round on its own terms: nothing in a view stands over its scroll bar.
ScrollBar {
    id: bar

    /// The view this bar answers for. A bar put on a `Flickable` is parented to it, which is what the default reads;
    /// one put on a `ScrollView` is parented to the view instead, and the flickable is that view's `contentItem` — so
    /// those name it (`DescriptionBox` / `MessageEditor`). **A `ScrollView`'s bar has to be told**: left to the style it
    /// stands only while the pointer is on its own 10px strip, so a box scrolled by the wheel never shows one at all
    /// (qmltestrunner 実測 2026-08-27: `active` false at rest, false after `contentY` is assigned, true only on hover of
    /// the bar). The two sideways bars answer for no view at all and leave this null.
    property Flickable view: parent as Flickable

    /// Whether this bar says the idle step by carrying less of itself. The style's thumb is one ink and says every
    /// state with the amount of it. The panels' slab is opaque on purpose and names a second colour instead, so it
    /// clears this (`PaneScrollBar`).
    property bool dimsItself: true

    /// Whether the reader is inside the range this bar sends. **The view answers for itself**: a `HoverHandler` on the
    /// view is an ancestor of its own rows, and an ancestor's handler leaves every row its own hover (規約 §QML 実装ルール
    /// — what must not happen is a handler on an item stacked *over* the rows). The two sideways bars have no view to
    /// ask and are handed their pane's answer instead. **A headless run writes this same property**, since hover cannot
    /// be injected (verify-ui).
    property bool inArea: viewHover.hovered
    HoverHandler {
        id: viewHover
        parent: bar.view
    }

    /// Whether the reader is here at all: pointing into the range, or holding the thumb — a drag may take the pointer
    /// anywhere and is still this view being sent.
    ///
    /// **Keyboard focus is not attention.** A list keeps `activeFocus` long after the reader has gone elsewhere — one
    /// click on a graph row holds it for the rest of the session — so reading it here left a bar bright while its pane
    /// was not being looked at, and let a send nobody asked for light it from across the window (qmltestrunner 実測
    /// 2026-08-28). Two panes then differed for a reason nothing on screen showed.
    readonly property bool attended: bar.inArea || bar.pressed

    /// Whether this bar has been sent since the reader arrived. **Raised by the sending, lowered by leaving** — what
    /// says a bar is done being read is the reader going elsewhere, not a count of milliseconds, so there is no timer
    /// here (デザイン規約 §QML 実装ルール のバーの明るさ).
    property bool lit: false
    onAttendedChanged: {
        if (!bar.attended)
            bar.lit = false
    }
    // Taken off screen, a bar keeps nothing it had: having been sent ends with the thing that was sent.
    onVisibleChanged: {
        if (!bar.visible)
            bar.lit = false
    }
    /// Says the view has just been sent. The bars attached to a flickable are wired to it below; the two that are not
    /// call it from whatever they report on (`GraphLaneBar` / `DiffCodeScroll`).
    ///
    /// **Only while somebody is here.** The graph grows in the background all through a load and `shiftRows` puts the
    /// reading back where it was; neither is a reader sending this view, and a bar lit by them would stay lit until a
    /// pointer had visited the pane and left again.
    function moved() {
        if (bar.attended)
            bar.lit = true
    }

    /// What the bar is painting: full ink while it is being used, a step down once it is not.
    readonly property bool bright: bar.lit || bar.pressed

    /// **This bar does not take the hover.** Nothing it paints reads its own `hovered` — brightness is the view being
    /// sent — and taking it costs twice over (qmltestrunner 実測 2026-08-28):
    ///
    ///  - **a `Flickable` loses its own handler's hover while the pointer is on a hover-taking child.** A plain `Item`
    ///    pane does not, but a view does, so `inArea` went false for the width of the bar's own strip — a hole in the
    ///    middle of the range, right where the reader reaches to grab the thing.
    ///  - **the bar's `hovered` latches after a press.** Drag the thumb, release, walk the pointer away, and it stayed
    ///    true until the pointer next entered and left the bar — holding `attended` up with it.
    ///
    /// Named rather than left off, because `Control.hoverEnabled` otherwise falls back to the theme's
    /// `useHoverEffects` (規約 §QML 実装ルール — measured false offscreen, and not something to inherit either way).
    hoverEnabled: false

    // `contentY`, not the bar's own `position`: `position` is `contentY / contentHeight`, so rows arriving below the
    // reader move it without the view having gone anywhere.
    Connections {
        target: bar.view
        function onContentYChanged() { bar.moved() }
    }

    z: 1
    policy: view && view.contentHeight + view.topMargin + view.bottomMargin > view.height + 1
            ? ScrollBar.AlwaysOn : ScrollBar.AsNeeded
    /// **No fade: a bar with nowhere to go is not drawn at all.** The style keeps the thumb painted for 450ms and then
    /// takes 200ms over it, so a view that had somewhere to go for one frame of layout went on showing a bar for two
    /// thirds of a second after it stopped having anywhere — a full-height bar standing over a box that fits (2026-08-27
    /// ユーザー報告 / 実測: the summary box settles at `policy=AsNeeded size=1 active=false` with the thumb still at 0.75).
    /// Nowhere to go is not a state to be eased out of; only the brightness below is.
    ///
    /// The policy above is the whole question, which is why the style's second term (`active && size < 1`) is not
    /// repeated here: that term is what an `AsNeeded` bar has instead of an answer, and here there is one.
    visible: policy === ScrollBar.AlwaysOn

    /// **Three tenths of itself when idle, up at once and down over 400ms** (規約 §QML 実装ルール のバーの明るさ). The ink
    /// is picked so the bar in use reads `borderDefault` over a pane's ground; three tenths of that lands on
    /// `bgElevated`, the one step the palette has between that ground and the lines a pane divides itself with. The
    /// share has to be that large because the ground is nearly black: the whole range a bar can occupy without
    /// shouting is some forty levels wide, and a step has to take a real part of it to read as a step at all.
    ///
    /// **Nothing is animated on the way up**: what the reader has just asked for should already be there, and between
    /// two brightnesses there is no jump to soften. Only the way down is drawn out, which is the half nobody should
    /// notice.
    ///
    /// **`gone` is what keeps a fall from being carried off screen and back.** The two sideways bars come and go under
    /// a reader's hand, and a bar hidden part way down would return at whatever the fall had reached and finish it in
    /// view — a soft flash on the way in. Standing first, `gone` wins over `lit`, and the transition below names its
    /// `to`, so the way out of `lit` is drawn only when the bar is still on screen to be looked at; every other
    /// crossing is instant.
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
