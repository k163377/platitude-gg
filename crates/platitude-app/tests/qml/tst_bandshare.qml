import QtQuick
import QtTest
import platitude.ui

// How the band hands its shortfall out among the state badges, and which of the group's three shapes that leaves
// (デザイン規約 §ウィンドウの縁 の譲る順).
//
// The arithmetic only: which widths go in is the font's and the model's (`PGG_AUTO_ACT=badges`). A window width does
// not name a shape — the same number folds on one platform's font and keeps its words on another's.
Item {
    id: root
    width: 400
    height: 200

    /// Costs named rather than measured, so the boundaries do not move with the platform font. Shaped like the
    /// real ones, not a claim about any machine's.
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

        /// Both sides of the boundary: a rule capping one pixel early would narrow a band that had the room.
        function test_a_group_that_fits_is_not_narrowed() {
            const natural = 80 + 71 + 68 + 2 * root.gap
            compare(share.cap(root.three, natural), share.uncapped)
            compare(share.cap(root.three, natural + 40), share.uncapped)
            verify(share.cap(root.three, natural - 1) < share.uncapped,
                   "a pixel short is short")
        }

        /// The widest gives way first and alone: the two under their share keep their own width.
        function test_the_widest_badge_is_the_one_that_gives_way() {
            // Room for 71 + 68 and a share of 75 for the third.
            const room = 71 + 68 + 75 + 2 * root.gap
            compare(share.cap(root.three, room), 75)
        }

        /// The max-min share, not a flat scaling: the short badge keeps its 50 while the two long ones come down to
        /// 65 together.
        function test_the_ones_that_give_way_come_out_equal() {
            compare(share.cap([80, 71, 50], 188), 65)
        }

        function test_a_room_under_every_share_caps_them_all_alike() {
            compare(share.cap(root.three, 196), 62)
        }

        function test_one_badge_and_none() {
            compare(share.cap([], 1000), share.uncapped, "nothing standing asks for nothing")
            compare(share.cap([80], 80), share.uncapped, "one badge with its own width fits")
            compare(share.cap([80], 60), 60, "and is cut to the room when it does not")
        }

        /// The caller's list is left unsorted: the row reads it afterwards in its own order.
        function test_the_order_in_does_not_change_the_answer_or_the_list() {
            const room = 68 + 2 * 60 + 2 * root.gap
            const given = [68, 80, 71]
            compare(share.cap(given, room), share.cap(root.three, room))
            compare(given.join(","), "68,80,71", "the row's own order survives the sort")
        }

        /// A fractional cap comes out over what the boxes are laid out at, and every word then elides in a band that
        /// had the room.
        function test_the_cap_is_whole_pixels() {
            const cap = share.cap(root.three, 68 + 2 * 60 + 2 * root.gap + 0.75)
            compare(cap, Math.floor(cap), "a box is laid out on a whole pixel")
        }

        // ---- which shape that leaves --------------------------------------------

        function test_the_words_go_one_pixel_under_the_floor() {
            verify(!share.folded(root.badgeMin, 0, 100, 0, false), "at the floor the words stand")
            verify(share.folded(root.badgeMin - 1, 0, 100, 0, false), "under it they do not")
            verify(!share.folded(share.uncapped, 0, 100, 0, false), "and a group nobody narrowed keeps them")
        }

        function test_a_crowded_strip_folds_a_group_that_was_never_narrowed() {
            // Six tabs wanting 600 in a run of 200 — two of them stand.
            verify(share.folded(share.uncapped, 600, 200, 6, false))
            compare(share.tabsInView(600, 200, 6), 2)
            // The same crowd with room for three is not a crowd.
            verify(!share.folded(share.uncapped, 600, 300, 6, false))
            compare(share.tabsInView(600, 300, 6), 3)
        }

        /// The tab count is only asked of a scrolling strip.
        function test_a_strip_with_room_never_folds_the_group() {
            verify(!share.folded(share.uncapped, 100, 400, 1, false),
                   "one tab in a run four times its width is not a crowd")
            compare(share.tabsInView(0, 400, 0), 3, "and a strip with no tabs at all answers for three")
        }

        function test_the_floor_takes_the_words_whatever_else_is_true() {
            verify(share.folded(share.uncapped, 100, 400, 1, true))
        }
    }
}
