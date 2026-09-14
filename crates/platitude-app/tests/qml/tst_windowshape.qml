import QtQuick
import QtTest
import platitude.ui

// Which screen a saved place belongs to, held against desktops this machine does not have.
//
// **The arithmetic is the whole of the restore's premise.** Everything after it — the width the size is kept inside,
// the work area the frame is pulled into — is measured against the screen this answers with, and before it the answer
// was "wherever the platform has just put the window", which on Windows is the screen the pointer is on. A desktop
// with a monitor left of the origin, one at a different scale and one that has been unplugged cannot be arranged on
// the machine a test runs on, so they are arranged here.
Item {
    id: root
    width: 200
    height: 200

    /// Two monitors side by side, the second one starting where the first ends — and a third above and to the left,
    /// which is where negative coordinates come from.
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
            compare(shape.screenHolding(240, 90, root.desktop).name, "\\\\.\\DISPLAY1")
            compare(shape.screenHolding(2160, 66, root.desktop).name, "\\\\.\\DISPLAY2")
        }

        /// A monitor above and to the left of the primary: its coordinates are negative, and a restore that read them
        /// as "off the desktop" would put the window somewhere nobody left it.
        function test_a_place_at_negative_coordinates_is_on_its_own_screen() {
            compare(shape.screenHolding(-1200, -1000, root.desktop).name, "\\\\.\\DISPLAY3")
            compare(shape.screenHolding(-1, -1, root.desktop).name, "\\\\.\\DISPLAY3")
        }

        /// **Scale is not in this arithmetic.** Qt lays every screen out in one coordinate space whatever each is
        /// scaled by, so the saved place is compared with those coordinates and nothing is multiplied — and what
        /// crosses to the platform side is the screen's name, because that space is Qt's own and Windows need not
        /// agree with it about where a point on a scaled monitor is. Whether the two do agree is a thing only two
        /// real monitors can say (P3-確認事項).
        function test_a_screen_at_another_scale_is_found_by_its_place_all_the_same() {
            compare(shape.screenHolding(2160, 66, root.desktop).name, "\\\\.\\DISPLAY2")
            compare(shape.screenHolding(-1200, -1000, root.desktop).name, "\\\\.\\DISPLAY3")
        }

        /// The edges, said once: a screen holds its own origin and not the first pixel of the next one along.
        function test_the_seam_between_two_screens_belongs_to_the_one_on_the_right() {
            compare(shape.screenHolding(1919, 0, root.desktop).name, "\\\\.\\DISPLAY1")
            compare(shape.screenHolding(1920, 0, root.desktop).name, "\\\\.\\DISPLAY2")
        }

        /// The two shapes with no answer, which the restore reads as "take the window's own nearest monitor": a first
        /// run, and a place on a monitor that is not here any more. **Not the same as being off the desktop** — both
        /// end with the platform side fitting the window to the nearest monitor rather than leaving it where nobody
        /// can reach it.
        function test_an_unsaved_place_and_a_screen_that_has_gone_answer_with_nothing() {
            compare(shape.screenHolding(shape.unplaced, shape.unplaced, root.desktop), null)
            compare(shape.screenHolding(240, shape.unplaced, root.desktop), null)
            compare(shape.screenHolding(6000, 200, root.desktop), null)
            compare(shape.screenHolding(240, 90, []), null)
        }
    }
}
