import QtQuick
import QtTest
import platitude.ui

// Where a mark set beside words lands against the letters of their face (デザイン規約 §アイコンの席), read off the
// pixels of both: a mark at a row's head on the capitals' middle whatever its height, a sign on the face's own `+`'s
// middle and never under the baseline, the tree on the capitals' middle in every class, a heading's mark a step larger,
// and every mark a seat takes in the middle of it with the face's own room round a `+` either side. The witness is the
// face's own `H` and `+` drawn beside the mark, and the mark's own pixels, not the numbers the mark is placed by.
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
            property real size: root.px
            property int weight: Font.Normal
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
                font.pixelSize: pair.size
                font.weight: pair.weight
            }
            // The mirror box: the words' line, their baseline as its own.
            NavIcon {
                id: mark
                x: words.width + Theme.spaceXs
                width: pair.size * 2
                height: words.height
                wordBase: words.baselineOffset
                size: pair.size
                tint: "black"
            }
        }
    }
    Component {
        id: seatOf
        // A mark alone in its seat, its box a line of words at `px`.
        NavIcon {
            id: seated
            x: 8
            y: 8
            width: seated.wordCell
            height: root.px * 2
            wordBase: root.px * 3 / 2
            size: root.px
            tint: "black"
        }
    }

    TestCase {
        name: "SetMark"
        when: windowShown

        // Every case sets its mark after it stands (`inks`), as a captioned toggle does, and the mark says nothing on
        // the way — held on every case, as only the first mark a file builds says anything (`NavIcon.target`).
        function init() {
            failOnWarning(/NavIcon\.qml/)
        }

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
        /// …and across the columns `0`–`width`.
        function inkAcross(picture, width, height) {
            const cover = new Array(width).fill(0)
            for (let x = 0; x < width; x++) {
                for (let y = 0; y < height; y++) {
                    const c = picture.pixel(x, y)
                    cover[x] = Math.max(cover[x], 1 - (c.r + c.g + c.b) / 3)
                }
            }
            let first = -1
            let last = -1
            for (let x = 0; x < width; x++) {
                if (cover[x] > 0.02) {
                    if (first < 0)
                        first = x
                    last = x
                }
            }
            return { left: first + 1 - cover[first], right: last + cover[last] }
        }
        /// The letter's ink and the mark's, down the pair's own pixels: at `root.px` unless a `size` and `weight` are
        /// given.
        function inks(set, kind, letter, turn, size, weight) {
            const pair = createTemporaryObject(pairOf, root, { letter: letter, size: size || root.px,
                                                               weight: weight === undefined ? Font.Normal : weight })
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
            const ink = inks("sign", "pr", "H")
            fuzzyCompare(ink.mark.bottom, ink.words.bottom, 1, "the foot on the `H`'s foot")
        }

        /// The tree and the branch, drawn as tall as each other, stand on the capitals' middle as signs too
        /// (`NavIcon.lettered`).
        function test_the_lettered_signs_stand_on_the_capitals_middle_data() {
            return [{ tag: "tree", kind: "tree" }, { tag: "branch", kind: "branch" }]
        }
        function test_the_lettered_signs_stand_on_the_capitals_middle(data) {
            const ink = inks("sign", data.kind, "H")
            fuzzyCompare(middle(ink.mark), middle(ink.words), 1, data.kind + " against the `H`")
            verify(ink.mark.bottom > ink.words.bottom + 1, data.kind + "'s stem runs under the baseline")
        }

        /// At every size a name is set in, in either weight, a lettered mark stands past the capitals drawn beside it,
        /// both ways: the face stands them on whole pixels, mostly a pixel above its own measure at these sizes, and a
        /// mark drawn to the measure stood no taller than the letters (`WordFace.capitalsAt`); and what runs past them,
        /// under a pixel, all landed on one side while the mark's top was rounded (`NavIcon.shiftY`).
        function test_a_lettered_mark_stands_past_the_capitals_at_a_names_size_data() {
            const rows = []
            for (const size of [Theme.fontSm, Theme.fontChip, Theme.fontMd, Theme.fontLg, Theme.fontXl]) {
                for (const weight of [Font.Normal, Theme.fontWeightStrong])
                    rows.push({ tag: size + (weight === Font.Normal ? "" : " strong"), size: size, weight: weight })
            }
            return rows
        }
        function test_a_lettered_mark_stands_past_the_capitals_at_a_names_size(data) {
            const ink = inks("name", "branch", "H", 0, data.size, data.weight)
            const past = (ink.mark.bottom - ink.mark.top) - (ink.words.bottom - ink.words.top)
            verify(past > 0.5, data.tag + ": the branch stands " + past.toFixed(2) + "px past the `H`")
            const over = ink.words.top - ink.mark.top
            const under = ink.mark.bottom - ink.words.bottom
            verify(over > 0.15 && under > 0.15,
                   data.tag + ": past the `H` both ways, " + over.toFixed(2) + " over and " + under.toFixed(2) + " under")
        }

        function test_a_heading_s_mark_is_a_step_larger_on_the_same_middle() {
            const row = inks("lead", "stash", "H")
            const heading = inks("head", "stash", "H")
            const rowSpan = row.mark.bottom - row.mark.top
            const headingSpan = heading.mark.bottom - heading.mark.top
            fuzzyCompare(headingSpan / rowSpan, Metrics.markHead, 0.05, "a step larger")
            fuzzyCompare(middle(heading.mark), middle(heading.words), 1, "on the capitals' middle")
        }

        /// Every mark a seat takes — a ref's kind in front of its name, a picture on a word, a letter of the words —
        /// stands in one width whatever its kind (デザイン規約 §アイコンの席「字の中の席」), in the middle of it with the
        /// seat's air either side at least (`MarkSeat.air`): the seat holds the widest of them.
        function test_a_seat_holds_every_seated_mark_in_its_middle_data() {
            return [{ tag: "branch", set: "sign", kind: "branch" }, { tag: "cloud", set: "sign", kind: "remote" },
                    { tag: "tag", set: "sign", kind: "tag" }, { tag: "tree", set: "sign", kind: "tree" },
                    { tag: "house", set: "sign", kind: "home" }, { tag: "padlock", set: "sign", kind: "lock" },
                    { tag: "a tally's mark", set: "sign", kind: "pen" },
                    { tag: "no newline at the end", set: "sign", kind: "no-entry" },
                    { tag: "cloud after a name", set: "name", kind: "remote" },
                    { tag: "pull request after a name", set: "name", kind: "pr" },
                    { tag: "push", set: "name", kind: "push" }]
        }
        function test_a_seat_holds_every_seated_mark_in_its_middle(data) {
            const mark = createTemporaryObject(seatOf, root, { set: data.set, kind: data.kind })
            verify(mark.seated, data.kind + " takes the seat")
            compare(mark.width, MarkSeat.cell(root.px), data.kind + ": one width whatever the kind")
            tryVerify(() => mark.inked)
            const ink = inkAcross(grabImage(mark), Math.round(mark.width), Math.round(mark.height))
            const after = mark.width - ink.right
            const air = MarkSeat.air * root.px
            verify(ink.left >= air - 1 && after >= air - 1,
                   data.tag + ": the seat's air either side of " + air + ": " + ink.left + ", " + after)
            // A pixel: the seat stands the ink on a whole one (`NavIcon.shiftX`), and the drawn edges read a hair more.
            fuzzyCompare(ink.left, after, 1.1, data.tag + " in the middle")
        }

        /// The widest mark stays inside its seat at every size words are set in, a seat laid in a line's spaces as well
        /// as one counted off the face (`MarkSeat.room` / `cell`), with the seat's air either side — a stem where the
        /// face sets its `+` tighter: a `Canvas` cuts what it draws past its edge, which a grabbed picture of the mark
        /// alone does not show.
        function test_the_widest_mark_keeps_inside_its_seat_at_every_size_data() {
            const rows = []
            for (const size of [Theme.fontSm, Theme.fontMd, Theme.fontLg, Theme.fontXl]) {
                for (const laid of [false, true])
                    rows.push({ tag: size + (laid ? " in a line" : ""), size: size, laid: laid })
            }
            return rows
        }
        function test_the_widest_mark_keeps_inside_its_seat_at_every_size(data) {
            const space = MarkSeat.space.advanceWidth * data.size / 1000
            const width = data.laid ? MarkSeat.room(space, data.size).length * space : MarkSeat.cell(data.size)
            const mark = createTemporaryObject(seatOf, root, { set: "name", kind: "remote", size: data.size,
                                                              width: width })
            const air = MarkSeat.air * data.size
            const after = mark.width - mark.inkRight
            verify(mark.inkLeft >= air - 1 && after >= air - 1,
                   data.tag + ": the seat's air either side of " + air + ": " + mark.inkLeft + ", " + after)
            fuzzyCompare(mark.inkLeft, after, 1, data.tag + " in the middle")
        }

        /// A sign drawn in place of a character — an arrow — takes no seat: it is set as that character, in the face's
        /// own room round its ink and a stem more (デザイン規約 §アイコンの席 の字の代替).
        function test_a_characters_stand_in_takes_no_seat() {
            const mark = createTemporaryObject(seatOf, root, { set: "sign", kind: "arrow", characterStandIn: true })
            verify(!mark.seated, "no seat")
            compare(mark.wordCell, Math.round(mark.inkAcross + 2 * mark.wordAir), "the character's own room")
            verify(mark.wordCell < MarkSeat.cell(root.px), "narrower than a seat")
        }

        /// A line opens the fewest of its own spaces that hold the widest mark: where a face sets its advances to
        /// whole pixels (offscreen's FreeType), a space rounded narrower can take one more at a size.
        function test_a_rooms_spaces_are_the_fewest_of_the_lines_own_that_hold_the_seat() {
            const size = Theme.fontMd
            const own = MarkSeat.space.advanceWidth * size / 1000
            for (const space of [own, Math.floor(own), Math.ceil(own)]) {
                const spaces = MarkSeat.room(space, size).length
                verify(spaces * space >= MarkSeat.need(size), space + ": holds the seat")
                verify((spaces - 1) * space < MarkSeat.need(size), space + ": the fewest that do")
            }
            compare(MarkSeat.cell(size), Math.round(MarkSeat.room(own, size).length * own), "the face's own, unlaid")
        }
    }
}
