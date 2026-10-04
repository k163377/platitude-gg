import QtQuick
import QtTest
import platitude.ui

// Where a cut line stands its mark against its words (`LineText.markAt` / `tailSlack`): at the edge of a character,
// whichever end is cut. Cut by the pixel, half a character shows beside the `…` and reads as another one — a stamp's
// `12` as `1` and a stroke, a path's `snapshot` as `pshot` with half a `p`. The widths between two characters are
// where that happens, so every one of them is walked.
Item {
    id: root
    width: 800
    height: 110

    LineText {
        id: stamp
        text: "2026-10-04 12:34"
        pixelSize: Theme.fontSm
    }
    LineText {
        id: authorName
        y: 30
        text: "Alexandria Montgomery-Vanderbilt"
        pixelSize: Theme.fontMd
        weight: Theme.fontWeightStrong
    }
    // Characters of two code units each, between plain ones: an edge asked for inside one answers its far side.
    LineText {
        id: subject
        y: 85
        text: "fix: the 🐛🐛 in the parser 🐛 again"
        pixelSize: Theme.fontMd
    }
    LineText {
        id: path
        y: 60
        text: "crates/platitude-core/src/session/integration/support/fixtures/refs_join_snapshot.rs"
        pixelSize: Theme.fontMd
        weight: Theme.fontWeightStrong
    }
    // The same words in the same face at their own width, asked where each character ends.
    TextEdit {
        id: whole
        visible: false
        font.family: Theme.uiFamily
        readOnly: true
        padding: 0
        textMargin: 0
    }

    TestCase {
        name: "LineCut"
        when: windowShown

        function edgesOf(line) {
            whole.font.pixelSize = line.pixelSize
            whole.font.weight = line.weight
            whole.text = line.text
            const edges = []
            for (let at = 0; at <= line.text.length; at++)
                edges.push(whole.positionToRectangle(at).x)
            return edges
        }

        function test_the_mark_starts_where_a_character_ends_data() {
            return [{ tag: "a stamp", line: stamp }, { tag: "a name", line: authorName },
                    { tag: "characters of two units", line: subject }]
        }
        function test_the_mark_starts_where_a_character_ends(data) {
            const line = data.line
            const edges = edgesOf(line)
            for (let width = line.implicitWidth - 1; width > line.markWidth + edges[2]; width -= 1) {
                line.width = width
                verify(line.clipped, "cut at " + width)
                const room = width - line.markWidth
                // The last edge the room still holds — under a pixel past it is the character's own air.
                let last = 0
                for (const edge of edges) {
                    if (edge < room + 1)
                        last = edge
                }
                verify(Math.abs(line.markAt - Math.min(last, room)) < 0.5,
                       "at " + width + " the mark stands at " + line.markAt + ", the last whole character ends at "
                       + last)
                // …and never past the room: the patch is at least as wide as the mark it carries.
                verify(line.markAt <= room + 0.01, "at " + width + " the mark stands " + (line.markAt - room)
                       + " past its room")
            }
        }

        /// Cut at the head or in the middle, the words are held against the far edge and the mark stands at the near
        /// one — so the character the mark's end falls in would show its back half. The words are drawn back by what
        /// is left of it, and the first one past the mark stands whole.
        function test_the_words_past_a_head_cut_start_on_a_character_data() {
            return [{ tag: "from the start", cut: "start" }, { tag: "in the middle", cut: "middle" }]
        }
        function test_the_words_past_a_head_cut_start_on_a_character(data) {
            path.cutAt = data.cut
            const edges = edgesOf(path)
            let drawnBack = 0
            for (let width = path.implicitWidth - 1; width > path.implicitWidth / 3; width -= 1) {
                path.width = width
                verify(path.clipped, "cut at " + width)
                // Where the mark ends in the words' own x, were they held hard against the far edge.
                const from = path.markWidth - (width - path.implicitWidth)
                // The first edge at or past it — or under a pixel before it.
                let next = edges[edges.length - 1]
                for (let at = edges.length - 1; at >= 0 && edges[at] > from - 1; at--)
                    next = edges[at]
                verify(Math.abs(path.tailSlack - Math.max(0, next - from)) < 0.5,
                       "at " + width + " the words are drawn back " + path.tailSlack + ", the next character starts "
                       + (next - from) + " past the mark")
                if (path.tailSlack > 0.5)
                    drawnBack++
            }
            verify(drawnBack > 0, "some width put the mark's end inside a character")
        }

        /// A holder that sizes the line for a count of whole characters keeps that count, a fraction of a pixel short
        /// or not: it rounds its sum apart from the mark's own width (`HashPlate.hashWidth` — three figures and the
        /// mark), and a face of fractional advances leaves the last figure under a pixel past the room.
        function test_a_line_sized_for_whole_characters_keeps_them() {
            const edges = edgesOf(stamp)
            for (const short of [0, 0.4, 0.9]) {
                stamp.width = edges[10] + stamp.markWidth - short
                verify(stamp.clipped, short + " short")
                verify(stamp.markAt > edges[9] + 0.5,
                       short + " short: the mark stands at " + stamp.markAt + ", the tenth character ends at "
                       + edges[10])
            }
            // A pixel and more short, the tenth does not stand.
            stamp.width = edges[10] + stamp.markWidth - 1.5
            verify(Math.abs(stamp.markAt - edges[9]) < 0.5, "the mark stands at " + stamp.markAt)
        }

        /// A line that fits wears no mark, and a width handed back takes it away again.
        function test_a_line_that_fits_is_not_cut() {
            stamp.width = stamp.implicitWidth - 20
            verify(stamp.clipped)
            stamp.width = stamp.implicitWidth
            verify(!stamp.clipped)
        }
    }
}
