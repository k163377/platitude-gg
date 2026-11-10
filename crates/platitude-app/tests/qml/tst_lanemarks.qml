import QtQuick
import QtTest
import platitude.ui
import "painted.js" as Painted

// Where the marks of a row's node stand (`GraphLaneCell`): a face, the uncommitted row's ring, the ring of a row the
// discard log would bring back, and a shared commit's two faces — each on the centre the row gives it. Read off the
// canvas's own buffer, so every pixel of ink is a mark's or a lane the case drew itself. The gate's pictures are
// judged by no pixel, and a mark handed another centre still paints: only this says where.
Item {
    id: root

    /// The node's lane: far enough along that its centre's x is no row's y.
    readonly property int lane: 2
    readonly property point node: Qt.point(Metrics.laneInset + root.lane * Metrics.laneW + Metrics.laneW / 2,
                                           Theme.graphRowHeight / 2)
    readonly property real r: Metrics.nodeIcon / 2
    /// Where a shared commit seats its two faces: the author's a border up-left of the node, the first co-author's
    /// at the node's lower right.
    readonly property point author: Qt.point(root.node.x - Theme.borderWidth, root.node.y - Theme.borderWidth)
    readonly property point badge: Qt.point(root.node.x + Metrics.nodeIcon / 2 - 2 * Theme.borderWidth,
                                            Theme.graphRowHeight / 2 + Metrics.nodeIcon / 2)
    /// Every cell of the generated pattern set (the low 15 bits), in lane colour 1 and in lane colour 2: a face's
    /// middle is then one square of that colour (`GraphLaneCell.face`), and the two faces of a shared commit tell
    /// apart by it.
    readonly property int authorFace: (1 << 15) | 0x7fff
    readonly property int mateFace: (2 << 15) | 0x7fff
    readonly property var mate: [{ "name": "mate", "email": "mate@example.invalid", "face": root.mateFace }]
    /// The node's own lane, into it from above and out of it below: a line straight down the node's middle.
    readonly property var ownLane: [{ "kind": "into", "lane": root.lane, "color": 0, "dashed": false },
                                    { "kind": "out", "lane": root.lane, "color": 0, "dashed": false }]
    /// The application's own icon: opaque across its middle, clear only along its edge and at its corners.
    readonly property string picture: Qt.resolvedUrl("../../assets/icon-64.png")

    /// The cell's canvas — the one `InkCanvas` under it.
    function inkOf(cell) {
        return Painted.itemUnder(cell, child => child.inked !== undefined)
    }

    /// The box of the canvas's pixels `wanted(r, g, b, a)` picks — its edges, its middle and its size — or null where
    /// it picks none.
    function boxOf(ink, wanted) {
        const w = ink.width
        const h = ink.height
        const d = ink.getContext("2d").getImageData(0, 0, w, h).data
        let left = w
        let top = h
        let right = -1
        let bottom = -1
        for (let y = 0; y < h; y++) {
            for (let x = 0; x < w; x++) {
                const i = 4 * (y * w + x)
                if (!wanted(d[i], d[i + 1], d[i + 2], d[i + 3]))
                    continue
                left = Math.min(left, x)
                right = Math.max(right, x)
                top = Math.min(top, y)
                bottom = Math.max(bottom, y)
            }
        }
        if (right < 0)
            return null
        return {
            "left": left, "top": top, "right": right, "bottom": bottom,
            "midX": (left + right + 1) / 2, "midY": (top + bottom + 1) / 2,
            "w": right - left + 1, "h": bottom - top + 1
        }
    }

    /// Any ink at all.
    function inked(r, g, b, a) {
        return a > 0
    }

    /// The pixels one colour covers whole. `Qt.tint` with nothing over it turns a lane's string (`Theme.graphLane`)
    /// into a colour.
    function whole(colour) {
        const c = Qt.tint(colour, "transparent")
        return function (r, g, b, a) {
            return a === 255 && Math.abs(r - c.r * 255) <= 2 && Math.abs(g - c.g * 255) <= 2
                && Math.abs(b - c.b * 255) <= 2
        }
    }

    /// The side of a face's coloured square: `identiconFill` of its disc.
    function squareOf(radius) {
        return 2 * radius * Metrics.identiconFill
    }

    /// Whether every pixel within `reach` of `centre` is opaque.
    function opaqueAcross(ink, centre, reach) {
        const side = 2 * reach
        const d = ink.getContext("2d").getImageData(centre.x - reach, centre.y - reach, side, side).data
        for (let y = 0; y < side; y++) {
            for (let x = 0; x < side; x++) {
                const within = Math.hypot(x + 0.5 - reach, y + 0.5 - reach) <= reach
                if (within && d[4 * (y * side + x) + 3] !== 255)
                    return false
            }
        }
        return true
    }

    width: 200
    height: 60

    Component {
        id: fixture
        GraphLaneCell {
            width: Metrics.laneInset + (root.lane + 2) * Metrics.laneW
            height: Theme.graphRowHeight
            xOffset: 0
            fullWidth: width
            geometry: []
            nodeLane: root.lane
            coAuthors: []
            avatar: root.authorFace
            avatarUrl: ""
            isWip: false
            stashRef: ""
            dimmed: false
            ground: Theme.bgSurface
        }
    }

    TestCase {
        name: "LaneMarks"
        when: windowShown

        /// A cell built with everything it draws set, once its first paint — the whole picture — is in.
        function inkWith(props) {
            const cell = createTemporaryObject(fixture, root, props === undefined ? {} : props)
            verify(cell !== null)
            const ink = root.inkOf(cell)
            verify(ink !== null, "the cell draws on an InkCanvas")
            tryVerify(function () { return ink.inked }, undefined, "the first paint")
            return ink
        }

        /// A box stands on `centre` and is `size` across, to within a pixel of antialiasing a side.
        function standsOn(box, centre, size, what) {
            verify(box !== null, what + ": painted")
            verify(Math.abs(box.midX - centre.x) <= 1, what + ": x " + box.midX + " against " + centre.x)
            verify(Math.abs(box.midY - centre.y) <= 1, what + ": y " + box.midY + " against " + centre.y)
            verify(Math.abs(box.w - size) <= 2, what + ": " + box.w + " wide against " + size)
            verify(Math.abs(box.h - size) <= 2, what + ": " + box.h + " tall against " + size)
        }

        /// Its rim, the disc of its ground and the square of its pattern, each drawn from the centre on its own. The
        /// rim lies half over the ground's edge, so the ground shows a border less than its disc.
        function test_a_face_stands_on_its_node() {
            const ink = inkWith()
            standsOn(root.boxOf(ink, root.inked), root.node, 2 * root.r + Theme.borderWidth, "the face and its rim")
            standsOn(root.boxOf(ink, root.whole(Theme.bgElevated)), root.node, 2 * root.r - Theme.borderWidth,
                     "the face's ground")
            standsOn(root.boxOf(ink, root.whole(Theme.graphLane[1])), root.node, root.squareOf(root.r),
                     "the face's pattern")
        }

        /// An assigned picture is drawn over the face's whole disc: the icon is opaque across its middle, so a picture
        /// drawn anywhere else leaves part of that middle clear. Read two pixels inside the rim, clear of its
        /// antialiasing.
        function test_an_assigned_picture_fills_its_face() {
            const ink = inkWith({ "avatarUrl": root.picture })
            tryVerify(function () { return root.opaqueAcross(ink, root.node, root.r - 2) }, undefined,
                      "the picture across the face's middle")
        }

        /// The uncommitted row: a ring a pixel inside the node's room, and nothing in it.
        function test_the_uncommitted_ring_stands_on_its_node() {
            const ink = inkWith({ "isWip": true })
            standsOn(root.boxOf(ink, root.inked), root.node, 2 * (root.r - 1) + Metrics.laneStroke, "the ring")
            compare(ink.getContext("2d").getImageData(root.node.x, root.node.y, 1, 1).data[3], 0, "an empty node")
        }

        function test_a_row_the_log_would_bring_back_is_ringed_on_its_node_data() {
            return [
                { "tag": "commit", "props": { "provisional": true }, "face": true },
                { "tag": "stash", "props": { "provisional": true, "stashRef": "stash" }, "face": false },
            ]
        }

        /// The same ring round a commit's face, drawn smaller to stand clear of it, and round a stash's box.
        function test_a_row_the_log_would_bring_back_is_ringed_on_its_node(data) {
            const ink = inkWith(data.props)
            standsOn(root.boxOf(ink, root.inked), root.node, 2 * (root.r - 1) + Metrics.laneStroke, "the ring")
            if (!data.face)
                return
            const inner = root.r - 2 * Theme.borderWidth - Metrics.laneStroke / 2
            standsOn(root.boxOf(ink, root.whole(Theme.graphLane[1])), root.node, root.squareOf(inner),
                     "the face inside the ring")
        }

        /// A shared commit: the author's face steps up-left by a border — to the pixel, against the same face alone —
        /// and the first co-author's stands at the node's lower right, cut by the row's own bottom edge.
        function test_a_shared_commit_steps_its_author_and_seats_its_mate() {
            const alone = root.boxOf(inkWith(), root.whole(Theme.graphLane[1]))
            const ink = inkWith({ "coAuthors": root.mate })
            const author = root.boxOf(ink, root.whole(Theme.graphLane[1]))
            verify(alone !== null && author !== null, "both faces painted")
            compare(author.left, alone.left - Theme.borderWidth, "the author's face, a border left")
            compare(author.top, alone.top - Theme.borderWidth, "and a border up")
            const side = root.squareOf(Theme.iconSm / 2)
            const mate = root.boxOf(ink, root.whole(Theme.graphLane[2]))
            verify(mate !== null, "the co-author's face painted")
            verify(Math.abs(mate.midX - root.badge.x) <= 1, "the badge: x " + mate.midX + " against " + root.badge.x)
            verify(Math.abs(mate.w - side) <= 2, "the badge: " + mate.w + " wide against " + side)
            verify(Math.abs(mate.top - (root.badge.y - side / 2)) <= 1,
                   "the badge: its top at " + mate.top + " against " + (root.badge.y - side / 2))
        }

        /// The badge is punched out of the author's face a border wider than itself, about the badge's own centre: the
        /// walk from the badge's middle to the author's is whole ink but for that one moat at the badge's rim. A punch
        /// standing anywhere else leaves no moat there, or a wider one.
        function test_the_badge_s_moat_rings_the_badge() {
            const ink = inkWith({ "coAuthors": root.mate })
            const far = Math.hypot(root.author.x - root.badge.x, root.author.y - root.badge.y)
            const w = ink.width
            const d = ink.getContext("2d").getImageData(0, 0, w, ink.height).data
            let first = -1
            let last = -1
            for (let s = 2; s <= far; s += 0.25) {
                const x = Math.floor(root.badge.x + s * (root.author.x - root.badge.x) / far)
                const y = Math.floor(root.badge.y + s * (root.author.y - root.badge.y) / far)
                if (d[4 * (y * w + x) + 3] === 255)
                    continue
                if (first < 0)
                    first = s
                last = s
            }
            const br = Theme.iconSm / 2
            verify(first >= 0, "a moat between the two faces")
            // A sample reads the pixel it falls in, whose own middle is up to a pixel's half diagonal off the walk.
            verify(first >= br - 1.5 && last <= br + Theme.borderWidth + 1.5,
                   "the moat from " + first + " to " + last + " of the badge's middle, against " + br + " to "
                   + (br + Theme.borderWidth))
        }

        /// A dimmed face is darker, not see-through: its disc is punched out of the lanes before it goes on. On a
        /// shared commit that disc stands with the author's face, a border up-left of the node: down the node's own
        /// lane, from the first row wholly inside it to the face's middle, the row reads pixel for pixel as it does
        /// with no lane drawn. A punch left on the node lets the lane through the face's top row.
        function test_a_dimmed_shared_face_is_punched_where_it_stands() {
            const over = inkWith({ "coAuthors": root.mate, "dimmed": true, "geometry": root.ownLane })
            const bare = inkWith({ "coAuthors": root.mate, "dimmed": true })
            const top = Math.ceil(root.author.y - (root.r + Theme.borderWidth / 2))
            const rows = root.author.y - top
            // The two pixel columns the lane covers (a `laneStroke` line on a whole-pixel centre).
            const left = root.node.x - Metrics.laneStroke / 2
            const a = over.getContext("2d").getImageData(left, top, Metrics.laneStroke, rows).data
            const b = bare.getContext("2d").getImageData(left, top, Metrics.laneStroke, rows).data
            verify(a.length > 0, "the lane's columns are on the canvas")
            for (let i = 0; i < a.length; i++) {
                const at = Math.floor(i / 4)
                compare(a[i], b[i], "channel " + (i % 4) + " at x " + (left + at % Metrics.laneStroke) + ", y "
                        + (top + Math.floor(at / Metrics.laneStroke)))
            }
        }
    }
}
