import QtQuick
import QtTest
import platitude.ui
import platitude.ui.face

// The cut mark (`CutMark`): drawn, not typed, in the room of the glyph it stands for — as wide as the words' face's
// own `…`, its three dots on the baseline, starting no later than that glyph's ink and inside its room (user
// decision). Each face cut words are set in, normal and strong, at a chip's size and the words', the drawn mark
// under the typed glyph from the same whole pixel.
Item {
    id: root
    width: 400
    height: 680

    Rectangle {
        anchors.fill: parent
        color: "white"
    }
    // Per face: the typed `…`, and `markLine` below it, the drawn mark on the same baseline offset.
    Repeater {
        id: rows
        model: {
            const faces = []
            for (const family of [Theme.uiFamily, Theme.monoFamily])
                for (const size of [Theme.fontChip, Theme.fontMd])
                    for (const weight of [Font.Normal, Theme.fontWeightStrong])
                        faces.push({ family: family, size: size, weight: weight })
            return faces
        }
        Item {
            id: row
            required property var modelData
            required property int index
            readonly property string face: modelData.family + " " + modelData.size + "px " + modelData.weight
            readonly property alias typed: typedGlyph
            readonly property alias mark: drawnMark
            readonly property int markLine: 40
            x: 20
            y: 20 + row.index * 80
            Text {
                id: typedGlyph
                text: "…"
                color: "black"
                font.family: row.modelData.family
                font.pixelSize: row.modelData.size
                font.weight: row.modelData.weight
            }
            CutMark {
                id: drawnMark
                y: row.markLine + typedGlyph.baselineOffset - drawnMark.baselineOffset
                color: "black"
                font: typedGlyph.font
            }
        }
    }
    CutMark {
        id: sized
        visible: false
    }
    TextMetrics {
        id: glyph
        text: "…"
    }

    TestCase {
        name: "CutMark"
        when: windowShown

        /// The columns holding ink in `[x0, x1) × [y0, y1)` of `picture`: `{ left, right, top, bottom }`, -1 for none.
        function inkBox(picture, x0, x1, y0, y1) {
            let top = -1, bottom = -1, left = -1, right = -1
            for (let y = Math.floor(y0); y < Math.ceil(y1); y++) {
                for (let x = Math.floor(x0); x < Math.ceil(x1); x++) {
                    if (picture.red(x, y) < 160) {
                        if (top < 0)
                            top = y
                        bottom = y
                        left = left < 0 ? x : Math.min(left, x)
                        right = Math.max(right, x)
                    }
                }
            }
            return { top: top, bottom: bottom, left: left, right: right }
        }

        function test_as_wide_as_the_face_s_own_glyph() {
            for (const family of [Theme.uiFamily, Theme.monoFamily]) {
                for (const size of [Theme.fontSm, Theme.fontChip, Theme.fontMd, Theme.fontCode]) {
                    for (const weight of [Font.Normal, Theme.fontWeightStrong]) {
                        const font_ = Qt.font({ family: family, pixelSize: size, weight: weight })
                        sized.font = font_
                        glyph.font = font_
                        fuzzyCompare(sized.implicitWidth, glyph.advanceWidth, 0.01,
                                     family + " at " + size + "px, weight " + weight)
                    }
                }
            }
        }

        function test_three_dots_on_the_baseline() {
            const picture = grabImage(root)
            for (let i = 0; i < rows.count; i++) {
                const row = rows.itemAt(i)
                const mark = row.mark
                const baseline = Math.round(row.y + row.markLine + row.typed.baselineOffset)
                const ink = inkBox(picture, row.x + WordFace.cutDotX(mark.ruler, 0, mark.x),
                                   row.x + WordFace.cutDotX(mark.ruler, 2, mark.x) + WordFace.cutDotSize,
                                   row.y + mark.y - 20, row.y + mark.y + 20)
                verify(ink.top >= 0, row.face + ": the dots are drawn")
                compare(ink.bottom, baseline - 1, row.face + ": the dots stand on the baseline")
                compare(ink.bottom - ink.top + 1, WordFace.cutDotSize, row.face + ": one dot tall")
                let runs = 0
                let inRun = false
                for (let x = ink.left; x <= ink.right; x++) {
                    const dark = picture.red(x, baseline - 1) < 160
                    if (dark && !inRun)
                        runs++
                    inRun = dark
                }
                compare(runs, 3, row.face + ": three dots")
            }
        }

        // The typed glyph and the drawn mark from the same whole pixel: the drawn one starts no later than the glyph's
        // ink — later opens air after the letter before — and ends inside the glyph's room, its ink as the face reports
        // it or its advance, the further (a mono `…` inks past its advance).
        function test_no_air_the_typed_glyph_would_not_leave() {
            const picture = grabImage(root)
            for (let i = 0; i < rows.count; i++) {
                const row = rows.itemAt(i)
                const left = row.x - 4
                const reach = row.x + row.mark.width + 8
                const typed = inkBox(picture, left, reach, row.y, row.y + row.typed.height)
                const drawn = inkBox(picture, left, reach, row.y + row.markLine,
                                     row.y + row.markLine + row.typed.height)
                verify(typed.left >= 0 && drawn.left >= 0, row.face + ": both are drawn")
                verify(drawn.left <= typed.left,
                       row.face + ": the mark starts at " + drawn.left + ", after the glyph's " + typed.left)
                const ink = row.mark.ruler.tightBoundingRect
                const room = Math.max(ink.x + ink.width, row.mark.implicitWidth)
                verify(drawn.right < row.x + Math.ceil(room),
                       row.face + ": the mark ends at " + drawn.right + ", past the glyph's room " + (row.x + room))
            }
        }
    }
}
