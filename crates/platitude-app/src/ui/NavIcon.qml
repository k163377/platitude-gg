import QtQuick
import platitude.ui

// Hand-drawn 16px-grid icons in the common git-client style (branch fork, cloud remote, price-tag, archive box, tree,
// clock). Scaled by the item size; single stroke color.
InkCanvas {
    id: icon
    property string kind: "branch"
    property color tint: Theme.textSecondary
    /// Line weight. The coordinates scale with the size and this does not (デザイン規約 §進行中・長押しの定数 —「座標だけ縮めて
    /// 線を縮めない」): a mark keeps its weight wherever it stands, so a row's icon and a badge read as the same hand.
    /// A mark that stands at `iconSm` **beside a word** is the one place that does not hold — the grid is 3/4 there
    /// while the line stays whole, so it carries 4/3 the weight of the letters next to it, and weight is what the eye
    /// reads as size. Those callers pass the grid ratio in, the way `HoldIcon` already scales.
    property real stroke: Metrics.iconStroke
    /// How much of the 16-grid this mark's ink actually spans sideways. A seat drawn to the box leaves the rest as air,
    /// and beside a word that air reads as a gap nobody wrote (デザイン規約 §余白). It is not a constant across the family:
    /// `plus` fills nine of the sixteen, `bang` is a stem and a dot and fills two, so a caller that puts marks of
    /// different kinds in one row cannot subtract one number from all of them. Kinds not listed fill their box.
    readonly property real inkGrid: {
        switch (icon.kind) {
        case "bang": return 2
        case "arrow": return 8
        // A head on a stem: the head is the whole of its width, and half the box.
        case "tree": return 8
        case "plus":
        case "minus":
        case "copyicon": return 9
        // Two diagonals corner to corner of 4.5..11.5 — inset further than the straight-stroked marks are, because
        // diagonals read heavier (`onPaint`). A tab seats its `✕` off this (デザイン規約 §余白).
        case "close": return 7
        // The tick runs 3.5 to 12.5, so it no more fills its box than `close` does. Said out loud because the seat
        // beside a name measures the gap to this ink, and the kind that shares that seat spans two (`SignatureMark`).
        case "check": return 9
        // The body and the point, turned 45° about the middle, so what they span sideways is their diagonal.
        case "pen": return 10
        // The prompt's chevron and the cursor under it, 3.0 to 13.0 with a gap between them.
        case "terminal": return 10
        // The two commit rings, each 1.7 either side of its own centre (4.5 and 11.5); the elbow stands inside them.
        case "pr": return 10.4
        // The three lobes as drawn: the left one reaches 2.0 and the right one 14.72. **Not centred in its box** —
        // which is why this kind answers `inkRightGrid` as well.
        case "remote": return 12.72
        // The two rings, each 1.8 either side of its own centre (5 and 11): 3.2 to 12.8, stem and curve inside that.
        case "branch": return 9.6
        // A square of 7.2 turned an eighth of a turn about the middle, so it spans its own diagonal.
        case "tag": return 10.18
        // Ring and bar are the same diameter, 2.5 to 13.5. Asked for because this one stands at the end of a line of
        // code and the gap to the last character is measured to the ink (§余白).
        case "no-entry": return 11
        default: return 16
        }
    }
    /// The same span measured across the mark's other axis, for callers that stand it upright (`rotation`). Only kinds
    /// that are actually turned need an entry — the rest answer with their sideways figure.
    readonly property real inkTallGrid: {
        switch (icon.kind) {
        // Stood on end, what spans sideways is the pair of barbs (5.2..10.8 of the grid), not the shaft.
        case "arrow": return 5.6
        default: return icon.inkGrid
        }
    }
    /// Where that ink stands in the grid, rather than how much of it there is: the far edge going right, and the near
    /// edge going down. A mark centred in a row never needs these; one set against a corner does (`SignatureMark`).
    readonly property real inkRightGrid: {
        switch (icon.kind) {
        // The tick's high end, and the dot the stem hangs over (8 ± its own radius).
        case "check": return 12.5
        case "bang": return 9
        // Square in its box, so this is `inkGrid`'s far end — the tab that seats it asks this rather than the span.
        case "close": return 11.5
        // The far ring and the far lobe: the badge a chip wears is set against the frame's right, so both kinds that
        // stand in that slot answer here.
        case "pr": return 13.2
        case "remote": return 14.72
        // The cursor's far end; the chevron behind it stops well short.
        case "terminal": return 13
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
    /// Where that ink's right edge falls in the item's own pixels, with the half line the cap hangs over it — so
    /// `width - inkRight` is the air the box holds on that side, which a caller seating this mark against something
    /// on its right takes off (デザイン規約 §余白). Asked of the unturned mark, the way `inkRightGrid` is.
    readonly property real inkRight: icon.inkRightGrid / 16 * icon.width + icon.stroke / 2
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
            // Three lobes, drawn to the same share of the grid its siblings take — the tag stands 10.2 of the sixteen
            // and the archive box 9. A cloud any smaller reads beside a name as a mark somebody had shrunk.
            ctx.beginPath()
            ctx.arc(5.6 * s, 9.2 * s, 3.6 * s, Math.PI * 0.5, Math.PI * 1.5)
            ctx.arc(8.6 * s, 6.56 * s, 3.84 * s, Math.PI * 0.95, Math.PI * 0.02, false)
            ctx.arc(11.6 * s, 9.68 * s, 3.12 * s, Math.PI * 1.55, Math.PI * 0.5)
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
        } else if (icon.kind === "lock") {
            // A padlock, shut. Drawn rather than borrowed from a symbol font for the reason `eye` is — U+1F512 is in
            // none of the families the chain names. **The shackle keeps its legs**: an arc that lands straight on the
            // body's top edge merges with it at the size a row wears this, and reads as a box with a lid.
            ctx.beginPath()
            ctx.moveTo(5.6 * s, 8.2 * s)
            ctx.lineTo(5.6 * s, 6.8 * s)
            ctx.arc(8 * s, 6.8 * s, 2.4 * s, Math.PI, 2 * Math.PI)
            ctx.lineTo(10.4 * s, 8.2 * s)
            ctx.stroke()
            ctx.strokeRect(3.5 * s, 8.2 * s, 9 * s, 5.3 * s)
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
            // and a mark cannot mean two things in one window. Same geometry and stroke as the ring it swaps with.
            ctx.lineJoin = "round"
            ctx.beginPath()
            ctx.moveTo(6 * s, 4 * s)
            ctx.lineTo(10 * s, 8 * s)
            ctx.lineTo(6 * s, 12 * s)
            ctx.stroke()
        } else if (icon.kind === "chevrons") {
            // The pair that moves a pane's edge, drawn rather than typed: a text guillemet sits on the lowercase band,
            // so centring its line box leaves the ink low in the button.
            ctx.lineJoin = "round"
            for (const x of [4, 9]) {
                ctx.beginPath()
                ctx.moveTo(x * s, 4 * s)
                ctx.lineTo((x + 4) * s, 8 * s)
                ctx.lineTo(x * s, 12 * s)
                ctx.stroke()
            }
        } else if (icon.kind === "terminal") {
            // The shell prompt the command log is read at, drawn rather than typed: in the mono family it sat in a
            // line's own box, low and a third the weight of the marks it stands among. The chevron is `chevron`'s own
            // geometry moved left to make room for the cursor, so the two read as one hand at the same seat.
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
            // The corner a box is pulled by, drawn as the two rules every browser puts there. The pair of chevrons one
            // mark over also moves an edge, but by being pressed rather than dragged. Inset like `close`, since
            // diagonals read heavier than bars of the same span, and hung off the lower-right corner.
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
            // The settings mark, stood beside the screen's own title. **One closed path for the teeth**, walked as
            // eight of them with four corners each — a rim drawn as separate spokes leaves eight seams the round join
            // cannot close, and at `iconMd` those read as a dotted ring. The corners sit an eighth of a turn apart,
            // half of that off the tooth's own centre, so a tooth is as wide as the gap beside it.
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
            // The three window marks are the platform's own shapes, drawn here because the band that carries them is
            // ours now. Kept on the same grid as every other mark, so they sit level with the menu at the other end.
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
            // borrowed from a symbol font: U+2691 and U+1F441 are in none of the families the chain names.
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
            // Struck through when it is not being looked at: two steps down the ramp is a state a reader can miss at
            // this size, and the stroke through it is what every layer list has meant by hidden.
            if (icon.kind === "eye-off") {
                ctx.beginPath()
                ctx.moveTo(3 * s, 13 * s)
                ctx.lineTo(13 * s, 3 * s)
                ctx.stroke()
            }
        } else if (icon.kind === "no") {
            // Worn beside a cursor, not in a row: the barred circle that says the thing under the hand will not take
            // the gesture (規約 §グラフ列は最も広い所のレーンまで). Only the badge — the shape it hangs off is the platform's
            // own cursor, left where it is.
            ctx.beginPath()
            ctx.arc(8 * s, 8 * s, 5.5 * s, 0, 2 * Math.PI)
            ctx.moveTo(4.11 * s, 4.11 * s)
            ctx.lineTo(11.89 * s, 11.89 * s)
            ctx.stroke()
        } else if (icon.kind === "no-entry") {
            // The same ring `no` wears with the bar laid flat instead of struck across: the road sign, which is what
            // says "this ends here" rather than "your hand will not be taken" (規約 §行末の改行が無いこと). Both are one
            // diameter of the same circle, so the two read as the same hand at different angles.
            ctx.beginPath()
            ctx.arc(8 * s, 8 * s, 5.5 * s, 0, 2 * Math.PI)
            ctx.moveTo(2.5 * s, 8 * s)
            ctx.lineTo(13.5 * s, 8 * s)
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
