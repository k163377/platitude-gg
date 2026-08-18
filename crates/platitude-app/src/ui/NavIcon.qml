import QtQuick
import platitude.ui

// Hand-drawn 16px-grid icons in the common git-client style (branch fork, cloud remote, price-tag, archive box, tree,
// clock). Scaled by the item size; single stroke color.
Canvas {
    id: icon
    property string kind: "branch"
    property color tint: Theme.textSecondary
    /// Line weight. The coordinates scale with the size and this does not (デザイン規約 §進行中・長押しの定数 —「座標だけ縮めて線を縮めない」), which
    /// is the drawing this family was cut for: a mark keeps its weight wherever it stands, so a row's icon and a badge
    /// read as the same hand.
    ///
    /// A mark that stands at `iconSm` **beside a word** is the one place that does not hold. The grid is 3/4 there
    /// while the line stays whole, so it carries 4/3 the weight of the letters next to it — and weight is what the eye
    /// reads as size from any distance. Those callers pass the grid ratio in, the way `HoldIcon` already scales.
    property real stroke: Metrics.iconStroke
    /// How much of the 16-grid this mark's ink actually spans sideways.
    ///
    /// A seat drawn to the box leaves the rest as air, and beside a word that air reads as a gap nobody wrote (デザイン規約
    /// §余白「印が自分で 持っている余白は、隣の詰めに数える」). It is not a constant across the family: `plus` fills nine of the sixteen, `bang`
    /// is a stem and a dot and fills two — so a caller that puts marks of different kinds in one row cannot subtract
    /// one number from all of them. Kinds not listed fill their box, which is what a seat of `width` already assumes.
    readonly property real inkGrid: {
        switch (icon.kind) {
        case "bang": return 2
        case "arrow": return 8
        case "plus":
        case "minus":
        case "copyicon": return 9
        // Two rotated pieces: the body and the point, turned 45° about the middle, so what they span sideways is their
        // diagonal.
        case "pen": return 10
        default: return 16
        }
    }
    /// The same span measured across the mark's other axis, for callers that stand it upright (`rotation`). Only kinds
    /// that are actually turned need an entry — the rest answer with their sideways figure, which is what every seat
    /// written before this already assumed.
    readonly property real inkTallGrid: {
        switch (icon.kind) {
        // Stood on end, what spans sideways is the pair of barbs (5.2..10.8 of the grid), not the shaft.
        case "arrow": return 5.6
        default: return icon.inkGrid
        }
    }
    /// Where that ink stands in the grid, rather than how much of it there is: the far edge going right, and the near
    /// edge going down. A mark centred in a row never needs these; one set against a corner does, and it is the same
    /// per-kind knowledge as `inkGrid` — the caller cannot have it (`SignatureMark`).
    readonly property real inkRightGrid: {
        switch (icon.kind) {
        // The tick's high end, and the dot the stem hangs over (8 ± its own radius).
        case "check": return 12.5
        case "bang": return 9
        default: return 16
        }
    }
    readonly property real inkTopGrid: {
        switch (icon.kind) {
        case "check": return 4.5
        case "bang": return 3.5
        default: return 0
        }
    }
    /// The same in the item's own pixels, with the line that hangs off either end of it. A quarter turn swaps which
    /// axis the caller is asking about; a half turn does not.
    readonly property real inkWidth:
        (icon.rotation % 180 === 0 ? icon.inkGrid : icon.inkTallGrid)
        / 16 * icon.width + icon.stroke
    width: Theme.iconMd
    height: Theme.iconMd
    onKindChanged: requestPaint()
    onTintChanged: requestPaint()
    onStrokeChanged: requestPaint()
    onPaint: {
        const ctx = getContext("2d")
        const s = width / 16
        ctx.clearRect(0, 0, width, height)
        ctx.strokeStyle = icon.tint
        ctx.fillStyle = icon.tint
        ctx.lineWidth = icon.stroke
        ctx.lineCap = "round"
        if (icon.kind === "branch") {
            ctx.beginPath()
            ctx.moveTo(5 * s, 5 * s)
            ctx.lineTo(5 * s, 11 * s)
            ctx.stroke()
            ctx.beginPath()
            ctx.moveTo(11 * s, 7 * s)
            ctx.bezierCurveTo(11 * s, 9.5 * s, 8 * s, 9.5 * s, 5.8 * s, 10.2 * s)
            ctx.stroke()
            for (const c of [[5, 3.5], [5, 12.5], [11, 5]]) {
                ctx.beginPath()
                ctx.arc(c[0] * s, c[1] * s, 1.8 * s, 0, 2 * Math.PI)
                ctx.stroke()
            }
        } else if (icon.kind === "remote") {
            ctx.beginPath()
            ctx.arc(6 * s, 9 * s, 3 * s, Math.PI * 0.5, Math.PI * 1.5)
            ctx.arc(8.5 * s, 6.8 * s, 3.2 * s, Math.PI * 0.95, Math.PI * 0.02, false)
            ctx.arc(11 * s, 9.4 * s, 2.6 * s, Math.PI * 1.55, Math.PI * 0.5)
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
            ctx.beginPath()
            ctx.arc(8 * s, 6.5 * s, 4 * s, 0, 2 * Math.PI)
            ctx.stroke()
            ctx.beginPath()
            ctx.moveTo(8 * s, 10.5 * s)
            ctx.lineTo(8 * s, 14 * s)
            ctx.stroke()
        } else if (icon.kind === "pen") {
            ctx.save()
            // This mark is the only one here whose ink is not centred on its own origin, and it is out by more than the
            // line: the body reaches 6 up and the point 5.5 down, and turning the pair 45° puts the body's far corner
            // at 5.23 while the tip lands at 3.89 — **0.67 of a grid above the middle**. Half a line hangs off the top
            // of the body as well (stroked, where the point is filled), which is another quarter-line. Both are put
            // back before the turn, so the shift travels along the pen's own axis.
            //
            // The old 0.25 covered the line and a little of the geometry, which held at `iconMd` beside a word — and
            // showed as soon as the mark stood at `iconSm` next to a digit of its own (2026-08-10, the graph row's
            // tallies).
            ctx.translate(8 * s, 8 * s + 0.67 * s + ctx.lineWidth / 4)
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
            // Network pair, drawn as one shape mirrored: the base line is this repository, and the arrow either lands
            // on it (fetch) or leaves it (push).
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
        } else if (icon.kind === "copyicon") {
            ctx.strokeRect(5.5 * s, 3.5 * s, 7 * s, 7 * s)
            ctx.strokeRect(3.5 * s, 5.5 * s, 7 * s, 7 * s)
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
        } else if (icon.kind === "pin") {
            // Map pin: "you are here".
            ctx.beginPath()
            ctx.arc(8 * s, 6.2 * s, 3.4 * s, Math.PI * 0.75, Math.PI * 0.25)
            ctx.lineTo(8 * s, 13.5 * s)
            ctx.closePath()
            ctx.stroke()
            ctx.beginPath()
            ctx.arc(8 * s, 6.2 * s, 1.2 * s, 0, 2 * Math.PI)
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
            // One of the pair below, for "this opens a list". The pair itself is spoken for — it moves a pane's edge —
            // and a mark cannot mean two things in one window. Same geometry and the same stroke, so it stays level
            // with the ring it swaps with in the same seat.
            ctx.lineJoin = "round"
            ctx.beginPath()
            ctx.moveTo(6 * s, 4 * s)
            ctx.lineTo(10 * s, 8 * s)
            ctx.lineTo(6 * s, 12 * s)
            ctx.stroke()
        } else if (icon.kind === "chevrons") {
            // The pair that moves a pane's edge, drawn rather than typed. A text guillemet sits on the lowercase band,
            // so centring its line box leaves the ink low in the button; and drawn, it carries the same stroke as every
            // other mark here.
            ctx.lineJoin = "round"
            for (const x of [4, 9]) {
                ctx.beginPath()
                ctx.moveTo(x * s, 4 * s)
                ctx.lineTo((x + 4) * s, 8 * s)
                ctx.lineTo(x * s, 12 * s)
                ctx.stroke()
            }
        } else if (icon.kind === "grip") {
            // The corner a box is pulled by, drawn as the two rules every browser puts there. The pair of chevrons one
            // mark over also moves an edge, but by pressing it: that one swaps a pane between two shapes, this one is
            // held and dragged, and a reader who has seen either still has to be told nothing about the other. Inset
            // like `close`, since diagonals read heavier than bars of the same span, and hung off the lower-right so
            // the ink lands on the box's own corner.
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
        } else if (icon.kind === "window-minimize") {
            // The three window marks are the platform's own shapes, drawn here because the band that carries them is
            // ours now: a rule, a square, and a square standing in front of another. Kept on the same grid as every
            // other mark so they sit level with the menu at the band's other end.
            ctx.beginPath()
            ctx.moveTo(3 * s, 8 * s)
            ctx.lineTo(13 * s, 8 * s)
            ctx.stroke()
        } else if (icon.kind === "window-maximize") {
            ctx.strokeRect(3.5 * s, 3.5 * s, 9 * s, 9 * s)
        } else if (icon.kind === "window-restore") {
            // Front pane, then the far corner of the one behind it — an outline rather than a second full square, so
            // the two do not read as a single grid at this size.
            ctx.strokeRect(3.5 * s, 5.5 * s, 7 * s, 7 * s)
            ctx.beginPath()
            ctx.moveTo(5.5 * s, 5.5 * s)
            ctx.lineTo(5.5 * s, 3.5 * s)
            ctx.lineTo(12.5 * s, 3.5 * s)
            ctx.lineTo(12.5 * s, 10.5 * s)
            ctx.lineTo(10.5 * s, 10.5 * s)
            ctx.stroke()
        } else if (icon.kind === "eye" || icon.kind === "eye-off") {
            // Lens and pupil — the visibility mark every layer list has used since Photoshop. Drawn rather than
            // borrowed from a symbol font: U+2691 and U+1F441 are in neither Segoe UI nor any of the families the chain
            // names, so a glyph would be whatever each of the three platforms falls back to.
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
            // Struck through when it is not being looked at. Two steps down the ramp is a state a reader can miss at
            // this size; the stroke through it is the one nobody misses, and it is what every layer list has meant by
            // hidden since Photoshop.
            if (icon.kind === "eye-off") {
                ctx.beginPath()
                ctx.moveTo(3 * s, 13 * s)
                ctx.lineTo(13 * s, 3 * s)
                ctx.stroke()
            }
        } else if (icon.kind === "no") {
            // Worn beside a cursor, not in a row: the barred circle that says the thing under the hand will not take
            // the gesture (規約 §グラフ列は最も広い所のレーンまで). Only the badge — the shape it hangs off is the platform's own cursor,
            // left where it is, so the refusal cannot look like a different tool from the divider one column over.
            ctx.beginPath()
            ctx.arc(8 * s, 8 * s, 5.5 * s, 0, 2 * Math.PI)
            ctx.moveTo(4.11 * s, 4.11 * s)
            ctx.lineTo(11.89 * s, 11.89 * s)
            ctx.stroke()
        } else if (icon.kind === "clock") {
            ctx.beginPath()
            ctx.arc(8 * s, 8 * s, 5.5 * s, 0, 2 * Math.PI)
            ctx.stroke()
            ctx.beginPath()
            ctx.moveTo(8 * s, 8 * s)
            ctx.lineTo(8 * s, 4.8 * s)
            ctx.moveTo(8 * s, 8 * s)
            ctx.lineTo(10.4 * s, 8 * s)
            ctx.stroke()
        }
    }
}
