import QtQuick
import QtQuick.Controls.Fusion
import platitude.ui

/// The shared tooltip — the one popup in this app nobody declares. The attached property builds it from the style, so
/// it arrives in Fusion's own clothes: a pale yellow ground, a frame that reads the *text* role (so the palette cannot
/// separate the two), a drawn shadow, and a seat in the middle of whatever raised it. Reaching it through `host` is the
/// only way to dress it, and dressing it once carries to every `ToolTip.text` in the tree.
///
/// Three things are put right here, and the last two are one thing said twice (規約 §hover のツールチップ):
///
/// * **The words can be taken away.** The content is a `CardText`, so the sentence is pressed, dragged over and copied
///   like any other text in this window — a path, a ref's full name and a git version are all things a reader wants in
///   their hands, not just in front of their eyes.
/// * **The hand can get to them.** A tooltip centred on its target opens a pane's width away from the pointer on a row
///   that is a pane wide, and there is no walking to it: leaving the row takes the tip down. It opens beside the hand
///   instead, and **flush** against the target — a gap is a band the pointer crosses while touching neither, and
///   whatever it was reaching for goes out under it (the same rule the ref list and the co-author card follow).
/// * **It waits.** The site's own binding falls the instant the pointer leaves the target, which is the instant the
///   hand starts walking into the tip. So a fall is not taken at its word: the tip comes back up and a beat
///   (`hoverKeepMs`) decides, by which time either the hand is inside it or nothing is asking.
///
/// An Item, not a QtObject: the Components below are children, and a QtObject has nowhere to put a child.
Item {
    id: shared

    /// The item the attached tooltip is read off, and the ground every placement below is measured in — one is enough
    /// for the whole tree. Required: `sharedTip` is read while this is built.
    required property Item host
    /// Where the hand is, for the tip to open beside. Window-wide, and it stops answering while a popup covers the
    /// pointer — which is exactly when the tip should fall back to its target instead (`PointerWatch`).
    required property PointerWatch hand

    readonly property var sharedTip: shared.host.ToolTip.toolTip

    /// Where the hand is on the target, for the runs that have no pointer to put anywhere: a share of the target's own
    /// width (0 = its left edge, 1 = its right). Negative — the ordinary case — means there is no stand-in, and a tip
    /// with no hand to open beside centres on its target the way the style's own did.
    ///
    /// The same shape every hover stand-in in this app has (`FileRowDelegate.tipPointedAt` and its kin): hover cannot
    /// be injected, so the automation writes what the pointer would have written.
    property real handAcross: -1

    /// The pointer is on the tip — on the padding band the ground covers, or on the words themselves. **Two handlers,
    /// because the background and the content are siblings rather than parent and child** (規約 §hover のツールチップ の
    /// 罠 (2) — the pair `AppCard` carries, needed here for the same reason and measured on the same scene).
    property bool groundPointed: false
    property bool wordPointed: false
    readonly property bool pointed: shared.groundPointed || shared.wordPointed

    /// The tip is only up because this part put it back; nothing has asked for it since. Cleared when it really goes,
    /// and when another target takes the tip over.
    property bool keeping: false
    /// This part is taking it down, so the fall below is not one to argue with.
    property bool dropping: false

    /// Where the tip was opened beside, in `host` coordinates, and whether there was a hand to read it off. **Frozen
    /// when the tip comes out**: a seat that followed the pointer would slide out from under the hand walking into it.
    property real anchorX: 0
    property bool anchorKnown: false

    /// Whether a hand could be walking into the tip, which is the whole of what the beat below waits for.
    ///
    /// **A run has no hand**, and that is not a detail: it points at rows through the stand-ins, one after another, and
    /// a tip held up for a walk nobody is taking is still standing — with the *previous* row's seat — when the run asks
    /// whether this one put one out. Held open that way it also stops the attached property from ever asking again
    /// (`QQuickToolTipAttached` reads the instance back and says "already showing"), so the next row's tip never comes
    /// (2026-08-27 実測: `nav-tip` waited out its watchdog with the right text and `visible=false`).
    ///
    /// **`hovered` is not the test.** The offscreen platform answers a window-wide `HoverHandler` with a hand at the
    /// origin, so `PointerWatch` reads `seen` there too; what tells the two apart is `handAcross`, the stand-in itself.
    /// Nothing is lost by leaving the beat out of those runs — a pointer walk is the one thing they cannot inject at
    /// all (verify-ui スキル), so it is measured on a throwaway `qmltestrunner` scene instead.
    readonly property bool walkable: shared.handAcross < 0 && shared.hand.seen

    /// Puts the app's own card on it (デザイン規約 §背景 names `bgElevated` as the tooltip's ground), gives the seat and
    /// the beat their bindings, and hands the words a field that can be selected.
    function dressToolTip() {
        const tip = shared.sharedTip
        tip.background = tipGround.createObject(tip)
        tip.contentItem = tipWord.createObject(tip)
        tip.padding = Theme.spaceSm
        // Placed inside the window here rather than by `Popup`, which knows nothing about the target it is standing on
        // — asked to push a tip back in, it moves the seat and leaves the hand behind.
        tip.margins = -1
        // A press inside is how a reader starts a selection, and the style's policy reads a press on the tip as a press
        // outside the *target*. Escape still takes it down, the way it takes down every card here.
        tip.closePolicy = Popup.CloseOnEscape
        // **Bindings, not assignments**: a popup handed its words is not its final size in that same frame (規約 §hover
        // のツールチップ「出す前に採寸する」), and a tip that opens above its target cannot be placed without that height —
        // assigned, it would come out one tip too low every time.
        tip.x = Qt.binding(shared.seatX)
        tip.y = Qt.binding(shared.seatY)
    }

    /// The target's top-left corner in `host` coordinates. A method, so nothing binds to it — the callers below read
    /// the sizes and the anchor, which is what moves the seat.
    function targetAt() {
        const at = shared.sharedTip.parent
        return at !== null ? at.mapToItem(shared.host, 0, 0) : Qt.point(0, 0)
    }

    /// Beside the hand, and inside the window. The hand is where the tip was opened; with none to read (a tip raised
    /// from inside a menu, or a run with no pointer at all) the target's own middle stands in, which is where the
    /// style put every one of them.
    function seatX() {
        const tip = shared.sharedTip
        const at = tip.parent
        if (at === null)
            return 0
        const p = shared.targetAt()
        const mid = shared.anchorKnown ? shared.anchorX : p.x + at.width / 2
        const edge = Theme.spaceXs
        const room = shared.host.width - tip.implicitWidth - edge
        return Math.max(edge, Math.min(mid - tip.implicitWidth / 2, room)) - p.x
    }

    /// Flush against the target, above it while there is room and below it when there is not — the band's own controls
    /// are one tip's height from the top of the window, and a tip pushed back in would stand on the hand.
    function seatY() {
        const tip = shared.sharedTip
        const at = tip.parent
        if (at === null)
            return 0
        const p = shared.targetAt()
        return p.y >= tip.implicitHeight ? -tip.implicitHeight : at.height
    }

    /// Reads the hand's seat off, for a tip that is coming out now.
    ///
    /// **And again a turn later.** The tip comes out inside the same delivery that moved the pointer, and the window's
    /// watch is another item in that delivery — which of the two Qt reaches first is not written down anywhere (規約
    /// §hover のツールチップ の罠 (3), the same unordered pair the beat exists for). Read from the first one, a tip whose
    /// site opens it without a rest lands on the target's middle as though there were no hand at all (実測 2026-08-27:
    /// with `delay: 0` the seat came out at the row's centre, and again at the far edge with a position the hand had
    /// already left). The second read is before anything is drawn, so nothing moves on screen; by then the hand is
    /// still on the target, because the walk into the tip has not begun.
    function freeze() {
        shared.keeping = false
        keep.stop()
        shared.readAnchor()
        Qt.callLater(shared.settleAnchor)
    }
    function settleAnchor() {
        if (shared.sharedTip.visible && !shared.keeping)
            shared.readAnchor()
    }
    function readAnchor() {
        const at = shared.sharedTip.parent
        if (at === null) {
            shared.anchorKnown = false
            return
        }
        if (shared.handAcross >= 0) {
            shared.anchorKnown = true
            shared.anchorX = shared.targetAt().x + at.width * shared.handAcross
            return
        }
        shared.anchorKnown = shared.hand.known
        if (shared.anchorKnown)
            shared.anchorX = shared.hand.handX
    }

    /// Whether anything still wants the tip where it is: the hand inside it, the hand back on the target, or the
    /// target saying so itself (a `Control` under a menu, where the window's own watch cannot see the pointer).
    function wanted() {
        if (shared.pointed)
            return true
        const at = shared.sharedTip.parent
        if (at === null)
            return false
        if (at.hovered === true)
            return true
        return shared.hand.over(at)
    }

    function drop() {
        shared.dropping = true
        shared.sharedTip.close()
        shared.dropping = false
        shared.keeping = false
    }

    /// Puts it back after a fall that the hand may be walking into.
    ///
    /// **A turn later, never inside the fall itself.** `Popup` announces the fall from inside its own teardown and is
    /// still holding itself open at that moment, so an `open()` called from the handler is dropped on the floor without
    /// a word — the tip went and never came back (2026-08-27 実測: one `visible=true`, one `visible=false`, and no third
    /// line). `Qt.callLater` runs before the scene is drawn again, so nothing blinks.
    function reopen() {
        if (!shared.keeping || shared.sharedTip.visible)
            return
        // **Without the rest.** The wait before a tip is the question "was that a hand going past, or one that meant
        // it?", and a tip that is already out has been answered (規約 §hover のツールチップ「出ているものの的へ戻る手は
        // 待たせない」). Put back through the delay it would still be counting when the beat below ran out, and the beat
        // would find nothing on screen to be inside of and take it down under the hand (実測 2026-08-27). The attached
        // property writes the delay again on the next real ask, so this heals itself.
        shared.sharedTip.delay = 0
        shared.sharedTip.open()
        keep.restart()
    }

    property Timer keep: Timer {
        interval: Metrics.hoverKeepMs
        onTriggered: {
            if (!shared.wanted())
                shared.drop()
        }
    }

    // The hand reached the tip: the beat has nothing left to decide. Letting go starts it again.
    onPointedChanged: {
        if (shared.pointed)
            keep.stop()
        else if (shared.sharedTip.visible)
            keep.restart()
    }

    Connections {
        target: shared.sharedTip
        // A target letting go, and a target taking over. The first is answered with a beat rather than a close (see the
        // component); the second is a new seat to read.
        function onVisibleChanged() {
            if (shared.sharedTip.visible) {
                if (!shared.keeping)
                    shared.freeze()
                return
            }
            if (shared.dropping || !shared.walkable)
                return
            shared.keeping = true
            Qt.callLater(shared.reopen)
        }
        function onParentChanged() {
            if (shared.sharedTip.visible)
                shared.freeze()
        }
    }

    Component {
        id: tipGround
        Rectangle {
            color: Theme.bgElevated
            radius: Theme.radiusMd
            border.color: Theme.borderDefault
            border.width: Theme.borderWidth
            HoverHandler {
                id: groundHover
                onHoveredChanged: shared.groundPointed = groundHover.hovered
            }
        }
    }
    Component {
        id: tipWord
        CardText {
            text: shared.sharedTip.text
            pixelSize: Theme.fontMd
            color: Theme.textPrimary
            // Half the window, the same share a graph row's card holds its message to: a tooltip is one sentence
            // (規約 §hover のツールチップ), but the ones that carry a name carry it in full, and neither a path nor a ref
            // has a length worth trusting. What will not fit wraps — a name cut in the place it went to be read is a
            // name nobody can read anywhere.
            width: Math.min(implicitWidth, shared.host.width / 2)
            onPointedChanged: shared.wordPointed = pointed
        }
    }
}
