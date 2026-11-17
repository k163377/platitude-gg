import QtQuick
import QtTest
import platitude.ui

// Which screen a saved place belongs to, held against desktops this machine does not have: everything the restore
// does after it is measured against the screen this answers with. And the length each side opens at on that screen.
Item {
    id: root
    width: 200
    height: 200

    /// Two monitors side by side and a third above and to the left, where negative coordinates come from.
    readonly property var desktop: [
        { name: "\\\\.\\DISPLAY1", virtualX: 0, virtualY: 0, width: 1920, height: 1080, devicePixelRatio: 1 },
        { name: "\\\\.\\DISPLAY2", virtualX: 1920, virtualY: -24, width: 2560, height: 1440, devicePixelRatio: 1.5 },
        { name: "\\\\.\\DISPLAY3", virtualX: -1280, virtualY: -1024, width: 1280, height: 1024, devicePixelRatio: 2 }
    ]

    WindowShape {
        id: shape
        window: QtObject {}
    }

    TestCase {
        name: "WindowShape"

        function test_a_saved_place_answers_with_the_screen_it_is_on() {
            compare(shape.screenHolding(Qt.point(240, 90), root.desktop).name, "\\\\.\\DISPLAY1")
            compare(shape.screenHolding(Qt.point(2160, 66), root.desktop).name, "\\\\.\\DISPLAY2")
        }

        /// A restore that read negative coordinates as "off the desktop" would put the window somewhere nobody left it.
        function test_a_place_at_negative_coordinates_is_on_its_own_screen() {
            compare(shape.screenHolding(Qt.point(-1200, -1000), root.desktop).name, "\\\\.\\DISPLAY3")
            compare(shape.screenHolding(Qt.point(-1, -1), root.desktop).name, "\\\\.\\DISPLAY3")
        }

        /// Qt lays every screen out in one space whatever its scale, so nothing is multiplied; what crosses to the
        /// platform is the name (`WindowShape.screenHolding`, P3-確認事項).
        function test_a_screen_at_another_scale_is_found_by_its_place_all_the_same() {
            compare(shape.screenHolding(Qt.point(2160, 66), root.desktop).name, "\\\\.\\DISPLAY2")
            compare(shape.screenHolding(Qt.point(-1200, -1000), root.desktop).name, "\\\\.\\DISPLAY3")
        }

        function test_the_seam_between_two_screens_belongs_to_the_one_on_the_right() {
            compare(shape.screenHolding(Qt.point(1919, 0), root.desktop).name, "\\\\.\\DISPLAY1")
            compare(shape.screenHolding(Qt.point(1920, 0), root.desktop).name, "\\\\.\\DISPLAY2")
        }

        /// No answer = a first run or an unplugged monitor; the platform side then fits the window to its nearest one.
        function test_an_unsaved_place_and_a_screen_that_has_gone_answer_with_nothing() {
            compare(shape.screenHolding(Qt.point(shape.unplaced, shape.unplaced), root.desktop), null)
            compare(shape.screenHolding(Qt.point(240, shape.unplaced), root.desktop), null)
            compare(shape.screenHolding(Qt.point(6000, 200), root.desktop), null)
            compare(shape.screenHolding(Qt.point(240, 90), []), null)
        }

        /// A share and not a size: a large screen gets a large window, a small one a small window.
        function test_a_window_never_sized_takes_a_share_of_each_side() {
            compare(shape.firstLength(shape.unplaced, 3840), Math.round(3840 * Metrics.firstWindowShare))
            compare(shape.firstLength(shape.unplaced, 1080), Math.round(1080 * Metrics.firstWindowShare))
            verify(shape.firstLength(shape.unplaced, 3840) > shape.firstLength(shape.unplaced, 1920))
        }

        /// A saved length comes back as it was, kept inside the screen it opens on.
        function test_a_saved_length_is_kept_inside_the_screen() {
            compare(shape.firstLength(1440, 3840), 1440)
            compare(shape.firstLength(1440, 1093), 1093)
        }
    }
}
