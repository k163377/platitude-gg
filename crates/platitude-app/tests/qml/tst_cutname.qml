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

    TestCase {
        name: "CutName"
        when: windowShown

        function test_the_mark_stands_clear_of_both_halves() {
            const labels = []
            for (const child of side.children)
                if (child.contentWidth !== undefined)
                    labels.push(child)
            const mark = labels.find(label => label.text === "…")
            const whole = side.implicitWidth
            for (let box = 60; box < whole; box++) {
                side.width = box
                verify(side.cutting, "cut at " + box)
                const ink = side.inkOf(side.headText) + side.inkOf("…") + side.inkOf(side.tailText)
                verify(ink <= box, "within the box at " + box + ": " + ink)
                // The mark stands at the head's whole-pixel end at the earliest (`implicitWidth` is rounded up): a
                // pixel's give.
                verify(mark.x + side.inkOf("…") <= box - side.inkOf(side.tailText) + 1,
                       "the tail clear of the mark at " + box)
            }
        }
    }
}
