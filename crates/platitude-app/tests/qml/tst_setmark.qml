import QtQuick
import QtTest
import platitude.ui

// Where a mark set beside words lands against the letters of their face (デザイン規約 §アイコンの席), read off the
// pixels of both: a mark at a row's head on the capitals' middle whatever its height, a sign on the face's own `+`'s
// middle and never under the baseline, the tree on the capitals' middle in every class, a heading's mark a step larger.
// The witness is the face's own `H` and `+` drawn beside the mark, not the numbers the mark is placed by.
Item {
    id: root
    width: 400
    height: 200

    /// Four times the window's `fontMd`, so a pixel of antialiasing is a quarter of one at the window's size.
    readonly property int px: Theme.fontMd * 4

    Component {
        id: pairOf
        // Straight under the root, away from its corner: `grabImage` cuts the window at the item's own x / y.
        Item {
            id: pair
            property string letter: "H"
            property alias mark: mark
            property alias words: words
            x: 8
            y: 8
            width: words.width + Theme.spaceXs + mark.width
            height: words.height
            Text {
                id: words
                text: pair.letter
                color: "black"
                font.family: Theme.uiFamily
                font.pixelSize: root.px
            }
            // The mirror box: the words' line, their baseline as its own.
            NavIcon {
                id: mark
                x: words.width + Theme.spaceXs
                width: root.px * 2
                height: words.height
                wordBase: words.baselineOffset
                size: root.px
                tint: "black"
            }
        }
    }

    TestCase {
        name: "SetMark"
        when: windowShown

        /// Where the ink starts and ends down the columns `from`–`to` of a picture, to a fraction of a pixel.
        function inkDown(picture, from, to, height) {
            const cover = new Array(height).fill(0)
            for (let x = from; x < to; x++) {
                for (let y = 0; y < height; y++) {
                    const c = picture.pixel(x, y)
                    cover[y] = Math.max(cover[y], 1 - (c.r + c.g + c.b) / 3)
                }
            }
            let first = -1
            let last = -1
            for (let y = 0; y < height; y++) {
                if (cover[y] > 0.02) {
                    if (first < 0)
                        first = y
                    last = y
                }
            }
            return { top: first + 1 - cover[first], bottom: last + cover[last] }
        }
        /// The letter's ink and the mark's, down the pair's own pixels.
        function inks(set, kind, letter, turn) {
            const pair = createTemporaryObject(pairOf, root, { letter: letter })
            pair.mark.set = set
            pair.mark.kind = kind
            pair.mark.turn = turn || 0
            tryVerify(() => pair.mark.inked)
            const picture = grabImage(pair)
            const words = inkDown(picture, 0, Math.ceil(pair.words.width), Math.round(pair.height))
            const mark = inkDown(picture, Math.floor(pair.mark.x), Math.floor(pair.mark.x + pair.mark.width),
                                 Math.round(pair.height))
            // Out of the next picture: a test that reads two pairs grabs both at the same x / y.
            pair.visible = false
            return { words: words, mark: mark }
        }
        function middle(ink) {
            return (ink.top + ink.bottom) / 2
        }

        function test_a_head_stands_on_the_capitals_middle_whatever_its_height_data() {
            return [{ tag: "squat stash", kind: "stash" }, { tag: "tall branch", kind: "branch" },
                    { tag: "flat minus", kind: "minus" }]
        }
        function test_a_head_stands_on_the_capitals_middle_whatever_its_height(data) {
            const ink = inks("lead", data.kind, "H")
            fuzzyCompare(middle(ink.mark), middle(ink.words), 1, data.kind + " against the `H`")
        }

        /// Qt answers the plus with no foot, so it is taken as square (`WordFace.plus`): a pixel and a half here,
        /// under 0.4 of one at the window's size.
        function test_a_sign_stands_on_the_face_s_own_plus() {
            const ink = inks("sign", "plus", "+")
            fuzzyCompare(middle(ink.mark), middle(ink.words), 1.5, "the drawn plus against the face's")
        }

        /// A sign taller than the room under its middle stops with its foot on the baseline: the band under it is the
        /// descenders'.
        function test_a_tall_sign_stops_at_the_baseline() {
            const ink = inks("sign", "branch", "H")
            fuzzyCompare(ink.mark.bottom, ink.words.bottom, 1, "the foot on the `H`'s foot")
        }

        /// The tree stands where the branch does, a sign's middle or not.
        function test_the_tree_stands_on_the_capitals_middle_as_a_sign_too() {
            const ink = inks("sign", "tree", "H")
            fuzzyCompare(middle(ink.mark), middle(ink.words), 1, "the tree against the `H`")
            verify(ink.mark.bottom > ink.words.bottom + 1, "its stem runs under the baseline")
        }

        function test_a_heading_s_mark_is_a_step_larger_on_the_same_middle() {
            const row = inks("lead", "stash", "H")
            const heading = inks("head", "stash", "H")
            const rowSpan = row.mark.bottom - row.mark.top
            const headingSpan = heading.mark.bottom - heading.mark.top
            fuzzyCompare(headingSpan / rowSpan, Metrics.markHead, 0.05, "a step larger")
            fuzzyCompare(middle(heading.mark), middle(heading.words), 1, "on the capitals' middle")
        }
    }
}
