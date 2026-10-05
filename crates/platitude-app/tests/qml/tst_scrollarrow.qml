import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The arrows at the two ends of an upright bar (デザイン規約 §スクロールバーの矢印): how much of the bar they take on a
// box of any height, what a press, a hold and a slide off do, that a real press reaches them, that on a surface gliding
// its wheel (a message box, the left menu) a step rides that glide, and that in a message box the grip leaves the down
// arrow its whole end and taking the bar takes the box over. Each check waits for the view to have moved; no clock is
// read.
Item {
    id: root
    width: 640
    height: 480

    ListModel { id: rows }

    // A see-through bar (the default of `AppListView`) and a pane's slab, over the same rows.
    AppListView {
        id: floatList
        width: 150
        height: 300
        model: rows
        delegate: Item {
            width: 150
            height: 24
        }
    }
    AppListView {
        id: paneList
        x: 160
        width: 150
        height: 300
        model: rows
        verticalBar: PaneScrollBar {}
        delegate: Item {
            width: 150
            height: 24
        }
    }
    // A list made on a window already up, whose rows come after it stands: its bar is born with nowhere to go, arrows
    // and all — the diff list and the WIP buckets, whose rows arrive from git.
    ListModel { id: lateRows }
    Component {
        id: lateList
        AppListView {
            y: 320
            width: 150
            height: 100
            model: lateRows
            delegate: Item {
                width: 150
                height: 24
            }
        }
    }
    DescriptionBox {
        id: desc
        x: 320
        width: 280
        height: 84
        room: 200
        readOnly: true
    }
    // A section of the left menu, which glides its wheel notches (`NavList.sendRows`) and hands that glide to its bar.
    ListModel { id: navRows }
    NavList {
        id: navList
        x: 160
        y: 320
        width: 150
        height: 150
        sectionModel: navRows
    }

    TestCase {
        name: "ScrollArrow"
        when: windowShown

        readonly property var floatBar: floatList.ScrollBar.vertical
        readonly property var paneBar: paneList.ScrollBar.vertical
        readonly property var navBar: navList.ScrollBar.vertical

        function initTestCase() {
            for (let i = 0; i < 200; i++)
                rows.append({})
            // Every field a sidebar row reads, as the sections hand them.
            for (let i = 0; i < 40; i++)
                navRows.append({
                    "name": "b" + i, "full": "b" + i, "oid_hex": "", "change": "", "bucket": "", "orig_path": "",
                    "orig_name": "", "is_head": false, "has_remote": false, "only_remote": false, "has_pr": false,
                    "depth": 0, "folder": false, "eol_mark": false, "ahead": 0, "behind": 0
                })
            desc.text = Array.from({ length: 40 }, (_, i) => "line " + i).join("\n")
            tryVerify(() => floatBar.visible && paneBar.visible && desc.bar.visible && navBar.visible)
        }
        function init() {
            floatList.height = 300
            paneList.height = 300
            floatList.contentY = 0
            paneList.contentY = 0
            navList.contentY = 0
        }
        function cleanup() {
            floatBar.releaseArrow()
            paneBar.releaseArrow()
            desc.bar.releaseArrow()
            navBar.releaseArrow()
            paneBar.releaseTrack()
            navBar.releaseTrack()
            // A step still gliding would carry on into the next test's reset.
            for (const bar of [floatBar, paneBar, desc.bar, navBar])
                bar.stepGlide.halt()
        }

        /// Each end is the arrow's ink with the thumb's side air above and below it: the see-through arrow is 6 across
        /// and about 4.6 tall, the slab's 10 across (half of it sunk into the edge) and about 7.7 tall, the air 2.
        function test_each_end_is_the_arrow_and_the_thumbs_side_air() {
            compare(floatBar.leftPadding, 2)
            compare(floatBar.arrowEnd, 9)
            compare(floatBar.topPadding, 9)
            compare(floatBar.bottomPadding, 9)
            compare(paneBar.leftPadding, 2)
            compare(paneBar.arrowEnd, 12)
            compare(paneBar.topPadding, 12)
            compare(paneBar.bottomPadding, 12)
        }

        /// Shorter than its two ends, a bar gives each half its length and keeps both arrows; the thumb goes once
        /// nothing is left between them (Chromium, measured).
        function test_a_short_bar_squeezes_its_ends_and_drops_the_thumb_last() {
            floatList.height = 17
            tryCompare(floatBar, "arrowEnd", 8)
            verify(floatBar.hasTrack, "a pixel is left between the ends")
            verify(floatBar.contentItem.visible)
            floatList.height = 16
            tryCompare(floatBar, "arrowEnd", 8)
            verify(!floatBar.hasTrack)
            verify(!floatBar.contentItem.visible, "no thumb with nothing between the ends")
            paneList.height = 20
            tryCompare(paneBar, "arrowEnd", 10)
            verify(!paneBar.contentItem.visible)
            paneList.height = 300
            tryCompare(paneBar, "arrowEnd", 12)
            verify(paneBar.contentItem.visible, "the thumb comes back with the room")
        }

        function test_a_click_sends_one_step() {
            floatBar.pressArrow(1)
            floatBar.releaseArrow()
            tryVerify(() => !floatBar.stepping)
            compare(floatList.contentY, Metrics.arrowStep)
            floatBar.pressArrow(-1)
            floatBar.releaseArrow()
            tryVerify(() => !floatBar.stepping)
            compare(floatList.contentY, 0)
        }

        function test_a_step_stops_at_the_end() {
            const end = floatList.clampY(1e9)
            floatList.contentY = end - 10
            floatBar.pressArrow(1)
            floatBar.releaseArrow()
            tryVerify(() => !floatBar.stepping)
            compare(floatList.contentY, end)
        }

        /// Held, the arrow runs on past its step, holds the bar the way a drag does, and stops where it stands on
        /// letting go.
        function test_a_hold_runs_on_until_let_go() {
            paneBar.pressArrow(1)
            compare(Hand.heldBar, paneBar)
            tryVerify(() => paneList.contentY > Metrics.arrowStep + 1, undefined, "the run started past the step")
            verify(paneBar.arrowRunning)
            paneBar.releaseArrow()
            verify(!paneBar.arrowRunning)
            compare(Hand.heldBar, null)
        }

        /// Slid off with the hand still down, the run stops and does not come back when the hand does (Chromium,
        /// measured); the bar stays held.
        function test_sliding_off_stops_the_run_for_good() {
            paneBar.pressArrow(1)
            tryVerify(() => paneBar.arrowRunning, undefined, "the run started")
            paneBar.pointerOnArrow(false)
            verify(!paneBar.arrowRunning)
            paneBar.pointerOnArrow(true)
            // waits(paced): the subject is a run that must **not** come back, so the wait is the pause it would come
            // back on, twice over.
            wait(2 * Metrics.arrowRepeatDelayMs)
            verify(!paneBar.arrowRunning, "the run stays stopped")
            compare(paneBar.arrowHeld, 1)
            compare(Hand.heldBar, paneBar)
        }

        /// A press on the track pages once towards it: seven eighths of the view, glided (Chromium, measured).
        function test_a_click_on_the_track_sends_one_page() {
            compare(paneBar.trackPage, Math.floor(paneList.height * Metrics.trackPageShare))
            const below = paneBar.height - paneBar.arrowEnd - 2
            verify(paneBar.pressTrack(below))
            paneBar.releaseTrack()
            tryVerify(() => !paneBar.stepping)
            compare(paneList.contentY, paneBar.trackPage)
            paneList.contentY = 1000
            verify(paneBar.pressTrack(paneBar.arrowEnd + 1))
            paneBar.releaseTrack()
            tryVerify(() => !paneBar.stepping)
            compare(paneList.contentY, 1000 - paneBar.trackPage)
        }

        /// Held, the pages run on until the thumb's far end is on the hand — no further — and the bar is held the way
        /// a drag holds it.
        function test_a_held_track_stops_with_the_thumb_on_the_hand() {
            const hand = 250
            verify(paneBar.pressTrack(hand))
            compare(Hand.heldBar, paneBar)
            // Past the pause and out of the run: a run can start and end between two samples, so neither edge is waited
            // for on its own.
            tryVerify(() => !paneBar.trackWaiting && !paneBar.trackRunning && !paneBar.stepping, undefined,
                      "the run stopped")
            verify(Math.abs(paneBar.thumbBottom() - hand) <= 1,
                   "the thumb ends at " + paneBar.thumbBottom() + ", the hand is at " + hand)
            paneBar.releaseTrack()
            compare(Hand.heldBar, null)
        }

        /// A hand that moves while it holds an arrow or the track keeps it: the list the bar stands in must not take the
        /// press for a drag of its rows.
        function test_a_moving_hand_keeps_the_arrow_and_the_track() {
            const x = paneBar.width / 2
            const low = paneBar.height - 2
            mousePress(paneBar, x, low)
            compare(paneBar.arrowHeld, 1)
            mouseMove(paneBar, x, low - 60, -1, Qt.LeftButton)
            mouseMove(paneBar, x, low - 120, -1, Qt.LeftButton)
            compare(paneBar.arrowHeld, 1, "the arrow is still in the hand")
            verify(!paneList.dragging, "the list did not take the press")
            mouseRelease(paneBar, x, low - 120)
            tryVerify(() => !paneBar.stepping)
            paneList.contentY = 0
            const track = paneBar.height - paneBar.arrowEnd - 30
            mousePress(paneBar, x, track)
            compare(paneBar.trackHeld, 1)
            mouseMove(paneBar, x, track - 60, -1, Qt.LeftButton)
            mouseMove(paneBar, x, track - 120, -1, Qt.LeftButton)
            compare(paneBar.trackHeld, 1, "the track is still in the hand")
            verify(!paneList.dragging, "the list did not take the press")
            mouseRelease(paneBar, x, track - 120)
        }

        /// A thumb in the hand lights the bar, not the arrows past the lit step: only the part in use goes up to the
        /// held step (Chromium, measured: a pressed thumb leaves the arrows as they are).
        function test_the_thumb_in_hand_leaves_the_arrows_at_the_lit_step() {
            for (const bar of [paneBar, floatBar]) {
                const arrows = bar.children.filter(c => c.down !== undefined)
                const mid = bar.contentItem.y + bar.contentItem.height / 2
                mousePress(bar, bar.width / 2, mid)
                verify(bar.pressed, "the thumb took it")
                const lit = bar === paneBar ? Theme.borderDefault : bar.palette.mid
                for (const a of arrows)
                    verify(Qt.colorEqual(a.ink, lit), "an arrow wears " + a.ink + " with the thumb in the hand")
                mouseRelease(bar, bar.width / 2, mid)
            }
        }

        /// A press on an arrow and a wheel notch straight after it make one movement: the step glides on the box's own
        /// wheel glide, so the notch is measured from where the step is going, and every hand that halts the box's
        /// glide halts the step.
        function test_a_notch_after_an_arrow_adds_to_its_step() {
            desc.resetForNewMessage()
            desc.bar.pressArrow(1)
            desc.bar.releaseArrow()
            desc.rollBy(-120)
            tryCompare(desc, "textAt", Metrics.arrowStep + desc.wheelStep)
        }

        /// The left menu's bar steps on the list's own wheel glide too: a notch down straight after an arrow's step is
        /// measured from where the step is going. Two glides of their own would each run to their own aim, and the
        /// notch would land where it would have from the top.
        function test_a_notch_after_a_left_menu_arrow_adds_to_its_step() {
            const notch = Metrics.wheelRows * Theme.rowHeight
            navBar.pressArrow(1)
            navBar.releaseArrow()
            navList.sendRows(-notch)
            tryCompare(navList, "contentY", Metrics.arrowStep + notch)
        }

        /// And a page from the left menu's track stops with the list's glide: every hand that halts it (a row brought
        /// into view, an opening shown or given back, the middle button's drift — `NavList.haltGlide`) halts the page,
        /// or the page drags the list on to where it was going.
        function test_halting_the_left_menus_glide_halts_a_page_from_its_track() {
            verify(navBar.pressTrack(navBar.height - navBar.arrowEnd - 2))
            navBar.releaseTrack()
            verify(navBar.stepping, "the page is gliding")
            navList.haltGlide()
            verify(!navBar.stepping, "and stopped with the list's glide")
        }

        /// Slid off the bar before the pause is over, the run never starts, and a hand coming back does not start it
        /// (Chromium, measured).
        function test_sliding_off_the_track_stops_it_for_good() {
            verify(paneBar.pressTrack(paneBar.height - paneBar.arrowEnd - 2))
            paneBar.pointerOnTrack(false, 0)
            verify(!paneBar.trackWaiting && !paneBar.trackRunning)
            paneBar.pointerOnTrack(true, paneBar.height - paneBar.arrowEnd - 2)
            // waits(paced): the subject is a run that must **not** start, so the wait is the pause it would start after,
            // twice over.
            wait(2 * Metrics.arrowRepeatDelayMs)
            verify(!paneBar.trackRunning, "the run stays stopped")
            compare(paneBar.trackHeld, 1)
        }

        /// A real press off the thumb is the track's; one on the thumb is the thumb's own drag.
        function test_a_press_on_the_track_pages_and_one_on_the_thumb_drags() {
            const below = paneBar.height - paneBar.arrowEnd - 2
            mousePress(paneBar, paneBar.width / 2, below)
            compare(paneBar.trackHeld, 1)
            mouseRelease(paneBar, paneBar.width / 2, below)
            compare(paneBar.trackHeld, 0)
            tryVerify(() => !paneBar.stepping)
            compare(paneList.contentY, paneBar.trackPage)
            const onThumb = (paneBar.thumbTop() + paneBar.thumbBottom()) / 2
            mousePress(paneBar, paneBar.width / 2, onThumb)
            verify(paneBar.pressed, "the thumb took it")
            compare(paneBar.trackHeld, 0)
            mouseMove(paneBar, paneBar.width / 2, onThumb + 20)
            verify(paneList.contentY > paneBar.trackPage, "and the drag moved the rows")
            mouseRelease(paneBar, paneBar.width / 2, onThumb + 20)
        }

        /// Arrows born in a bar with nowhere to go are drawn when the rows arrive and the bar comes up — or a picture
        /// waits on their ink for good (`Ink.owed`).
        function test_arrows_born_in_a_hidden_bar_are_drawn_when_it_comes_up() {
            // Laid out later than it is made, as a pane on a page not yet shown: the arrows come into the scene with no
            // size, and take it while the bar is down.
            const list = createTemporaryObject(lateList, root, { height: 0 })
            const bar = list.ScrollBar.vertical
            verify(!bar.visible, "no rows, nowhere to go")
            const arrows = bar.children.filter(c => c.down !== undefined)
            compare(arrows.length, 2)
            // A frame drawn with the arrows in the scene and no size, and another once they have it: the passes a
            // first paint would have been asked for in.
            verify(waitForRendering(list), "the list was rendered with no size")
            list.height = 100
            tryCompare(bar, "arrowEnd", 9)
            verify(waitForRendering(list), "the list was rendered with its size, the bar down")
            for (let i = 0; i < 40; i++)
                lateRows.append({})
            tryVerify(() => bar.visible, undefined, "the rows came and the bar is up")
            tryVerify(() => arrows.every(a => a.inked), undefined, "both arrows drawn")
        }

        /// Only the part in use lights: the held arrow takes the held step, the other keeps the slab's.
        function test_the_held_arrow_alone_takes_the_held_ink() {
            paneBar.pressArrow(1)
            const up = paneBar.children.filter(c => c.down === false)[0]
            const down = paneBar.children.filter(c => c.down === true)[0]
            verify(Qt.colorEqual(down.ink, Theme.borderStrong))
            verify(Qt.colorEqual(up.ink, paneBar.slabColor))
        }

        /// A real press on either end reaches the arrow, not the track under it.
        function test_a_press_on_an_end_is_the_arrow() {
            mousePress(paneBar, paneBar.width / 2, paneBar.height - 2)
            compare(paneBar.arrowHeld, 1)
            mouseRelease(paneBar, paneBar.width / 2, paneBar.height - 2)
            compare(paneBar.arrowHeld, 0)
            tryVerify(() => !paneBar.stepping)
            compare(paneList.contentY, Metrics.arrowStep)
            mousePress(paneBar, paneBar.width / 2, 2)
            compare(paneBar.arrowHeld, -1)
            mouseRelease(paneBar, paneBar.width / 2, 2)
            tryVerify(() => !paneBar.stepping)
            compare(paneList.contentY, 0)
        }

        /// With the grip offered the bar ends at the mark's highest ink — seven pixels into the text's inset — so the
        /// down arrow stands wholly above the grip; the grip's square is split along the bar: what the bar covers is
        /// the arrow's, the rest the grip's, and nothing outside the square is the grip's.
        function test_the_down_arrow_stands_over_the_grip_and_shares_its_square() {
            verify(desc.grips)
            compare(desc.gripYield, 7)
            compare(desc.bar.height, desc.height - 2 * Theme.spaceXs - desc.gripYield)
            mousePress(desc.bar, desc.bar.width / 2, desc.bar.height - 1)
            compare(desc.bar.arrowHeld, 1, "the air over the mark is the arrow's")
            mouseRelease(desc.bar, desc.bar.width / 2, desc.bar.height - 1)
            tryVerify(() => !desc.bar.stepping)
            // The rest of the grip's square is the grip's: here left of the bar's column, down beside the long stroke.
            mousePress(desc, desc.width - 16 + 4, desc.height - 16 + 9)
            verify(desc.gripDragging, "the square beside the arrow is the grip's")
            mouseRelease(desc, desc.width - 16 + 4, desc.height - 16 + 9)
            // And nothing outside the square is: the mask is the grip's whole hit test.
            mousePress(paneBar, paneBar.width / 2, paneBar.height - 2)
            verify(!desc.gripDragging, "a press on another list's arrow is not the grip's")
            compare(paneBar.arrowHeld, 1)
            mouseRelease(paneBar, paneBar.width / 2, paneBar.height - 2)
            // On the long stroke (x + y = 18 on the mark's grid, the mark in the box's last 16px).
            mousePress(desc, desc.width - 7, desc.height - 7)
            verify(desc.gripDragging, "the mark is the grip's")
            compare(desc.bar.arrowHeld, 0)
            mouseRelease(desc, desc.width - 7, desc.height - 7)
        }

        /// A new message holds the box at its top until the reader moves it; taking the bar is moving it.
        function test_an_arrow_takes_a_pinned_box_over() {
            desc.resetForNewMessage()
            desc.bar.pressArrow(1)
            desc.bar.releaseArrow()
            tryVerify(() => !desc.bar.stepping)
            compare(desc.textAt, Metrics.arrowStep)
        }

        function test_dragging_the_thumb_takes_a_pinned_box_over() {
            desc.resetForNewMessage()
            const thumb = desc.bar.contentItem
            const at = thumb.mapToItem(desc.bar, thumb.width / 2, thumb.height / 2)
            mouseDrag(desc.bar, at.x, at.y, 0, 20)
            verify(desc.textAt > 0, "the thumb moved the text")
        }
    }
}
