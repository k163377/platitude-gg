import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// A button's phrase short of room (`ActionButtonLabel.giving` / `given`): `+N`, then the words, then the ref past half
// the phrase, then the command's chip, then `by <face>`, and the ref last — each only once every part before it is at
// its least — and the phrase drawn inside the label down to that least. The commit button's phrase, set by hand; where
// its room comes from is the right pane's (`window-fill`).
//
// A control, to hand the label the window's font as `Main` does: the parts are priced in the faces they draw in.
Control {
    id: root
    width: 800
    height: 80
    font.family: Theme.uiFamily
    font.pixelSize: Theme.fontMd

    ActionButtonLabel {
        id: label
        height: 40
        phraseHead: "commit"
        phraseCount: "120"
        text: qsTr("files to")
        phraseTail: "feature/give-way-in-a-narrow-pane"
        phraseTailMark: "branch"
    }

    TestCase {
        name: "PhraseGive"
        when: windowShown

        /// The phrase's row, which the label builds once it has a head.
        function phrase() {
            for (let i = 0; i < label.children.length; i++) {
                const item = label.children[i].item
                if (item && item.giving !== undefined)
                    return item
            }
            return null
        }
        /// The phrase with every part that gives at its least, beside the parts that never give.
        function least(row) {
            return row.phraseFixed + row.giving.reduce((sum, part) => sum + part.least, 0)
        }
        function at(row, i) {
            return row.given[i]
        }
        function whole(row, i) {
            return row.giving[i].whole
        }
        function floor(row, i) {
            return row.giving[i].least
        }

        /// The phrase's drawn parts, found by what they draw rather than by `giving`'s order: the command's chip, the
        /// words, the ref's chip, the face, and the cut marks standing in the row itself.
        function drawn(row) {
            const parts = { "head": null, "words": null, "tail": null, "face": null, "mates": false, "marks": [] }
            for (let i = 0; i < row.children.length; i++) {
                const child = row.children[i]
                if (child.word !== undefined && child.word === label.phraseHead)
                    parts.head = child
                else if (child.word !== undefined && child.word === label.phraseTail)
                    parts.tail = child
                else if (child.face !== undefined)
                    parts.face = child
                else if (child.cutting !== undefined && child.text === label.text)
                    parts.words = child
                else if (child.visible && child.text === "+" + label.phraseMates)
                    parts.mates = true
                else if (child.visible && child.base !== undefined)
                    parts.marks.push(child)
            }
            return parts
        }
        /// The parts in the order they are to give, each as whether the phrase holds it and whether it has begun to give
        /// way on screen: `+N` gone, the words cut, the ref cut past its share, the command's chip cut (`CodeChip.cut`),
        /// the face gone, the ref under its share — read off its cap (`tailCap`), not its drawn width, since a chip
        /// draws up to a glyph short of its cap.
        function gaveOnScreen(row) {
            const parts = drawn(row)
            const want = parts.tail.wantWidth
            return [{ "holds": label.phraseMates > 0, "gave": !parts.mates },
                    { "holds": label.text !== "", "gave": !parts.words.visible || parts.words.cutting },
                    { "holds": want > row.tailShare, "gave": parts.tail.cut },
                    { "holds": true, "gave": parts.head.cut },
                    { "holds": row.faced, "gave": !parts.face.visible },
                    { "holds": true, "gave": row.tailCap < Math.min(want, row.tailShare) - 0.5 }]
        }

        /// Sweeps the label from wide down to the phrase's least, checking at every other pixel that a part gives only
        /// once those before it are at their least, and that what is drawn stays inside the label.
        function sweep(what) {
            label.width = root.width
            tryVerify(() => phrase() !== null)
            const row = phrase()
            const end = Math.ceil(least(row)) + 1
            for (let width = root.width; width >= end; width -= 2) {
                label.width = width
                const note = what + " at " + width
                for (let i = 1; i < row.giving.length; i++) {
                    if (at(row, i) < whole(row, i) - 0.5) {
                        for (let j = 0; j < i; j++)
                            verify(at(row, j) < floor(row, j) + 0.5,
                                   note + ": part " + i + " gave while part " + j + " stood above its least")
                    }
                }
                // The derived answers say the same order: the chip cut only past `+N`, the words and the ref's excess,
                // the face gone only past the chip's least, the ref under its share only past the face.
                if (row.headCap > 0) {
                    verify(label.phraseMates === 0 || !row.mateFits, note + ": the chip cut beside a whole `+N`")
                    verify(at(row, 2) < 0.5, note + ": the chip cut while the ref held more than its share")
                }
                if (!row.faceFits)
                    verify(Math.abs(at(row, 3) - floor(row, 3)) < 0.5, note + ": the face went before the chip's least")
                if (row.tailCap < Math.min(whole(row, 2) + whole(row, 5), row.tailShare) - 0.5)
                    verify(!row.faced || !row.faceFits, note + ": the ref went under its share beside the face")
                // Laid out a polish after the caps move.
                tryVerify(() => row.width <= label.width + 0.5, undefined,
                          note + ": the phrase ran " + (row.width - label.width) + "px past the label")
                // The same order read off what is drawn: a part begins to give only once every part before it that
                // the phrase holds has begun to. That each of those is at its least by then is the check above
                // (`at` against `floor`); the last part's start is its cap's.
                const parts = gaveOnScreen(row)
                for (let k = 1; k < parts.length; k++) {
                    for (let j = 0; parts[k].holds && parts[k].gave && j < k; j++)
                        verify(!parts[j].holds || parts[j].gave,
                               note + ": on screen, part " + k + " gave while part " + j + " stood")
                }
                if (row.faced && !drawn(row).face.visible)
                    verify(drawn(row).marks.length >= 1, note + ": the face went without a mark in its place")
            }
        }

        function test_a_with_a_face_and_names_to_count() {
            label.phraseFace = 3
            label.phraseMates = 2
            sweep("face and +2")
        }

        function test_b_with_a_face_alone() {
            label.phraseFace = 3
            label.phraseMates = 0
            sweep("face alone")
        }

        function test_c_with_no_face() {
            label.phraseFace = -1
            label.phraseMates = 0
            sweep("no face")
        }

        // `by <face>` and `+N` gone together leave one mark, so the room they leave is the ref's: the parts at their
        // least price that one mark once, not twice — and the mark is drawn.
        function test_d_the_face_and_the_count_share_one_mark() {
            label.phraseFace = 3
            label.phraseMates = 2
            label.width = root.width
            tryVerify(() => phrase() !== null)
            const row = phrase()
            compare(floor(row, 4), 0, "the face's least is the mark `+N` already holds")
            verify(floor(row, 0) > 0, "and `+N` holds it")
            label.width = Math.ceil(least(row)) + 1
            tryVerify(() => !row.faceFits && !row.mateFits)
            compare(drawn(row).marks.length, 1, "the face and `+N` gone, their mark is drawn")
        }

        /// A signature mark of `kind` drawn in the phrase: on the face, or on the shoulder of the mark the face leaves.
        function signatureStands(item, kind) {
            for (let i = 0; i < item.children.length; i++) {
                const child = item.children[i]
                if (child.kind === kind && child.visible && child.tipShown !== undefined)
                    return true
                if (signatureStands(child, kind))
                    return true
            }
            return false
        }
        /// The signature marks in the phrase asking for their tip, drawn or not.
        function marksAsking(item) {
            let asking = []
            for (let i = 0; i < item.children.length; i++) {
                const child = item.children[i]
                if (child.tipShown !== undefined && child.pointed)
                    asking.push(child)
                asking = asking.concat(marksAsking(child))
            }
            return asking
        }

        // Signing on, the signature mark stays when `by <face>` gives way: on the face while it stands, then on the
        // shoulder of the mark left in its place — and the phrase still inside the label down to its least. A run's
        // stand-in for the pointer reaches the drawn mark alone: a hidden one asking too takes the window's one tip.
        function test_e_the_signature_mark_outlives_the_face() {
            label.phraseFace = 3
            label.phraseMates = 2
            label.phraseSignature = "signed"
            sweep("signed")
            label.width = root.width
            label.phraseSignaturePointedAt = true
            const row = phrase()
            const end = Math.ceil(least(row)) + 1
            let gone = false
            for (let width = root.width; width >= end; width -= 2) {
                label.width = width
                verify(signatureStands(row, "signed"), "the signature mark stands at " + width)
                const asking = marksAsking(row)
                compare(asking.length, 1, "the marks asking for their tip at " + width)
                verify(asking[0].visible, "the mark asking for its tip is drawn at " + width)
                if (!row.faceFits) {
                    // On the shoulder, its box's top on the line box's top (デザイン規約 §アイコンの席 の ⑤), where
                    // the cut mark's box stands.
                    const mark = drawn(row).marks[0]
                    tryVerify(() => Math.abs(asking[0].mapToItem(row, 0, 0).y - mark.y) < 0.5, undefined,
                              "the shoulder mark's box stands at " + asking[0].mapToItem(row, 0, 0).y
                              + ", the line box's top at " + mark.y + ", at " + width)
                }
                gone = gone || !row.faceFits
            }
            verify(gone, "the face gave way somewhere on the way down")
            label.phraseSignaturePointedAt = false
            label.phraseSignature = ""
        }

    }
}
