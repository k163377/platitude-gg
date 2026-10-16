import QtQuick
import platitude.ui

// Hand-drawn 16px-grid icons, scaled by the item size, or for a mark set beside words by their capitals (`set`),
// in one stroke colour; `home` alone is filled.
InkCanvas {
    id: icon
    property string kind: "branch"
    property color tint: Theme.textSecondary
    /// Line weight of a mark drawn to its box (`set` empty — ④ and ⑤ of デザイン規約 §アイコンの席): the coordinates
    /// scale with the size and this does not. A mark set beside words ignores it and draws the words' stem (`pen`).
    property real stroke: Metrics.iconStroke
    /// Where each kind's ink can reach, in the 16-grid as it is drawn below: `[x, y, reach]`, `reach` being how many
    /// half lines the ink runs past the point in every direction — 0 on a filled edge, 1 on a round end or a stroked
    /// edge, √2 on a mitred corner turned 45° (`tag`, `pen`). The ink is the box round these, and every mark is drawn
    /// moved so that box stands in the middle of its own (デザイン規約 §アイコンの席) — across only for a mark set beside
    /// words, which stands against the words down (`shiftY`). A box centred on a line puts the ink on that line's
    /// middle, at every size and every line weight, with nothing for the caller to add.
    /// `tst_iconink.qml` holds each list to the pixels drawn.
    readonly property var reaches: {
        switch (icon.kind) {
        case "branch": return [[3.2, 3.5, 1], [12.8, 5, 1], [5, 1.7, 1], [5, 14.3, 1]]
        case "remote": return [[1.7, 10, 1], [14.6, 9.5, 1], [8.05, 3.1, 1], [8, 12.9, 1]]
        case "tag": return [[8, 3.41, 1.41], [8, 13.59, 1.41], [2.91, 8.5, 1.41], [13.09, 8.5, 1.41]]
        case "stash": return [[3, 4, 1], [13, 4, 1], [4, 13, 1], [12, 13, 1]]
        // As tall as `branch`, ring and stem (`lettered`).
        case "tree": return [[3.5, 6, 1], [12.5, 6, 1], [8, 1.5, 1], [8, 14.1, 1]]
        // The body's four corners turned 45°, and the filled point.
        case "pen": return [[11.25, 2.77, 1.41], [13.23, 4.75, 1.41], [5.24, 8.78, 1.41], [7.22, 10.76, 1.41],
                            [4.11, 11.89, 0]]
        case "plus": return [[3.5, 8, 1], [12.5, 8, 1], [8, 3.5, 1], [8, 12.5, 1]]
        case "minus": return [[3.5, 8, 1], [12.5, 8, 1]]
        case "window-minimize": return [[3, 8, 1], [13, 8, 1]]
        case "arrow": return [[3.5, 8, 1], [11.5, 8, 1], [8.8, 5.2, 1], [8.8, 10.8, 1]]
        case "fetch": return [[8, 2.5, 1], [3.5, 13, 1], [12.5, 13, 1], [4.8, 6.8, 1], [11.2, 6.8, 1]]
        // The barbs meet the point in a mitred right angle.
        case "push": return [[8, 2.5, 1.41], [3.5, 13, 1], [12.5, 13, 1], [4.8, 5.7, 1], [11.2, 5.7, 1]]
        case "spinner": return [[3, 8, 1], [13, 8, 1], [8, 3, 1], [8, 13, 1]]
        // The ring from the barb round to just under the left, and the filled barb.
        case "history": return [[8, 2.5, 1], [13.5, 8, 1], [8, 13.5, 1], [2.53, 8.57, 1], [3.63, 4.66, 1],
                                [2.02, 3.4, 0], [2.02, 6.48, 0], [5.12, 6.48, 0]]
        case "copyicon": return [[3.5, 9, 1], [12.5, 7, 1], [9, 3.5, 1], [7, 12.5, 1]]
        // The stroke above, the filled dot below.
        case "bang": return [[8, 3.5, 1], [8, 10, 1], [7, 12.8, 0], [9, 12.8, 0], [8, 13.8, 0]]
        case "hier": return [[3, 3, 1], [13, 6.5, 1], [13, 13.5, 1], [10, 13.5, 1]]
        // The filled dots stand out past the lines' ends.
        case "list": return [[2.5, 4.5, 0], [3.4, 3.6, 0], [3.4, 12.4, 0], [13, 4.5, 1], [13, 11.5, 1], [5.5, 4.5, 1],
                             [5.5, 11.5, 1]]
        case "pr": return [[2.8, 3.8, 1], [2.8, 12.2, 1], [13.2, 12.2, 1], [4.5, 2.1, 1], [4.5, 13.9, 1],
                           [11.5, 13.9, 1], [8.8, 2.5, 1]]
        // The tick's corner is a mitred join, its point a half line and more below the corner.
        case "check": return [[3.5, 8.5, 1], [6.8, 11.8, 1.41], [12.5, 4.5, 1]]
        case "lock": return [[3.5, 8.2, 1], [12.5, 13.5, 1], [8, 4.4, 1]]
        // Filled throughout.
        case "home": return [[2.5, 6.5, 0], [8, 2.3, 0], [13.5, 13.7, 0], [2.5, 13.7, 0]]
        case "folder": return [[2.5, 4.5, 1], [13.5, 12.5, 1], [2.5, 12.5, 1]]
        case "chevron": return [[6, 4, 1], [10, 8, 1], [6, 12, 1]]
        case "chevrons": return [[4, 4, 1], [13, 8, 1], [4, 12, 1]]
        case "terminal": return [[3, 4, 1], [7, 8, 1], [3, 12, 1], [13, 12, 1]]
        // A resize corner hangs off the box's lower right and is not centred (`centred`).
        case "grip": return [[6, 12, 1], [12, 6, 1], [10, 12, 1], [12, 10, 1]]
        case "menu": return [[3, 4.5, 1], [13, 11.5, 1]]
        case "close": return [[4.5, 4.5, 1], [11.5, 11.5, 1]]
        // The teeth's outer corners on the axes.
        case "gear": return [[2.03, 8, 1], [13.97, 8, 1], [8, 2.03, 1], [8, 13.97, 1]]
        case "window-maximize":
        case "window-restore": return [[3.5, 3.5, 1], [12.5, 12.5, 1]]
        case "app-window": return [[2.5, 3.5, 1], [13.5, 12.5, 1]]
        case "unified":
        case "split": return [[3, 4, 1], [13, 12, 1]]
        // The lens's two curves meet at a mitred point at each end.
        case "eye": return [[2.5, 8, 1.36], [13.5, 8, 1.36], [8, 5, 1], [8, 11, 1]]
        case "eye-off": return [[2.5, 8, 1.36], [13.5, 8, 1.36], [3, 13, 1], [13, 3, 1]]
        case "no":
        case "no-entry": return [[2.5, 8, 1], [13.5, 8, 1], [8, 2.5, 1], [8, 13.5, 1]]
        case "search": return [[2.8, 7, 1], [7, 2.8, 1], [13.2, 13.2, 1]]
        default: return []
        }
    }
    /// Whether the drawing is moved to stand its ink in the middle of the box. Only the resize corner is not.
    readonly property bool centred: icon.kind !== "grip"
    /// Set beside words (デザイン規約 §アイコンの席): the mark is drawn as a letter of the words' face rather than to its
    /// box — its size, its line and where it stands all come off the words. Empty for a mark that stands alone, drawn
    /// to the middle of its box as above.
    ///   - "name": a letter of the words, a state after a name included — the capitals' scale, on their middle; its
    ///     cell the ink, with the row's step to the words kept by the caller, or the seat of the column it stands in
    ///   - "lead": the head of a row or a phrase — as "name", in the seat it is given
    ///   - "head": the head of a heading — a step over the capitals (`Metrics.markHead`), on their middle
    ///   - "sign": a sign on the words — the capitals' scale, on the face's own `+`'s middle and never under the
    ///     baseline, in the face's own room round its ink and a stem more
    property string set: ""
    /// The words' pixel size: what each measure of a set mark is a share of.
    property real size: Theme.fontMd
    /// Where the words' baseline runs, in this item's y.
    property real wordBase: icon.height
    /// Quarter turns, clockwise, of a set mark's drawing — not `rotation`, which would turn its place against the
    /// words with it.
    property int turn: 0
    /// `reaches`, turned with the drawing about the grid's middle.
    readonly property var points: icon.turn % 360 === 0 ? icon.reaches
        : icon.reaches.map(p => icon.turn === 90 || icon.turn === -270 ? [16 - p[1], p[0], p[2]]
                              : icon.turn === -90 || icon.turn === 270 ? [p[1], 16 - p[0], p[2]]
                              : [16 - p[0], 16 - p[1], p[2]])
    /// Whether a set mark stands among the letters in every class, a sign's included: `tree` is drawn as tall as
    /// `branch` and stands where it does, on the capitals' middle — past them a little either way, its stem's foot
    /// under the baseline with the branch's — never on a sign's middle or stopped at the baseline, which would drop
    /// its ring under the names' tall letters or stand its foot on the line (デザイン規約 §アイコンの席).
    readonly property bool lettered: icon.kind === "tree"
    /// What a set mark is drawn against: `height`, the capitals' (a step over them for a heading), and `middle`, where
    /// the marks' middles stand from the baseline — the capitals' middle for a mark among the words, the face's own
    /// `+`'s for a sign (`sign`).
    readonly property var target: {
        if (icon.set === "")
            return null
        const step = icon.set === "head" ? Metrics.markHead : 1
        const sign = icon.set === "sign" && !icon.lettered
        return { height: WordFace.capital * icon.size * step,
                 middle: sign ? (WordFace.plus.y + WordFace.plus.height / 2) * icon.size
                              : -WordFace.capital * icon.size / 2,
                 sign: sign }
    }
    /// The line: the stroke given, or for a set mark the words' stem. Not `line`, as `wordCell` is not `cell`: an
    /// instance built in a component of its own (a `Loader`'s, a delegate's) looks its own properties up before the
    /// ids of the file around that component, so a property of an id's name hides it (`NavFactLine`'s `line`).
    readonly property real pen: icon.set === "" ? icon.stroke
                                 : Metrics.markStem * icon.size * (icon.set === "head" ? Metrics.markHead : 1)
    /// Pixels to a square of the 16-grid: the box's sixteenth, or for a set mark one scale for every kind, as a face
    /// has one em for every letter — the grid's own proportions between the marks are the drawing (a tall branch, a
    /// squat stash), and fitting each to one height would thin the one and fatten the other. 11.5 squares, line ends
    /// to line ends, are the capitals' height.
    readonly property real unit: icon.set === "" ? icon.width / 16
                                 : Math.max(0, icon.target.height - icon.pen) / 11.5
    /// The ink's box in the 16-grid, line included, before any move: `[left, top, right, bottom]`. A grid square is
    /// `unit` pixels and the line keeps its pixels, so the half line is worth more of the grid on a small mark.
    readonly property var inkBox: icon.inkAt(icon.unit)
    /// How far the drawing moves, in grid squares: to stand its ink in the middle of the box, or for a set mark to its
    /// place against the words — the middle across; down, on the capitals' middle, or for a sign the face's own `+`'s
    /// middle, stopped at the baseline.
    readonly property real shiftX: {
        if (!icon.centred)
            return 0
        if (icon.set === "")
            return 8 - (icon.inkBox[0] + icon.inkBox[2]) / 2
        return Math.round((icon.width - icon.inkAcross) / 2) / icon.unit - icon.inkBox[0]
    }
    readonly property real shiftY: {
        if (!icon.centred)
            return 0
        if (icon.set === "")
            return 8 - (icon.inkBox[1] + icon.inkBox[3]) / 2
        // Every other mark on its middle line, whatever its height: a squat mark stood on the baseline reads as
        // dropped, a tall one centred runs past the capitals both ways. A sign never goes under the baseline — that
        // band is the descenders', and a sign hanging in it reads as dropped.
        const middle = Math.round(icon.wordBase + icon.target.middle - icon.inkDown / 2)
        const top = icon.target.sign ? Math.min(middle, Math.round(icon.wordBase) - icon.inkDown) : middle
        return top / icon.unit - icon.inkBox[1]
    }
    /// The ink's span in the item's pixels, line included, across and down the mark as drawn (`turn` included,
    /// `rotation` not).
    readonly property real inkAcross: (icon.inkBox[2] - icon.inkBox[0]) * icon.unit
    readonly property real inkDown: (icon.inkBox[3] - icon.inkBox[1]) * (icon.set === "" ? icon.height / 16 : icon.unit)
    /// The span along the item's own axes: a quarter turn (`rotation`) swaps the two.
    readonly property real inkWidth: icon.rotation % 180 === 0 ? icon.inkAcross : icon.inkDown
    readonly property real inkHeight: icon.rotation % 180 === 0 ? icon.inkDown : icon.inkAcross
    /// The ink's right edge and top edge in the item's pixels, before any `rotation`: `width - inkRight` is the air the
    /// box holds on that side, which is the same on the other (デザイン規約 §アイコンの席).
    readonly property real inkRight: (icon.inkBox[2] + icon.shiftX) * icon.unit
    readonly property real inkTop: (icon.inkBox[1] + icon.shiftY) * (icon.set === "" ? icon.height / 16 : icon.unit)
    /// The width a set mark takes in its run of words: for a sign, the face's own room round a `+` either side of the
    /// ink and a stem more, so it stands off its word by a hair rather than touching it; for any other, the ink alone.
    readonly property real wordCell: Math.round(icon.set === "sign"
                                                ? icon.inkAcross + 2 * (WordFace.signSide * icon.size + icon.pen)
                                                : icon.inkAcross)

    /// The ink's box at `u` pixels to the grid square, the line keeping its pixels.
    function inkAt(u) {
        const half = u > 0 ? icon.pen / 2 / u : 0
        let box = [16, 16, 0, 0]
        for (const p of icon.points) {
            const out = p[2] * half
            box = [Math.min(box[0], p[0] - out), Math.min(box[1], p[1] - out),
                   Math.max(box[2], p[0] + out), Math.max(box[3], p[1] + out)]
        }
        return icon.reaches.length > 0 ? box : [0, 0, 16, 16]
    }

    width: Theme.iconMd
    height: Theme.iconMd
    onKindChanged: requestPaint()
    onTintChanged: requestPaint()
    onPenChanged: requestPaint()
    onTurnChanged: requestPaint()
    onShiftXChanged: requestPaint()
    onShiftYChanged: requestPaint()
    onUnitChanged: requestPaint()
    onPaint: {
        const ctx = getContext("2d")
        const s = icon.unit
        // The context keeps its transform from paint to paint, so the move below starts from nothing each time.
        ctx.setTransform(1, 0, 0, 1, 0, 0)
        ctx.clearRect(0, 0, width, height)
        ctx.translate(icon.shiftX * s, icon.shiftY * s)
        if (icon.turn % 360 !== 0) {
            ctx.translate(8 * s, 8 * s)
            ctx.rotate(icon.turn * Math.PI / 180)
            ctx.translate(-8 * s, -8 * s)
        }
        ctx.strokeStyle = icon.tint
        ctx.fillStyle = icon.tint
        ctx.lineWidth = icon.pen
        ctx.lineCap = "round"
        // Reset every paint: the context outlives a `kind`, and a round join set below would carry to the next mark.
        ctx.lineJoin = "miter"
        if (icon.kind === "branch") {
            // A line stops on the ring's circle, so its cap reaches the hole's edge at every size. Drawn to the
            // centre, it stands in the hole as a spike.
            ctx.beginPath()
            ctx.moveTo(5 * s, 5.3 * s)
            ctx.lineTo(5 * s, 10.7 * s)
            ctx.stroke()
            ctx.beginPath()
            ctx.moveTo(11 * s, 6.8 * s)
            ctx.bezierCurveTo(11 * s, 8.6 * s, 8.6 * s, 9.2 * s, 5.6 * s, 9.2 * s)
            ctx.stroke()
            for (const c of [[5, 3.5], [5, 12.5], [11, 5]]) {
                ctx.beginPath()
                ctx.arc(c[0] * s, c[1] * s, 1.8 * s, 0, 2 * Math.PI)
                ctx.stroke()
            }
        } else if (icon.kind === "remote") {
            // Three lobes off a flat base, as wide as its siblings (smaller reads as shrunk beside a name). The
            // middle lobe rises alone — three of a size read as bumps — and the ends stay well below it: the line
            // does not shrink, so at `iconSm` a shallower valley fills with ink. The angles are where the circles
            // cross; an arc stopped short is joined by a straight line that dents the outline.
            ctx.lineJoin = "round"
            ctx.beginPath()
            ctx.arc(4.6 * s, 10 * s, 2.9 * s, Math.PI * 0.5, Math.PI * 1.445)
            ctx.arc(8.05 * s, 7.05 * s, 3.95 * s, Math.PI * 0.9925, Math.PI * 1.9287)
            ctx.arc(11.2 * s, 9.5 * s, 3.4 * s, Math.PI * 1.5661, Math.PI * 0.5)
            ctx.closePath()
            ctx.stroke()
        } else if (icon.kind === "tag") {
            ctx.save()
            ctx.translate(8 * s, 8.5 * s)
            ctx.rotate(Math.PI / 4)
            ctx.strokeRect(-3.6 * s, -3.6 * s, 7.2 * s, 7.2 * s)
            ctx.beginPath()
            ctx.arc(-1.4 * s, -1.4 * s, 1 * s, 0, 2 * Math.PI)
            ctx.fill()
            ctx.restore()
        } else if (icon.kind === "stash") {
            ctx.strokeRect(3 * s, 4 * s, 10 * s, 3 * s)
            ctx.strokeRect(4 * s, 7 * s, 8 * s, 6 * s)
            ctx.beginPath()
            ctx.moveTo(6.5 * s, 9.5 * s)
            ctx.lineTo(9.5 * s, 9.5 * s)
            ctx.stroke()
        } else if (icon.kind === "tree") {
            // The ring and the stem as tall as `branch` (`lettered`).
            ctx.beginPath()
            ctx.arc(8 * s, 6 * s, 4.5 * s, 0, 2 * Math.PI)
            ctx.stroke()
            ctx.beginPath()
            ctx.moveTo(8 * s, 10.5 * s)
            ctx.lineTo(8 * s, 14.1 * s)
            ctx.stroke()
        } else if (icon.kind === "pen") {
            ctx.save()
            ctx.translate(8 * s, 8 * s)
            ctx.rotate(Math.PI / 4)
            ctx.strokeRect(-1.4 * s, -6 * s, 2.8 * s, 8.5 * s)
            ctx.beginPath()
            ctx.moveTo(-1.4 * s, 2.5 * s)
            ctx.lineTo(0, 5.5 * s)
            ctx.lineTo(1.4 * s, 2.5 * s)
            ctx.closePath()
            ctx.fill()
            ctx.restore()
        } else if (icon.kind === "plus") {
            ctx.beginPath()
            ctx.moveTo(8 * s, 3.5 * s)
            ctx.lineTo(8 * s, 12.5 * s)
            ctx.moveTo(3.5 * s, 8 * s)
            ctx.lineTo(12.5 * s, 8 * s)
            ctx.stroke()
        } else if (icon.kind === "minus") {
            ctx.beginPath()
            ctx.moveTo(3.5 * s, 8 * s)
            ctx.lineTo(12.5 * s, 8 * s)
            ctx.stroke()
        } else if (icon.kind === "arrow") {
            ctx.beginPath()
            ctx.moveTo(3.5 * s, 8 * s)
            ctx.lineTo(11.5 * s, 8 * s)
            ctx.moveTo(11.5 * s, 8 * s)
            ctx.lineTo(8.8 * s, 5.2 * s)
            ctx.moveTo(11.5 * s, 8 * s)
            ctx.lineTo(8.8 * s, 10.8 * s)
            ctx.stroke()
        } else if (icon.kind === "fetch" || icon.kind === "push") {
            // One shape mirrored: the base line is this repository; the arrow lands on it (fetch) or leaves it (push).
            const down = icon.kind === "fetch"
            const tip = down ? 10 : 2.5
            const tail = down ? 2.5 : 10
            const barb = down ? tip - 3.2 : tip + 3.2
            ctx.beginPath()
            ctx.moveTo(8 * s, tail * s)
            ctx.lineTo(8 * s, tip * s)
            ctx.moveTo(4.8 * s, barb * s)
            ctx.lineTo(8 * s, tip * s)
            ctx.lineTo(11.2 * s, barb * s)
            ctx.moveTo(3.5 * s, 13 * s)
            ctx.lineTo(12.5 * s, 13 * s)
            ctx.stroke()
        } else if (icon.kind === "spinner") {
            // Three quarters of a ring: the gap is what shows it turning.
            ctx.beginPath()
            ctx.arc(8 * s, 8 * s, 5 * s, -Math.PI / 2, Math.PI)
            ctx.stroke()
        } else if (icon.kind === "history") {
            // Octicons `history-16` on this grid: its 1.5 stroke is ours, so the ring is its centreline (radius 7.25)
            // brought in to `no-entry`'s 5.5. The ring runs clockwise from the barb (217.4°) round to just under the
            // left (174.1°); the barb is the filled corner it leaves from, and the hands meet a step left of centre as
            // the source's do.
            ctx.lineJoin = "round"
            ctx.beginPath()
            ctx.arc(8 * s, 8 * s, 5.5 * s, 217.4 * Math.PI / 180, (174.1 + 360) * Math.PI / 180)
            ctx.stroke()
            ctx.beginPath()
            ctx.moveTo(2.02 * s, 3.4 * s)
            ctx.lineTo(2.02 * s, 6.48 * s)
            ctx.lineTo(5.12 * s, 6.48 * s)
            ctx.closePath()
            ctx.fill()
            ctx.beginPath()
            ctx.moveTo(7.81 * s, 5.53 * s)
            ctx.lineTo(7.81 * s, 8.19 * s)
            ctx.lineTo(9.71 * s, 8.95 * s)
            ctx.stroke()
        } else if (icon.kind === "copyicon") {
            // Corner radius 1 of the grid at most: rounder, the two overlapping sheets read as blobs. Paths, since
            // `strokeRect` takes no radius.
            for (const sheet of [[5.5, 3.5], [3.5, 5.5]]) {
                ctx.beginPath()
                ctx.roundedRect(sheet[0] * s, sheet[1] * s, 7 * s, 7 * s, 1 * s, 1 * s)
                ctx.stroke()
            }
        } else if (icon.kind === "bang") {
            ctx.beginPath()
            ctx.moveTo(8 * s, 3.5 * s)
            ctx.lineTo(8 * s, 10 * s)
            ctx.stroke()
            ctx.beginPath()
            ctx.arc(8 * s, 12.8 * s, 1 * s, 0, 2 * Math.PI)
            ctx.fill()
        } else if (icon.kind === "hier") {
            ctx.strokeRect(3 * s, 3 * s, 3 * s, 3 * s)
            ctx.beginPath()
            ctx.moveTo(4.5 * s, 6 * s)
            ctx.lineTo(4.5 * s, 12 * s)
            ctx.moveTo(4.5 * s, 8 * s)
            ctx.lineTo(10 * s, 8 * s)
            ctx.moveTo(4.5 * s, 12 * s)
            ctx.lineTo(10 * s, 12 * s)
            ctx.stroke()
            ctx.strokeRect(10 * s, 6.5 * s, 3 * s, 3 * s)
            ctx.strokeRect(10 * s, 10.5 * s, 3 * s, 3 * s)
        } else if (icon.kind === "list") {
            ctx.beginPath()
            for (const ly of [4.5, 8, 11.5]) {
                ctx.moveTo(5.5 * s, ly * s)
                ctx.lineTo(13 * s, ly * s)
            }
            ctx.stroke()
            for (const ly of [4.5, 8, 11.5]) {
                ctx.beginPath()
                ctx.arc(3.4 * s, ly * s, 0.9 * s, 0, 2 * Math.PI)
                ctx.fill()
            }
        } else if (icon.kind === "pr") {
            // GitHub-style pull request: left commit line, right elbow arrow into the merge node.
            ctx.beginPath()
            ctx.moveTo(4.5 * s, 5.5 * s)
            ctx.lineTo(4.5 * s, 10.5 * s)
            ctx.stroke()
            for (const c of [[4.5, 3.8], [4.5, 12.2], [11.5, 12.2]]) {
                ctx.beginPath()
                ctx.arc(c[0] * s, c[1] * s, 1.7 * s, 0, 2 * Math.PI)
                ctx.stroke()
            }
            ctx.beginPath()
            ctx.moveTo(7.4 * s, 3.8 * s)
            ctx.lineTo(9.8 * s, 3.8 * s)
            ctx.quadraticCurveTo(11.5 * s, 3.8 * s, 11.5 * s, 5.5 * s)
            ctx.lineTo(11.5 * s, 10.5 * s)
            ctx.stroke()
            ctx.beginPath()
            ctx.moveTo(8.8 * s, 2.5 * s)
            ctx.lineTo(7.2 * s, 3.8 * s)
            ctx.lineTo(8.8 * s, 5.1 * s)
            ctx.stroke()
        } else if (icon.kind === "check") {
            ctx.beginPath()
            ctx.moveTo(3.5 * s, 8.5 * s)
            ctx.lineTo(6.8 * s, 11.8 * s)
            ctx.lineTo(12.5 * s, 4.5 * s)
            ctx.stroke()
        } else if (icon.kind === "lock") {
            // Drawn, since no family in the font chain has U+1F512. The shackle keeps its legs: an arc landing on
            // the body's top edge merges with it and reads as a lidded box.
            ctx.beginPath()
            ctx.moveTo(5.6 * s, 8.2 * s)
            ctx.lineTo(5.6 * s, 6.8 * s)
            ctx.arc(8 * s, 6.8 * s, 2.4 * s, Math.PI, 2 * Math.PI)
            ctx.lineTo(10.4 * s, 8.2 * s)
            ctx.stroke()
            ctx.strokeRect(3.5 * s, 8.2 * s, 9 * s, 5.3 * s)
        } else if (icon.kind === "home") {
            // Filled, and proportioned off octicons / Feather (デザイン規約 §左メニューの所作「家の印だけは塗りで描く」).
            // The apex keeps the miter join: rounded, the slopes close into a dome.
            ctx.beginPath()
            ctx.moveTo(2.5 * s, 6.5 * s)
            ctx.lineTo(8 * s, 2.3 * s)
            ctx.lineTo(13.5 * s, 6.5 * s)
            ctx.lineTo(13.5 * s, 13.7 * s)
            ctx.lineTo(2.5 * s, 13.7 * s)
            ctx.closePath()
            // The doorway, wound against the outline so the non-zero fill leaves it open (this part is never told
            // the background's colour).
            ctx.moveTo(6.9 * s, 13.7 * s)
            ctx.lineTo(9.1 * s, 13.7 * s)
            ctx.lineTo(9.1 * s, 9.3 * s)
            ctx.lineTo(6.9 * s, 9.3 * s)
            ctx.closePath()
            ctx.fill()
        } else if (icon.kind === "folder") {
            ctx.beginPath()
            ctx.moveTo(2.5 * s, 12.5 * s)
            ctx.lineTo(2.5 * s, 4.5 * s)
            ctx.lineTo(6.5 * s, 4.5 * s)
            ctx.lineTo(8 * s, 6 * s)
            ctx.lineTo(13.5 * s, 6 * s)
            ctx.lineTo(13.5 * s, 12.5 * s)
            ctx.closePath()
            ctx.stroke()
        } else if (icon.kind === "chevron") {
            // One of `chevrons`' pair, for "opens a list" — a fold, a combo's list, a menu's card. The pair moves a
            // pane's edge, and one mark cannot mean two things. Same grid and stroke as the `spinner` ring `AppCombo`
            // swaps it with.
            ctx.lineJoin = "round"
            ctx.beginPath()
            ctx.moveTo(6 * s, 4 * s)
            ctx.lineTo(10 * s, 8 * s)
            ctx.lineTo(6 * s, 12 * s)
            ctx.stroke()
        } else if (icon.kind === "chevrons") {
            // Drawn, not a guillemet: that sits on the lowercase band and centres low in the button.
            ctx.lineJoin = "round"
            for (const x of [4, 9]) {
                ctx.beginPath()
                ctx.moveTo(x * s, 4 * s)
                ctx.lineTo((x + 4) * s, 8 * s)
                ctx.lineTo(x * s, 12 * s)
                ctx.stroke()
            }
        } else if (icon.kind === "terminal") {
            // Drawn, not typed: as text it sits low and light among the marks. The chevron is `chevron`'s, moved left
            // for the cursor.
            ctx.lineJoin = "round"
            ctx.beginPath()
            ctx.moveTo(3 * s, 4 * s)
            ctx.lineTo(7 * s, 8 * s)
            ctx.lineTo(3 * s, 12 * s)
            ctx.stroke()
            ctx.beginPath()
            ctx.moveTo(9 * s, 12 * s)
            ctx.lineTo(13 * s, 12 * s)
            ctx.stroke()
        } else if (icon.kind === "grip") {
            // A resize corner, as browsers draw it: inset like `close`, hung off the lower right.
            ctx.beginPath()
            ctx.moveTo(12 * s, 6 * s)
            ctx.lineTo(6 * s, 12 * s)
            ctx.moveTo(12 * s, 10 * s)
            ctx.lineTo(10 * s, 12 * s)
            ctx.stroke()
        } else if (icon.kind === "menu") {
            for (const y of [4.5, 8, 11.5]) {
                ctx.beginPath()
                ctx.moveTo(3 * s, y * s)
                ctx.lineTo(13 * s, y * s)
                ctx.stroke()
            }
        } else if (icon.kind === "close") {
            // Inset further than the straight-stroked marks: diagonals read heavier than bars of the same span.
            ctx.beginPath()
            ctx.moveTo(4.5 * s, 4.5 * s)
            ctx.lineTo(11.5 * s, 11.5 * s)
            ctx.moveTo(11.5 * s, 4.5 * s)
            ctx.lineTo(4.5 * s, 11.5 * s)
            ctx.stroke()
        } else if (icon.kind === "gear") {
            // One closed path for the teeth: separate spokes leave seams the round join cannot close, which read as
            // a dotted ring. A tooth is as wide as the gap beside it.
            ctx.lineJoin = "round"
            ctx.beginPath()
            for (let tooth = 0; tooth < 8; tooth++) {
                for (let corner = 0; corner < 4; corner++) {
                    const angle = (tooth + (corner - 0.5) / 4) * Math.PI / 4
                    const radius = (corner < 2 ? 6 : 4.7) * s
                    const x = 8 * s + Math.cos(angle) * radius
                    const y = 8 * s + Math.sin(angle) * radius
                    if (tooth === 0 && corner === 0)
                        ctx.moveTo(x, y)
                    else
                        ctx.lineTo(x, y)
                }
            }
            ctx.closePath()
            ctx.stroke()
            ctx.beginPath()
            ctx.arc(8 * s, 8 * s, 2 * s, 0, 2 * Math.PI)
            ctx.stroke()
        } else if (icon.kind === "window-minimize") {
            // The window buttons' marks, on the family's grid so they sit level with the menu across the band.
            ctx.beginPath()
            ctx.moveTo(3 * s, 8 * s)
            ctx.lineTo(13 * s, 8 * s)
            ctx.stroke()
        } else if (icon.kind === "window-maximize") {
            ctx.strokeRect(3.5 * s, 3.5 * s, 9 * s, 9 * s)
        } else if (icon.kind === "window-restore") {
            // Front pane, then the outline of the one behind it, so the two read as two.
            ctx.strokeRect(3.5 * s, 5.5 * s, 7 * s, 7 * s)
            ctx.beginPath()
            ctx.moveTo(5.5 * s, 5.5 * s)
            ctx.lineTo(5.5 * s, 3.5 * s)
            ctx.lineTo(12.5 * s, 3.5 * s)
            ctx.lineTo(12.5 * s, 10.5 * s)
            ctx.lineTo(10.5 * s, 10.5 * s)
            ctx.stroke()
        } else if (icon.kind === "app-window") {
            // This program (settings' `Application`, 規約 §設定の画面): landscape and banded, unlike maximize's bare
            // square.
            ctx.strokeRect(2.5 * s, 3.5 * s, 11 * s, 9 * s)
            // Cut square: a round cap would poke out past the frame's stroke.
            ctx.lineCap = "butt"
            ctx.beginPath()
            ctx.moveTo(2.5 * s, 6.5 * s)
            ctx.lineTo(13.5 * s, 6.5 * s)
            ctx.stroke()
        } else if (icon.kind === "unified" || icon.kind === "split") {
            // The diff view toggle's pair (規約 §diff を 2 列で読む): a frame with one rule, across for one column and
            // down for two. Ten wide like `hier` / `list`, since toggles are measured by ink (§余白). The rule sits in
            // the middle to tell `unified` from `app-window`, and is cut square as that band is.
            ctx.strokeRect(3 * s, 4 * s, 10 * s, 8 * s)
            ctx.lineCap = "butt"
            ctx.beginPath()
            if (icon.kind === "unified") {
                ctx.moveTo(3 * s, 8 * s)
                ctx.lineTo(13 * s, 8 * s)
            } else {
                ctx.moveTo(8 * s, 4 * s)
                ctx.lineTo(8 * s, 12 * s)
            }
            ctx.stroke()
        } else if (icon.kind === "eye" || icon.kind === "eye-off") {
            // Lens and pupil. Drawn, since no family in the font chain has U+2691 / U+1F441.
            ctx.beginPath()
            ctx.moveTo(2.5 * s, 8 * s)
            ctx.quadraticCurveTo(8 * s, 2 * s, 13.5 * s, 8 * s)
            ctx.quadraticCurveTo(8 * s, 14 * s, 2.5 * s, 8 * s)
            ctx.closePath()
            ctx.stroke()
            // Half the lens, so the two still read apart once the mark is worn small on the rail.
            ctx.beginPath()
            ctx.arc(8 * s, 8 * s, 1.5 * s, 0, 2 * Math.PI)
            ctx.fill()
            // Struck through: at this size a dimmer tint alone is missed.
            if (icon.kind === "eye-off") {
                ctx.beginPath()
                ctx.moveTo(3 * s, 13 * s)
                ctx.lineTo(13 * s, 3 * s)
                ctx.stroke()
            }
        } else if (icon.kind === "no") {
            // The refusal badge worn beside the platform's own cursor (規約 §グラフ列は最も広い所のレーンまで).
            ctx.beginPath()
            ctx.arc(8 * s, 8 * s, 5.5 * s, 0, 2 * Math.PI)
            ctx.moveTo(4.11 * s, 4.11 * s)
            ctx.lineTo(11.89 * s, 11.89 * s)
            ctx.stroke()
        } else if (icon.kind === "no-entry") {
            // `no`'s ring with a flat bar standing free inside it — the road sign for "this ends here", in the
            // proportions of GitHub's octicon `no-entry` (規約 §行末の改行が無いこと). **The bar's ink stops a stroke
            // short of the ring's inner edge**: run out to the ring like `no`'s slash, its round ends land on the
            // ring's outer edge and the bar reads as poking through it.
            ctx.beginPath()
            ctx.arc(8 * s, 8 * s, 5.5 * s, 0, 2 * Math.PI)
            ctx.moveTo(5.5 * s, 8 * s)
            ctx.lineTo(10.5 * s, 8 * s)
            ctx.stroke()
        } else if (icon.kind === "search") {
            // The handle starts on the lens's circle, as `branch`'s lines do: started outside it, the two part at
            // large sizes.
            const reach = 4.2 / Math.SQRT2
            ctx.beginPath()
            ctx.arc(7 * s, 7 * s, 4.2 * s, 0, 2 * Math.PI)
            ctx.stroke()
            ctx.beginPath()
            ctx.moveTo((7 + reach) * s, (7 + reach) * s)
            ctx.lineTo(13.2 * s, 13.2 * s)
            ctx.stroke()
        }
    }
}
