import QtQuick
import QtTest
import platitude.ui

// A name cut in the middle keeps its mark clear of both halves (規約 §寸法「列に並ぶ名前の省略は、余りを切れ目へ入れる」):
// head, mark and tail together within the box at every width. The elide the cut starts from can answer a few pixels
// past the box in a small face — the WORKTREES row's branch column (`NavRowBody.branchSeat`), at `fontSm` — and the
// tail, drawn against the box's right edge, then lands on the mark. The widths are where that happens, so every one of
// them is walked.
Item {
    id: root
    width: 800
    height: 40

    CutName {
        id: side
        height: 20
        text: "review-the-long-worktree-folder-names-in-the-repository-card-and-every-other-menu-too"
        pixelSize: Theme.fontSm
    }
    // A line that stands for more lines, in a box it fits.
    CutName {
        id: firstLine
        y: 20
        width: 400
        height: 20
        closed: true
        cutAt: "end"
        more: true
        text: "fatal: the first of more lines"
        pixelSize: Theme.fontSm
    }
    // Words cut at their end, in no column: a heading of the right pane.
    CutName {
        id: words
        y: 20
        height: 20
        closed: true
        cutAt: "end"
        text: "UNSTAGED FILES AND MORE"
        pixelSize: Theme.fontMd
    }
    // The same words cut at their start and in their middle: the tail keeps no space after the mark either.
    CutName {
        id: wordsFromStart
        y: 40
        height: 20
        closed: true
        cutAt: "start"
        text: words.text
        pixelSize: Theme.fontMd
    }
    CutName {
        id: wordsInMiddle
        y: 60
        height: 20
        closed: true
        cutAt: "middle"
        text: words.text
        pixelSize: Theme.fontMd
    }
    // The same name as a line in no column.
    CutName {
        id: standing
        y: 20
        height: 20
        closed: true
        text: side.text
        pixelSize: Theme.fontSm
    }

    TestCase {
        name: "CutName"
        when: windowShown

        function test_the_mark_stands_clear_of_both_halves() {
            // The drawn mark (`CutMark`), its advance its `implicitWidth`.
            let mark = null
            for (const child of side.children)
                if (child instanceof CutMark)
                    mark = child
            verify(mark !== null, "the cut draws the cut mark")
            const whole = side.implicitWidth
            for (let box = 60; box < whole; box++) {
                side.width = box
                verify(side.cutting, "cut at " + box)
                const ink = side.inkOf(side.headText) + mark.implicitWidth + side.inkOf(side.tailText)
                verify(ink <= box, "within the box at " + box + ": " + ink)
                // The mark stands at the head's whole-pixel end at the earliest (`implicitWidth` is rounded up): a
                // pixel's give.
                verify(mark.x + mark.implicitWidth <= box - side.inkOf(side.tailText) + 1,
                       "the tail clear of the mark at " + box)
            }
        }

        /// A line in no column (`closed`) is drawn as `Text.elide` draws its cut: the mark straight after the head and
        /// the tail straight after the mark, no air between them that a typed `…` would not leave — or, cut at its
        /// start, the mark straight before the tail against the right edge.
        function test_a_closed_cut_draws_its_pieces_closed_up() {
            for (const at of ["end", "middle", "start"]) {
                standing.cutAt = at
                for (let box = 60; box < standing.implicitWidth; box += 7) {
                    standing.width = box
                    verify(standing.cutting, at + " cut at " + box)
                    let mark = null, head = null, tail = null
                    for (const child of standing.children) {
                        if (child instanceof CutMark)
                            mark = child
                        else if (head === null && child.text === standing.headText)
                            head = child
                        else if (child.text === standing.tailText)
                            tail = child
                    }
                    if (at === "start") {
                        fuzzyCompare(mark.x + mark.implicitWidth + tail.implicitWidth, box, 0.01,
                                     "the mark straight before the tail, against the edge, at " + box)
                    } else {
                        compare(mark.x, head.implicitWidth, at + ": the mark straight after the head at " + box)
                    }
                    if (at === "middle")
                        compare(tail.x, mark.x + mark.implicitWidth, "the tail straight after the mark at " + box)
                    // The pieces as laid out fit the box they were cut for.
                    const drawn = head.implicitWidth + mark.implicitWidth + (tail === null ? 0 : tail.implicitWidth)
                    verify(drawn <= box + 1, at + ": drawn " + drawn + " in a box of " + box)
                }
            }
        }

        /// A line that stands for more lines past it (`more`) ends on the mark where it fits with it, whole.
        function test_a_line_with_more_after_it_ends_on_the_mark() {
            verify(firstLine.cutting, "the mark is drawn")
            compare(firstLine.headText, firstLine.text, "the line whole")
            compare(firstLine.tailText, "")
        }

        // A cut that lands on a word's edge keeps no space before its mark: at every width the head ends on a letter.
        function test_no_space_stands_before_the_mark() {
            let landed = false
            for (let box = 30; box < words.implicitWidth; box++) {
                words.width = box
                verify(words.cutting, "cut at " + box)
                verify(!/\s$/.test(words.headText), "at " + box + " the head ends on a space: '" + words.headText + "'")
                landed = landed || words.text.charAt(words.headText.length) === " "
            }
            verify(landed, "some width cut on a word's edge")
        }

        // The same for the tail of a cut at the start or in the middle: at every width it starts on a letter.
        function test_no_space_stands_after_the_mark_data() {
            return [{ "tag": "start", "item": wordsFromStart }, { "tag": "middle", "item": wordsInMiddle }]
        }
        function test_no_space_stands_after_the_mark(data) {
            const item = data.item
            let landed = false
            for (let box = 30; box < item.implicitWidth; box++) {
                item.width = box
                verify(item.cutting, data.tag + " cut at " + box)
                verify(!/^\s/.test(item.tailText),
                       data.tag + " at " + box + ": the tail starts on a space: '" + item.tailText + "'")
                landed = landed || item.text.charAt(item.text.length - item.tailText.length - 1) === " "
            }
            verify(landed, data.tag + ": some width cut on a word's edge")
        }
    }
}
