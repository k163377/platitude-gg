import QtQuick
import QtTest
import platitude.ui

// The graph column's right edge (デザイン規約 §グラフ列は最も広い所のレーンまで): past it a lane and a face sink into the
// ground together, over `spaceSm`, and the edge stays with the column while the lanes are sent sideways. Read off the
// window — the dissolve is laid over the canvas, so the canvas's own buffer does not show it.
Item {
    id: root
    width: 400
    height: 2 * Theme.graphRowHeight

    /// Lane 1's centre, where its node's face is centred too.
    readonly property int laneX: Metrics.laneInset + Metrics.laneW + Metrics.laneW / 2
    /// A pixel column the lane covers whole (a `laneStroke` line on a whole-pixel centre), inside the face's pattern.
    readonly property int inkX: root.laneX - 1
    /// A row the lane crosses above the face, and the face's middle row.
    readonly property int laneY: 1
    readonly property int faceY: Theme.graphRowHeight / 2
    /// Every cell of the generated pattern set, in lane colour 0 (`GraphLaneCell.face`): the face's middle is that
    /// colour throughout — the lane's own, so where the two stand at one x their pixels agree, whole or sinking.
    readonly property int solidFace: 0x7fff

    function lanes(count) {
        const out = []
        for (let l = 0; l < count; l++)
            out.push({ "kind": "through", "lane": l, "color": l, "dashed": false })
        return out
    }

    // A row's lanes over its ground, with the strip past the column the dissolve covers. Built with everything it draws
    // set, so its first paint is the whole picture.
    Component {
        id: fixture
        Item {
            id: box
            property color ground: Theme.bgSurface
            property real columnW: 60
            property var geometry: [{ "kind": "through", "lane": 1, "color": 0, "dashed": false }]
            property real xOffset: 0
            readonly property alias cell: laneCell
            width: box.columnW + Theme.spaceSm + Metrics.laneW
            height: Theme.graphRowHeight
            Rectangle {
                anchors.fill: parent
                color: box.ground
            }
            GraphLaneCell {
                id: laneCell
                width: box.columnW
                height: Theme.graphRowHeight
                xOffset: box.xOffset
                fullWidth: 400
                geometry: box.geometry
                nodeLane: 1
                coAuthors: []
                avatar: root.solidFace
                avatarUrl: ""
                isWip: false
                stashRef: ""
                dimmed: false
                ground: box.ground
            }
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

    TestCase {
        name: "LaneEdge"
        when: windowShown

        function boxWithSpy(props) {
            const box = createTemporaryObject(fixture, root, props)
            verify(box !== null)
            const ink = root.inkOf(box.cell)
            verify(ink !== null, "the cell draws on an InkCanvas")
            const spy = createTemporaryObject(paints, root, { "target": ink })
            verify(spy !== null)
            tryCompare(spy, "count", 1, undefined, "the first paint")
            return { "box": box, "spy": spy }
        }

        /// Channel by channel, to within the rounding of two blends. `Qt.tint` with nothing over it turns a lane's
        /// string (`Theme.graphLane`) into a colour.
        function near(a, b, what) {
            const want = Qt.tint(b, "transparent")
            const d = Math.max(Math.abs(a.r - want.r), Math.abs(a.g - want.g), Math.abs(a.b - want.b)) * 255
            verify(d <= 2, what + ": " + a + " against " + want)
        }

        function test_a_lane_and_a_face_sink_alike_data() {
            return [
                { "tag": "plain", "ground": Theme.bgSurface },
                { "tag": "selected", "ground": Theme.bgSelected },
                { "tag": "lit", "ground": Qt.tint(Theme.bgSurface, Theme.bgHover) },
            ]
        }

        /// The column's edge walked across the lane and its face, from short of it to the message tick: at every step
        /// the face's pixel is the lane's. Each step is its own row, alone on screen while it is read.
        function test_a_lane_and_a_face_sink_alike(data) {
            for (let past = -2; past <= Theme.spaceSm; past++) {
                const t = boxWithSpy({ "ground": data.ground, "columnW": root.inkX - past })
                const shot = grabImage(t.box)
                t.box.visible = false
                const lane = shot.pixel(root.inkX, root.laneY)
                near(shot.pixel(root.inkX, root.faceY), lane, "the face against the lane, " + past + "px past the edge")
                if (past < 0)
                    near(lane, Theme.graphLane[0], "whole inside the column")
                if (past === Theme.spaceSm)
                    near(lane, data.ground, "gone into the ground at the message tick")
            }
        }

        /// A hand sends the lanes a pixel at a time: the canvas widens once, then only slides. The picture it lands on
        /// is the one a row built at that offset draws — the edge has not travelled with the lanes.
        function test_the_edge_stays_with_the_column_while_the_lanes_slide() {
            const slid = boxWithSpy({ "geometry": root.lanes(10) })
            slid.box.xOffset = 1
            tryCompare(slid.spy, "count", 2, undefined, "the paint the widening asks for")
            slid.box.xOffset = 3 * Metrics.laneW
            const built = boxWithSpy({ "geometry": root.lanes(10), "xOffset": 3 * Metrics.laneW })
            compare(slid.spy.count, 2, "the slide itself paints nothing")
            // The two share a place on screen: each is read alone.
            built.box.visible = false
            const a = grabImage(slid.box)
            slid.box.visible = false
            built.box.visible = true
            const b = grabImage(built.box)
            for (let x = 0; x < a.width; x++) {
                for (let y = 0; y < a.height; y += 3)
                    near(a.pixel(x, y), b.pixel(x, y), "at " + x + "," + y)
            }
        }
    }
}
