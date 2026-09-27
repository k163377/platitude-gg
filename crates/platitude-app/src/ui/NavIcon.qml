import QtQuick
import platitude.ui

// Hand-drawn 16px-grid icons, scaled by the item size, in one stroke colour; `home` alone is filled.
InkCanvas {
    id: icon
    property string kind: "branch"
    property color tint: Theme.textSecondary
    /// Line weight. The coordinates scale with the size and this does not (デザイン規約 §進行中・長押しの定数「座標だけ縮めて
    /// 線を縮めない」). Beside a word at `iconSm` it would outweigh the letters by 4/3, so those callers pass the grid
    /// ratio in, the way `HoldIcon` scales.
    property real stroke: Metrics.iconStroke
    /// How much of the 16-grid this mark's ink spans sideways, for callers that seat it by its ink (デザイン規約 §余白).
    /// It differs per kind, so no one number can be subtracted for all. Kinds not listed fill their box.
    readonly property real inkGrid: {
        switch (icon.kind) {
        case "bang": return 2
        case "arrow": return 8
        // The crown, 4 to 12.
        case "tree": return 8
        // The body, 3.5 to 12.5; the shackle stands inside it.
        case "lock": return 9
        // The house, 2.5 to 13.5 — filled, which at `iconSm` matches the padlock's 9 plus its stroke. The seat caps
        // it at 11. Being filled, `inkWidth` overcounts it by a stroke; no caller seats it by its ink.
        case "home": return 11
        case "plus":
        case "minus":
        case "copyicon": return 9
        // Diagonals across 4.5..11.5; a tab seats its `✕` off this.
        case "close": return 7
        // The tick, 3.5 to 12.5.
        case "check": return 9
        // Turned 45°, so this is the diagonal of body and point.
        case "pen": return 10
        // Chevron and cursor, 3 to 13.
        case "terminal": return 10
        // The rings' outer edges, 2.8 to 13.2.
        case "pr": return 10.4
        // The lobes, 1.7 to 14.6 — off centre, hence `inkRightGrid` too.
        case "remote": return 12.9
        // The rings' outer edges, 3.2 to 12.8.
        case "branch": return 9.6
        // 6 to 10 across. Every caller turns it a quarter, so its turned width is `inkTallGrid`.
        case "chevron": return 4
        // The 7.2 square turned 45°: its diagonal.
        case "tag": return 10.18
        // Ring and bar, 2.5 to 13.5.
        case "no-entry": return 11
        // The frame, 2.5 to 13.5.
        case "app-window": return 11
        // The frame, 3 to 13.
        case "unified":
        case "split": return 10
        // Lens edge to handle end, 2.8 to 13.2 — the same both ways.
        case "search": return 10.4
        default: return 16
        }
    }
    /// The same span down the other axis, for a turned mark (`rotation`) and for `OpsPicker.markMiddle`. Only kinds
    /// one of those asks about have an entry.
    readonly property real inkTallGrid: {
        switch (icon.kind) {
        // The barbs, 5.2 to 10.8.
        case "arrow": return 5.6
        // 4 to 12 down.
        case "chevron": return 8
        // The rings, 1.7 to 14.3.
        case "branch": return 12.6
        // The dome's top to the flat base, 3.1 to 12.9.
        case "remote": return 9.8
        default: return icon.inkGrid
        }
    }
    /// Where that ink stands in the grid — its right edge here, its top in `inkTopGrid` — for a mark set against a
    /// corner (`SignatureMark`).
    readonly property real inkRightGrid: {
        switch (icon.kind) {
        // The tick's high end; `bang`'s dot, 8 plus its radius.
        case "check": return 12.5
        case "bang": return 9
        // Square in its box: `inkGrid`'s far end.
        case "close": return 11.5
        // The far ring and the far lobe: both stand in a chip's badge slot, set against its right.
        case "pr": return 13.2
        case "remote": return 14.6
        // The cursor's far end.
        case "terminal": return 13
        default: return 16
        }
    }
    readonly property real inkTopGrid: {
        switch (icon.kind) {
        case "check": return 4.5
        case "bang": return 3.5
        // The long stroke's upper end.
        case "grip": return 6
        default: return 0
        }
    }
    /// The ink's top edge in the item's pixels, half a stroke included. Of the unturned mark.
    readonly property real inkTop: icon.inkTopGrid / 16 * icon.height - icon.stroke / 2
    /// The ink span in the item's pixels, stroke included. A quarter turn reads the other axis.
    readonly property real inkWidth:
        (icon.rotation % 180 === 0 ? icon.inkGrid : icon.inkTallGrid)
        / 16 * icon.width + icon.stroke
    /// The ink's right edge in the item's pixels, half a stroke included: `width - inkRight` is the air the box
    /// holds on that side (デザイン規約 §余白). Of the unturned mark.
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
            ctx.beginPath()
            ctx.arc(8 * s, 6.5 * s, 4 * s, 0, 2 * Math.PI)
            ctx.stroke()
            ctx.beginPath()
            ctx.moveTo(8 * s, 10.5 * s)
            ctx.lineTo(8 * s, 14 * s)
            ctx.stroke()
        } else if (icon.kind === "pen") {
            ctx.save()
            // Recentred before the turn: turned 45°, the body's far corner lands at 5.23 and the tip at 3.89, so the
            // ink sits 0.67 of a grid high, plus a quarter-line for the stroked (not filled) body end.
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
            // One of `chevrons`' pair, for "opens a list" — the pair moves a pane's edge, and one mark cannot mean
            // two things. Same grid and stroke as the `spinner` ring `AppCombo` swaps it with.
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
            // `no`'s ring with the bar laid flat — the road sign for "this ends here" (規約 §行末の改行が無いこと).
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
