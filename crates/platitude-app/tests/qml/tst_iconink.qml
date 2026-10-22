import QtQuick
import QtTest
import platitude.ui

// Every mark's ink stands in the middle of its own box, at the sizes and line weights the window draws marks at
// (デザイン規約 §アイコンの席): a box centred on a line is then a mark on that line's middle wherever the window runs,
// with nothing for a caller to add. And the ink a mark reports (`inkWidth` / `inkHeight` / `inkRight` / `inkTop`) is
// the ink it draws — what a caller seats it by. Both are read off the pixels.
Item {
    id: root
    // Room for every kind at once, each in a cell of its own a gap apart, so that one picture of the root reads
    // them all: `grabImage` renders the window each time it is asked, and a grab per mark is a window per mark.
    width: 10 * (Theme.iconMd * 4 + gap) + gap
    height: 4 * (Theme.iconMd * 4 + gap) + gap

    /// Between two cells, and from the root's corner: no mark's ink reaches another's box, and none sits at the
    /// corner `grabImage` cuts at.
    readonly property int gap: 8

    Component {
        id: markOf
        NavIcon {
            tint: "black"
        }
    }

    TestCase {
        name: "IconInk"
        when: windowShown

        /// Every kind the window draws but the resize corner, which hangs off its box's corner on purpose.
        readonly property var kinds: ["branch", "remote", "tag", "stash", "tree", "pen", "plus", "minus", "arrow",
            "fetch", "push", "spinner", "history", "copyicon", "bang", "hier", "list", "pr", "check", "lock", "home",
            "folder", "chevron", "chevrons", "terminal", "menu", "close", "gear", "window-minimize", "window-maximize",
            "window-restore", "app-window", "unified", "split", "eye", "eye-off", "no", "no-entry", "search", "github"]
        /// Four times the grid, so a pixel's antialiasing is a quarter of one at the grid's own size: at the window's
        /// sizes a thin end — a round cap, a point — leaves less than a pixel of ink at the edge, and the reading leans.
        readonly property int large: Theme.iconMd * 4
        /// The line's share of the box, at each weight the window draws: the grid's own (`iconMd`), the small mark kept
        /// at the grid's line (`iconSm`) and the large one (`iconXl`). A half line reaches a different share of the
        /// grid at each, which is what `NavIcon.reaches` counts.
        readonly property var shares: [Metrics.iconStroke / Theme.iconMd, Metrics.iconStroke / Theme.iconSm,
            Metrics.iconStroke / Theme.iconXl]

        /// Where the ink starts and ends along one axis, to a fraction of a pixel: the first and last line of pixels
        /// holding any, eased by how much of that line the ink covers at most.
        function edges(cover) {
            let first = -1
            let last = -1
            for (let i = 0; i < cover.length; i++) {
                if (cover[i] > 0.02) {
                    if (first < 0)
                        first = i
                    last = i
                }
            }
            return [first + 1 - cover[first], last + cover[last]]
        }
        /// The ink of a drawn mark, read off `picture` (one of the root) inside the mark's own box:
        /// `[left, top, right, bottom]` in its own pixels.
        function inkOf(picture, mark) {
            const across = new Array(mark.width).fill(0)
            const down = new Array(mark.height).fill(0)
            for (let x = 0; x < mark.width; x++) {
                for (let y = 0; y < mark.height; y++) {
                    const c = picture.pixel(mark.x + x, mark.y + y)
                    const cover = 1 - (c.r + c.g + c.b) / 3
                    across[x] = Math.max(across[x], cover)
                    down[y] = Math.max(down[y], cover)
                }
            }
            const h = edges(across)
            const v = edges(down)
            return [h[0], v[0], h[1], v[1]]
        }
        /// Every kind of `which` painted at once at `size` with a line of `stroke`, each in its own cell, and the
        /// one picture of the root they all stand in.
        function painted(which, size, stroke) {
            const cell = size + root.gap
            const perRow = Math.floor((root.width - root.gap) / cell)
            const marks = which.map((kind, i) => createTemporaryObject(markOf, root, {
                kind: kind, width: size, height: size, stroke: stroke,
                x: root.gap + (i % perRow) * cell, y: root.gap + Math.floor(i / perRow) * cell
            }))
            for (const mark of marks)
                tryVerify(() => mark.inked)
            return { marks: marks, picture: grabImage(root) }
        }
        function cleared(marks) {
            for (const mark of marks)
                mark.destroy()
        }

        function test_the_ink_is_what_the_mark_reports_and_stands_in_its_middle_data() {
            return shares.map(share => ({ tag: "a line of " + share.toFixed(3) + " of the box", share: share }))
        }
        function test_the_ink_is_what_the_mark_reports_and_stands_in_its_middle(data) {
            const drawn = painted(kinds, large, large * data.share)
            for (const mark of drawn.marks) {
                const at = mark.kind + " with a line of " + data.share.toFixed(3) + " of the box"
                const ink = inkOf(drawn.picture, mark)
                // A pixel at this size: a quarter of one at the grid's.
                fuzzyCompare((ink[0] + ink[2]) / 2, large / 2, 1, at + ": in the middle across")
                fuzzyCompare((ink[1] + ink[3]) / 2, large / 2, 1, at + ": in the middle down")
                fuzzyCompare(mark.inkRight - mark.inkWidth, ink[0], 1, at + ": reports its left edge")
                fuzzyCompare(mark.inkRight, ink[2], 1, at + ": reports its right edge")
                fuzzyCompare(mark.inkTop, ink[1], 1, at + ": reports its top edge")
                fuzzyCompare(mark.inkTop + mark.inkHeight, ink[3], 1, at + ": reports its bottom edge")
            }
            cleared(drawn.marks)
        }

        /// The resize corner hangs off its box's corner on purpose, so only what it reports is held: a caller seats it
        /// by its ink (`DescriptionBox`).
        function test_the_resize_corner_reports_its_ink() {
            for (const share of shares) {
                const at = "grip with a line of " + share.toFixed(3) + " of the box"
                const drawn = painted(["grip"], large, large * share)
                const mark = drawn.marks[0]
                const ink = inkOf(drawn.picture, mark)
                fuzzyCompare(mark.inkRight - mark.inkWidth, ink[0], 1, at + ": reports its left edge")
                fuzzyCompare(mark.inkRight, ink[2], 1, at + ": reports its right edge")
                fuzzyCompare(mark.inkTop, ink[1], 1, at + ": reports its top edge")
                fuzzyCompare(mark.inkTop + mark.inkHeight, ink[3], 1, at + ": reports its bottom edge")
                cleared(drawn.marks)
            }
        }

        function test_at_the_window_s_sizes_too_data() {
            return [Theme.iconMd, Theme.iconLg, Theme.iconXl].map(size => ({ tag: "at " + size, size: size }))
        }
        /// The sizes a mark stands alone at, where a pixel off the middle is seen; the reading leans a third of one.
        function test_at_the_window_s_sizes_too(data) {
            const drawn = painted(kinds, data.size, Metrics.iconStroke)
            for (const mark of drawn.marks) {
                const ink = inkOf(drawn.picture, mark)
                fuzzyCompare((ink[0] + ink[2]) / 2, data.size / 2, 0.35,
                             mark.kind + " at " + data.size + ": in the middle across")
                fuzzyCompare((ink[1] + ink[3]) / 2, data.size / 2, 0.35,
                             mark.kind + " at " + data.size + ": in the middle down")
            }
            cleared(drawn.marks)
        }
    }
}
