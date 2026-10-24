import QtQuick
import QtTest
import platitude.ui

// A press on a face its hand covers, carried as a point to the words beside it (`RowHand` → `LineText.anchorFrom` /
// `extendFrom`), and a head-cut line handed points of another item directly, as `SweepRoom` hands them. The words
// stand at neither the root's origin nor the hand's, and every point has x != y, so a point read in the wrong item or
// with its axes crossed lands on other characters. Characters are told apart by the mono face's one advance.
Item {
    id: root
    width: 400
    height: 200

    property int taps: 0
    property int takes: 0

    LineText {
        id: words
        x: 150
        y: 60
        width: 200
        mono: true
        grabbable: false
        text: "abcdefghijklmnop"
    }
    /// Wider than the words and starting before them, so a point of the hand is never the same point of the words.
    Item {
        id: face
        x: 100
        y: 40
        width: 280
        height: 60
        RowHand {
            id: hand
            field: words
            onTapped: root.taps++
            onTaken: root.takes++
        }
    }
    /// Held by its head cut, so its field hangs off the near edge (negative `field.x`).
    LineText {
        id: cut
        x: 40
        y: 140
        width: 60
        mono: true
        grabbable: false
        cutAt: "start"
        text: "abcdefghijklmnop"
    }
    TextMetrics {
        id: glyph
        font.family: Theme.monoFamily
        font.pixelSize: Theme.fontMd
        text: "a"
    }

    TestCase {
        name: "RowHand"
        when: windowShown

        function init() {
            root.taps = 0
            root.takes = 0
            words.deselect()
            cut.deselect()
        }

        /// A quarter into the character after boundary `i` of the words, in the hand's coordinates, `dy` off the
        /// line's middle. Whole pixels, as a pointer delivers them.
        function handPoint(i, dy) {
            const p = words.mapToItem(hand, (i + 0.25) * glyph.advanceWidth, words.height / 2 + dy)
            return Qt.point(Math.round(p.x), Math.round(p.y))
        }
        function inRoot(p) {
            return hand.mapToItem(root, p)
        }

        function test_a_drag_anchors_where_the_press_landed() {
            const from = handPoint(4, 0)
            // Below the line: a one-line field answers with the line (`LineText.onLine`).
            const to = handPoint(9, 20)
            verify(from.x !== from.y, "the press has x != y")
            const a = inRoot(from)
            const b = inRoot(to)
            mousePress(root, a.x, a.y)
            compare(root.takes, 1, "the press was taken before it was decided")
            compare(hand.pressPosition.x, from.x, "the press is kept in the hand's own x")
            compare(hand.pressPosition.y, from.y, "and its own y")
            // One move, straight past the threshold: the anchor is the press, not where the drag was decided.
            mouseMove(root, b.x, b.y)
            verify(hand.dragging, "the press became a drag")
            mouseRelease(root, b.x, b.y)
            compare(words.selected, "efghi")
            compare(root.taps, 0, "a drag is not a tap")
        }

        function test_a_press_inside_the_threshold_on_both_axes_is_a_tap() {
            const from = handPoint(4, 0)
            const step = hand.drag.threshold - 1
            const a = inRoot(from)
            mousePress(root, a.x, a.y)
            mouseMove(root, a.x + step, a.y + step)
            verify(!hand.dragging, "a tremor inside the threshold is still a press")
            mouseRelease(root, a.x + step, a.y + step)
            compare(root.taps, 1)
            compare(words.selected, "", "and it selected nothing")
        }

        function test_travel_on_one_axis_alone_is_a_drag_data() {
            return [
                { tag: "across", dx: 1, dy: 0 },
                { tag: "down", dx: 0, dy: 1 },
            ]
        }
        function test_travel_on_one_axis_alone_is_a_drag(data) {
            const from = handPoint(4, 0)
            const t = hand.drag.threshold
            const a = inRoot(from)
            mousePress(root, a.x, a.y)
            mouseMove(root, a.x + data.dx * t, a.y + data.dy * t)
            verify(hand.dragging, "the threshold is reached on one axis")
            mouseRelease(root, a.x + data.dx * t, a.y + data.dy * t)
            compare(root.taps, 0, "a drag is not a tap")
        }

        function test_the_press_is_kept_as_a_copy() {
            const from = handPoint(2, 0)
            const press = Qt.point(from.x, from.y)
            hand.takeAt(press)
            press.x = 0
            press.y = 0
            compare(hand.pressPosition.x, from.x, "changing the caller's point does not move the press")
            compare(hand.pressPosition.y, from.y)
            hand.followAt(handPoint(6, 0))
            hand.releaseNow()
            compare(words.selected, "cdef")
        }

        // The field of a head cut hangs off the near edge; a point of another item lands on the characters drawn
        // under it, not on the ones the field would have there unmoved.
        function test_a_head_cut_takes_the_characters_drawn_under_the_points() {
            verify(cut.cutsHead, "the line is cut at its head")
            const w = glyph.advanceWidth
            // Where the words end, in the line's coordinates (`LineText.tailSlack`).
            const end = cut.width - cut.tailSlack
            const past = cut.mapToItem(root, end + 2, cut.height / 2)
            const back = cut.mapToItem(root, end - 3.75 * w, cut.height / 2 + 7)
            cut.anchorFrom(root, past)
            cut.extendFrom(root, back)
            compare(cut.selected, "mnop")
        }
    }
}
