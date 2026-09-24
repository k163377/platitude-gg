import QtQuick
import QtTest
import platitude.ui

// How the band hands its shortfall out among the state badges, and which of the group's three shapes that leaves
// (デザイン規約 §ウィンドウの縁 の譲る順).
//
// **The arithmetic and nothing else.** Which widths go in is the font's and the model's business and the headless run
// proves it (`PGG_AUTO_ACT=badges`); what is left once the widths are in hand is a function of numbers, asked here
// for the price of a function call instead of a window, a repository with a stopped merge in it, and a tab per name.
//
// A window width does not name a shape: the same number lands in different ones on different fonts. Measured
// 2026-09-18, one width meant for the folded shape folded on Windows (`cap=31` under a floor of 33) and kept its words
// on Linux (`cap=37` over a floor of 35) — a verb that reached the shapes by naming window widths would never
// photograph that shape on Linux, and nothing would say so unless the shape is in a `must_say`.
Item {
    id: root
    width: 400
    height: 200

    /// The costs the group works in, named here rather than measured: what a boundary case is about is the
    /// arithmetic on either side of it, and a cost read off the platform's UI font moves the boundary between
    /// machines — which is what would make these cases a window's to answer. They are the shape of the
    /// real ones (Windows offscreen: `badgeMin=33`, a three-badge group asking for 227), not a claim about any
    /// machine's.
    readonly property real gap: 4
    readonly property real badgeMin: 33
    /// One stopped merge, its conflicts and an unset identity, at the widths they ask for.
    readonly property var three: [80, 71, 68]

    BandStateShare {
        id: share
        gap: root.gap
        badgeMinW: root.badgeMin
    }

    TestCase {
        name: "BandShare"

        // ---- the room handed out -----------------------------------------------

        /// Room to spare is answered with no cap at all, which is what the group draws every word at. **Both sides of
        /// the boundary**, since a rule that capped one pixel early would narrow a band that had the room.
        function test_a_group_that_fits_is_not_narrowed() {
            // 80 + 71 + 68 and two gaps.
            const natural = 80 + 71 + 68 + 2 * root.gap
            compare(share.cap(root.three, natural), share.uncapped)
            compare(share.cap(root.three, natural + 40), share.uncapped)
            verify(share.cap(root.three, natural - 1) < share.uncapped,
                   "a pixel short is short")
        }

        /// The widest gives way first and alone: at a room one pixel under what they all want, the two that already
        /// sit under their share keep their own width and the cap lands on the longest.
        function test_the_widest_badge_is_the_one_that_gives_way() {
            // Room for 71 + 68 and a share of 75 for the third.
            const room = 71 + 68 + 75 + 2 * root.gap
            compare(share.cap(root.three, room), 75)
        }

        /// The ones that give way come out **equal**, and the one already under its share keeps its own width — the
        /// max-min share, not a flat scaling. A short badge beside two long ones is drawn at 50 while they come down
        /// to 65 together, out of a room that would give each of them 62 if it were shared flat.
        function test_the_ones_that_give_way_come_out_equal() {
            compare(share.cap([80, 71, 50], 188), 65)
        }

        /// And where even the shortest is over its share, the cap is that share and every badge takes it.
        function test_a_room_under_every_share_caps_them_all_alike() {
            compare(share.cap(root.three, 196), 62)
        }

        /// A single badge is its own share, and an empty group has nothing to narrow.
        function test_one_badge_and_none() {
            compare(share.cap([], 1000), share.uncapped, "nothing standing asks for nothing")
            compare(share.cap([80], 80), share.uncapped, "one badge with its own width fits")
            compare(share.cap([80], 60), 60, "and is cut to the room when it does not")
        }

        /// The order the badges arrive in is the row's, not the arithmetic's — and the caller's list is left alone,
        /// because the row reads it afterwards in that same order.
        function test_the_order_in_does_not_change_the_answer_or_the_list() {
            const room = 68 + 2 * 60 + 2 * root.gap
            const given = [68, 80, 71]
            compare(share.cap(given, room), share.cap(root.three, room))
            compare(given.join(","), "68,80,71", "the row's own order survives the sort")
        }

        /// Whole pixels out of a fractional room: a cap summed off fractions comes out over what the boxes are laid
        /// out at, and every word then elides in a band that had the room (`BandStateMetrics` carries the reading).
        function test_the_cap_is_whole_pixels() {
            const cap = share.cap(root.three, 68 + 2 * 60 + 2 * root.gap + 0.75)
            compare(cap, Math.floor(cap), "a box is laid out on a whole pixel")
        }

        // ---- which shape that leaves --------------------------------------------

        /// The floor is where the words go, and **the boundary is the claim**: at exactly the floor a badge still
        /// says something, one pixel under it the group is a mark. This is the shape the width-named runs reached on
        /// one platform and missed on the other.
        function test_the_words_go_one_pixel_under_the_floor() {
            verify(!share.folded(root.badgeMin, 0, 100, 0, false), "at the floor the words stand")
            verify(share.folded(root.badgeMin - 1, 0, 100, 0, false), "under it they do not")
            verify(!share.folded(share.uncapped, 0, 100, 0, false), "and a group nobody narrowed keeps them")
        }

        /// The strip's crowd folds the group without narrowing it at all: two of the three ways in are the strip's,
        /// and a group with all the room in the world gives its words up beside a strip that cannot show three tabs.
        function test_a_crowded_strip_folds_a_group_that_was_never_narrowed() {
            // Six tabs wanting 600 in a run of 200 — two of them stand.
            verify(share.folded(share.uncapped, 600, 200, 6, false))
            compare(share.tabsInView(600, 200, 6), 2)
            // The same crowd with room for three is not a crowd.
            verify(!share.folded(share.uncapped, 600, 300, 6, false))
            compare(share.tabsInView(600, 300, 6), 3)
        }

        /// A strip that is not scrolling at all never folds the group, however few tabs it holds — the count is only
        /// asked once the names have run out of room.
        function test_a_strip_with_room_never_folds_the_group() {
            verify(!share.folded(share.uncapped, 100, 400, 1, false),
                   "one tab in a run four times its width is not a crowd")
            compare(share.tabsInView(0, 400, 0), 3, "and a strip with no tabs at all answers for three")
        }

        /// The window on its floor gives the words up whatever else is true (規約 §ウィンドウの縁).
        function test_the_floor_takes_the_words_whatever_else_is_true() {
            verify(share.folded(share.uncapped, 100, 400, 1, true))
        }
    }
}
