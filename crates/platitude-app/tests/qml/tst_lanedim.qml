import QtQuick
import QtTest
import platitude.ui

// What a row the search passed over dims (デザイン規約 §コミットを探す), read off the canvas: its lanes and node go down to
// `dimFade` (the node without the lanes under it showing through), a matched row's are at full strength — on every
// paint. The 2D context outlives a paint and the list hands a cell from row to row, so only the canvas's own pixels
// answer: a screenshot is one paint, and the leak is between two.
Item {
    id: root
    width: 120
    height: 60

    /// Lane 1 runs straight through the row; the node sits on lane 0, clear of it.
    readonly property var through: [{ "kind": "through", "lane": 1, "color": 0, "dashed": false }]
    /// A pixel the lane covers whole (a `laneStroke` line on a whole-pixel centre), well above the node.
    readonly property int laneX: Metrics.laneInset + Metrics.laneW + Metrics.laneW / 2 - 1
    readonly property int laneY: 2
    /// The node's centre: inside the face, whatever the pattern draws there.
    readonly property int nodeX: Metrics.laneInset + Metrics.laneW / 2
    readonly property int nodeY: Theme.graphRowHeight / 2
    /// The node's own lane, into it from above and out of it below — a line straight through the face.
    readonly property var ownLane: [{ "kind": "into", "lane": 0, "color": 0, "dashed": false },
                                    { "kind": "out", "lane": 0, "color": 0, "dashed": false }]
    /// A column that line covers whole.
    readonly property int ownLaneX: root.nodeX - 1

    Component {
        id: fixture
        GraphLaneCell {
            width: 2 * Metrics.laneInset + 3 * Metrics.laneW
            height: Theme.graphRowHeight
            xOffset: 0
            fullWidth: width
            geometry: root.through
            nodeLane: 0
            coAuthors: []
            avatar: 0x5a5a
            avatarUrl: ""
            isWip: false
            stashRef: ""
            dimmed: true
        }
    }

    Component {
        id: paints
        SignalSpy {
            signalName: "painted"
        }
    }

    /// The cell's canvas — the one `InkCanvas` under it.
    function inkOf(item) {
        for (let i = 0; i < item.children.length; i++) {
            const child = item.children[i]
            if (child.inked !== undefined)
                return child
            const found = root.inkOf(child)
            if (found !== null)
                return found
        }
        return null
    }

    /// The alpha the canvas itself holds at one pixel — its own buffer, not the window's composite of it.
    function alphaAt(ink, x, y) {
        return ink.getContext("2d").getImageData(x, y, 1, 1).data[3]
    }

    TestCase {
        name: "LaneDim"
        when: windowShown

        function cellWithSpy(props) {
            const cell = createTemporaryObject(fixture, root, props === undefined ? {} : props)
            verify(cell !== null)
            const ink = root.inkOf(cell)
            verify(ink !== null, "the cell draws on an InkCanvas")
            const spy = createTemporaryObject(paints, root, { "target": ink })
            verify(spy !== null)
            tryCompare(spy, "count", 1, undefined, "the first paint")
            return { "cell": cell, "ink": ink, "spy": spy }
        }

        function dimmedAt(ink, x, y, what) {
            const a = root.alphaAt(ink, x, y)
            verify(Math.abs(a - 255 * Metrics.dimFade) <= 2, what + " at dimFade, alpha " + a)
        }

        function test_a_dimmed_row_dims_its_lanes_on_every_paint() {
            const t = cellWithSpy()
            dimmedAt(t.ink, root.laneX, root.laneY, "the lanes on the first paint")
            dimmedAt(t.ink, root.nodeX, root.nodeY, "the node on the first paint")
            t.ink.requestPaint()
            tryCompare(t.spy, "count", 2, undefined, "the second paint")
            dimmedAt(t.ink, root.laneX, root.laneY, "the lanes on the second paint")
            dimmedAt(t.ink, root.nodeX, root.nodeY, "the node on the second paint")
        }

        /// The list's reuse as it scrolls, and the whole graph as the search closes.
        function test_a_row_lit_after_a_dimmed_one_is_lit_whole() {
            const t = cellWithSpy()
            t.cell.dimmed = false
            tryCompare(t.spy, "count", 2, undefined, "the paint the change asks for")
            compare(root.alphaAt(t.ink, root.laneX, root.laneY), 255, "the lanes")
            compare(root.alphaAt(t.ink, root.nodeX, root.nodeY), 255, "the node")
        }

        /// A dimmed face is darker, not see-through: over its own lane it reads pixel for pixel as over nothing (read
        /// down the lane's column, inside the outline).
        function test_a_dimmed_face_does_not_show_its_lane_through() {
            const over = cellWithSpy({ "geometry": root.ownLane })
            const bare = cellWithSpy({ "geometry": [] })
            const r = Metrics.nodeIcon / 2
            const top = root.nodeY - r + 3
            const rows = 2 * r - 6
            const a = over.ink.getContext("2d").getImageData(root.ownLaneX, top, 1, rows).data
            const b = bare.ink.getContext("2d").getImageData(root.ownLaneX, top, 1, rows).data
            for (let i = 0; i < a.length; i++)
                compare(a[i], b[i], "channel " + (i % 4) + " at y " + (top + Math.floor(i / 4)))
        }
    }
}
