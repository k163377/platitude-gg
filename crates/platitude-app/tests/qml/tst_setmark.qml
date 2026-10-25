import QtQuick
import QtTest
import platitude.ui
import "painted.js" as Painted

// Where a mark set beside words lands against the letters of their face (デザイン規約 §アイコンの席), read off the
// pixels of both: a mark at a row's head on the capitals' middle whatever its height, a sign on the face's own `+`'s
// middle and never under the baseline, the tree on the capitals' middle in every class, a heading's mark a step larger,
// and every mark a seat takes in the middle of it with the face's own room round a `+` either side — in front of a
// name, in the middle between the seat's head and the name's ink. The witness is the
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
    Component {
        id: namedOf
        // A mark seated in front of its name as a chip sets the two (`CodeChip`): the seat from the head, the name in
        // the code face straight after it.
        Item {
            id: named
            property alias mark: namedMark
            property alias name: nameWords
            x: 8
            y: 8
            width: namedMark.width + nameWords.width
            height: nameWords.height
            FontMetrics {
                id: nameFace
                font: nameWords.font
            }
            NavIcon {
                id: namedMark
                width: namedMark.wordCell
                height: nameWords.height
                set: "sign"
                size: root.px
                wordBase: nameWords.baselineOffset
                nameAir: WordFace.nameAir(nameFace, nameWords.text)
                tint: "black"
            }
            Text {
                id: nameWords
                x: namedMark.width
                color: "black"
                font.family: Theme.monoFamily
                font.pixelSize: root.px
            }
        }
    }

    // The parts that set a mark in front of a name, each with a name opening on a narrow letter.
    Component {
        id: chipOf
        CodeChip {
            x: 8
            y: 8
            word: "issue"
            mark: "branch"
            size: root.px
        }
    }
    Component {
        id: lineOf
        CardText {
            x: 8
            y: 8
            width: 380
            text: "Open issue"
            marks: [[5, "branch", Theme.accent, 5]]
            pixelSize: root.px
        }
    }
    Component {
        id: runOf
        NameRun {
            x: 8
            y: 8
            width: 380
            height: root.px * 2
            name: "issue"
            pixelSize: root.px
        }
    }
    Component {
        id: sentenceOf
        AppMenuItem {
            x: 8
            y: 8
            width: 380
            refSentence: "%1 here"
            refName: "issue"
            refTint: Theme.textPrimary
            refMark: "branch"
        }
    }
    // Names set in the window's face behind their marks: a graph's chip (a worktree's, behind the tree), and the boxes
    // that name a ref — the plain one and the one with a list.
    Component {
        id: refChipOf
        RefChip {
            x: 8
            y: 8
            maxWidth: 380
            records: [{ "kind": "worktree", "name": "issue", "isHead": false, "hasRemote": false, "hasPr": false,
                        "here": true, "held": false, "locked": false, "remote": "", "key": "worktree:issue" }]
        }
    }
    Component {
        id: slimOf
        SlimField {
            x: 8
            y: 8
            width: 300
            leadMark: "branch"
            text: "issue"
        }
    }
    Component {
        id: comboOf
        AppCombo {
            x: 8
            y: 8
            width: 300
            leadMark: "branch"
            wanted: "issue"
            editText: "issue"
        }
    }
    // A branch whose upstream is gone: the cloud in the row-end column's seat, the `!` on its shoulder.
    Component {
        id: goneOf
        GoneBadge {
            x: 8
            y: 8
            gone: true
            wordBase: Theme.iconSm
        }
    }
    FontMetrics {
        id: codeLarge
        font.family: Theme.monoFamily
        font.pixelSize: 1000
    }
    FontMetrics {
        id: wordLarge
        font.family: Theme.uiFamily
        font.pixelSize: 1000
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
            return inkBetween(picture, 0, width, height)
        }
        /// …and across the columns `from`–`to`.
        function inkBetween(picture, from, to, height) {
            const cover = new Array(to).fill(0)
            for (let x = from; x < to; x++) {
                for (let y = 0; y < height; y++) {
                    const c = picture.pixel(x, y)
                    cover[x] = Math.max(cover[x], 1 - (c.r + c.g + c.b) / 3)
                }
            }
            let first = -1
            let last = -1
            for (let x = from; x < to; x++) {
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
        /// both ways: the face stands them on whole pixels, mostly a pixel above its own measure at these sizes, so a
        /// mark drawn to the measure stands no taller than the letters (`WordFace.capitalsAt`); and what runs past them
        /// is under a pixel, which a rounded top lands all on one side (`NavIcon.shiftY`).
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
                   data.tag + ": past the `H` both ways, " + over.toFixed(2) + " over and " + under.toFixed(2)
                   + " under")
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

        /// A mark in front of its name stands in the middle between the seat's head and the name's ink, the first
        /// letter's own air counted on the name's side (デザイン規約 §アイコンの席「字の中の席」): measured to the
        /// name's advance, that air lands behind the mark alone. A narrow first letter sits in the middle of its cell
        /// in the code face, which is the most air there is; a name in the window's face reads its own face's air.
        function test_a_mark_stands_between_its_seat_s_head_and_its_name_s_ink_data() {
            return [{ tag: "branch main", kind: "branch", name: "main" },
                    { tag: "cloud origin", kind: "remote", name: "origin" },
                    { tag: "tag v1.0", kind: "tag", name: "v1.0" },
                    { tag: "tree topic", kind: "tree", name: "topic" },
                    { tag: "branch issue", kind: "branch", name: "issue/12" },
                    { tag: "tree issue in the window's face", kind: "tree", name: "issue", face: Theme.uiFamily },
                    { tag: "padlock lab in the window's face", kind: "lock", name: "lab", face: Theme.uiFamily }]
        }
        function test_a_mark_stands_between_its_seat_s_head_and_its_name_s_ink(data) {
            const named = createTemporaryObject(namedOf, root)
            named.mark.kind = data.kind
            if (data.face !== undefined)
                named.name.font.family = data.face
            named.name.text = data.name
            tryVerify(() => named.mark.inked)
            const picture = grabImage(named)
            const height = Math.round(named.height)
            const mark = inkBetween(picture, 0, Math.round(named.mark.width), height)
            const name = inkBetween(picture, Math.round(named.mark.width), Math.ceil(named.width), height)
            named.visible = false
            // A pixel: the mark stands on a whole one (`NavIcon.shiftX`), and a round edge — the cloud's arcs, the
            // diamond's tip, an `o` — reads a little inside its outline.
            fuzzyCompare(mark.left, name.left - mark.right, 1.25,
                         data.tag + ": " + mark.left + " ahead of the mark, " + (name.left - mark.right) + " behind it")
        }

        /// The first `kind` mark set inside `item`.
        function markIn(item, kind) {
            return Painted.itemUnder(item, child => child.nameAir !== undefined && child.visible && child.kind === kind)
        }
        /// …and every part that sets a mark in front of a name hands the mark that name's air, read in the face the
        /// name is drawn in, at the size it is drawn at: a chip, a line of words, a run, a menu row's sentence.
        function test_a_part_hands_its_mark_the_name_s_air_data() {
            return [{ tag: "chip", of: chipOf, kind: "branch" }, { tag: "line of words", of: lineOf, kind: "branch" },
                    { tag: "run", of: runOf, kind: "tree" },
                    { tag: "menu row's sentence", of: sentenceOf, kind: "branch" }]
        }
        function test_a_part_hands_its_mark_the_name_s_air(data) {
            const part = createTemporaryObject(data.of, root)
            tryVerify(() => markIn(part, data.kind) !== null && markIn(part, data.kind).inked)
            const mark = markIn(part, data.kind)
            const air = codeLarge.tightBoundingRect("i").x * mark.size / 1000
            verify(air > 1, "`i` holds air ahead of its ink in the code face: " + air)
            fuzzyCompare(mark.nameAir, air, 0.01, data.tag + ": the name's air")
            part.visible = false
        }
        /// …and so does every part whose name is set in the window's face: a graph's chip, the boxes that name a ref.
        function test_a_part_in_the_window_s_face_hands_its_mark_the_name_s_air_data() {
            return [{ tag: "graph chip", of: refChipOf, kind: "tree" },
                    { tag: "plain box", of: slimOf, kind: "branch" },
                    { tag: "box with a list", of: comboOf, kind: "branch" }]
        }
        function test_a_part_in_the_window_s_face_hands_its_mark_the_name_s_air(data) {
            const part = createTemporaryObject(data.of, root)
            tryVerify(() => markIn(part, data.kind) !== null && markIn(part, data.kind).inked)
            const mark = markIn(part, data.kind)
            const air = wordLarge.tightBoundingRect("i").x * mark.size / 1000
            verify(air > 0.25, "`i` holds air ahead of its ink in the window's face: " + air)
            fuzzyCompare(mark.nameAir, air, 0.01, data.tag + ": the name's air")
            part.visible = false
        }

        /// A badge after a name takes the seat of the name's letters, the row-end column's, and a gone upstream's `!`
        /// stands on the cloud's shoulder against its ink — the seat holds air past the cloud, which the seat's edge
        /// would stand the `!` off (デザイン規約 §アイコンの席 の右肩). The inks are the marks' own, which `tst_iconink`
        /// holds to the pixels drawn.
        function test_a_gone_badge_s_bang_stands_against_the_cloud_s_ink() {
            const badge = createTemporaryObject(goneOf, root)
            compare(badge.implicitWidth, MarkSeat.cell(badge.size), "the seat of the name's letters")
            const marks = badge.children.filter(child => child.inkRight !== undefined)
            const cloud = marks.find(mark => mark.kind === "remote")
            const bang = marks.find(mark => mark.kind === "bang")
            verify(cloud.width - cloud.inkRight > 1, "the seat holds air past the cloud")
            fuzzyCompare(bang.x + bang.inkLeft, cloud.x + cloud.inkRight, 0.01, "the `!`'s ink against the cloud's")
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
        /// whole pixels (offscreen's FreeType), a space rounded narrower or wider can take one more or one fewer.
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
