import QtQuick
import QtTest
import platitude.ui

// Every mark's ink stands in the middle of its own box, at the sizes and line weights the window draws marks at
// (デザイン規約 §アイコンの席): a box centred on a line is then a mark on that line's middle wherever the window runs,
// with nothing for a caller to add. And the ink a mark reports (`inkWidth` / `inkHeight` / `inkRight` / `inkTop`) is
// the ink it draws — what a caller seats it by. Both are read off the pixels.
Item {
    id: root
    width: 96
    height: 96

    Component {
        id: markOf
        // Straight under the root, away from its corner: `grabImage` cuts the window at the item's own x / y.
        NavIcon {
            x: 8
            y: 8
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
            "window-restore", "app-window", "unified", "split", "eye", "eye-off", "no", "no-entry", "search"]
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
        /// The ink of a drawn mark: `[left, top, right, bottom]` in its own pixels.
        function inkOf(mark) {
            const picture = grabImage(mark)
            const across = new Array(mark.width).fill(0)
            const down = new Array(mark.height).fill(0)
            for (let x = 0; x < mark.width; x++) {
                for (let y = 0; y < mark.height; y++) {
                    const c = picture.pixel(x, y)
                    const cover = 1 - (c.r + c.g + c.b) / 3
                    across[x] = Math.max(across[x], cover)
                    down[y] = Math.max(down[y], cover)
                }
            }
            const h = edges(across)
            const v = edges(down)
            return [h[0], v[0], h[1], v[1]]
        }
        function painted(kind, size, stroke) {
            const mark = createTemporaryObject(markOf, root, { kind: kind, width: size, height: size, stroke: stroke })
            tryVerify(() => mark.inked)
            return mark
        }

        function test_the_ink_is_what_the_mark_reports_and_stands_in_its_middle_data() {
            return kinds.map(kind => ({ tag: kind, kind: kind }))
        }
        function test_the_ink_is_what_the_mark_reports_and_stands_in_its_middle(data) {
            for (const share of shares) {
                const at = data.kind + " with a line of " + share.toFixed(3) + " of the box"
                const mark = painted(data.kind, large, large * share)
                const ink = inkOf(mark)
                // A pixel at this size: a quarter of one at the grid's.
                fuzzyCompare((ink[0] + ink[2]) / 2, large / 2, 1, at + ": in the middle across")
                fuzzyCompare((ink[1] + ink[3]) / 2, large / 2, 1, at + ": in the middle down")
                fuzzyCompare(mark.inkRight - mark.inkWidth, ink[0], 1, at + ": reports its left edge")
                fuzzyCompare(mark.inkRight, ink[2], 1, at + ": reports its right edge")
                fuzzyCompare(mark.inkTop, ink[1], 1, at + ": reports its top edge")
                fuzzyCompare(mark.inkTop + mark.inkHeight, ink[3], 1, at + ": reports its bottom edge")
                mark.destroy()
            }
        }

        /// The resize corner hangs off its box's corner on purpose, so only what it reports is held: a caller seats it
        /// by its ink (`DescriptionBox`).
        function test_the_resize_corner_reports_its_ink() {
            for (const share of shares) {
                const at = "grip with a line of " + share.toFixed(3) + " of the box"
                const mark = painted("grip", large, large * share)
                const ink = inkOf(mark)
                fuzzyCompare(mark.inkRight - mark.inkWidth, ink[0], 1, at + ": reports its left edge")
                fuzzyCompare(mark.inkRight, ink[2], 1, at + ": reports its right edge")
                fuzzyCompare(mark.inkTop, ink[1], 1, at + ": reports its top edge")
                fuzzyCompare(mark.inkTop + mark.inkHeight, ink[3], 1, at + ": reports its bottom edge")
                mark.destroy()
            }
        }

        function test_at_the_window_s_sizes_too_data() {
            return kinds.map(kind => ({ tag: kind, kind: kind }))
        }
        /// The sizes a mark stands alone at, where a pixel off the middle is seen; the reading leans a third of one.
        function test_at_the_window_s_sizes_too(data) {
            for (const size of [Theme.iconMd, Theme.iconLg, Theme.iconXl]) {
                const mark = painted(data.kind, size, Metrics.iconStroke)
                const ink = inkOf(mark)
                fuzzyCompare((ink[0] + ink[2]) / 2, size / 2, 0.35, data.kind + " at " + size + ": in the middle across")
                fuzzyCompare((ink[1] + ink[3]) / 2, size / 2, 0.35, data.kind + " at " + size + ": in the middle down")
                mark.destroy()
            }
        }
    }
}
