import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The band over the graph (`RecoverBand`): it stands no taller than its ceiling however many parts an entry has, the
// parts' lines scroll under a heading, a date and controls that stay put, and each press stands on the middle of the
// lines it acts on.
Item {
    id: root
    width: 800
    height: 900

    RecoverBand {
        id: band
        width: root.width
        when: "2026-11-07 01:02 (Yesterday)"
    }

    TestCase {
        name: "RecoverBand"
        when: windowShown

        function entry(title, count, byPart) {
            const parts = []
            for (let i = 0; i < count; i++)
                parts.push({ mark: "branch", tint: Theme.accent, badged: false, text: (i + 1) + " commits",
                             restore: "as stack/layer-" + i + "-pgg-restored", restoreMarks: [] })
            return { key: title, title: title, titleMarks: [], worktree: "", worktreeMark: "tree", parts: parts,
                     byPart: byPart }
        }
        function show(shown, width) {
            band.width = width
            band.reach = shown.parts.map(() => true)
            band.entry = shown
        }
        /// The first item under `item` that `is` answers true for, null for none.
        function under(item, is) {
            for (const child of item.children) {
                if (is(child))
                    return child
                const found = under(child, is)
                if (found !== null)
                    return found
            }
            return null
        }
        function partsView() {
            return under(band, item => item.contentY !== undefined && item.contentHeight !== undefined)
        }
        function closeMark() {
            return under(band, item => item.markSize !== undefined)
        }
        function entryPress(word) {
            return under(band, item => item.frameColor !== undefined && item.text === word)
        }
        /// The part's own press and words, on its line.
        function partPress() {
            return under(partsView(), item => item.text === "Restore" && item.font !== undefined)
        }
        function partWords() {
            return under(partsView(), item => item.lineHeight !== undefined)
        }
        function topOf(item) {
            return item.mapToItem(band, 0, 0).y
        }
        function middleOf(item) {
            return topOf(item) + item.height / 2
        }
        /// The lines laid out past what the band holds of them.
        function overflowing() {
            return partsView().contentHeight > partsView().height + 100
        }

        function init() {
            band.entry = null
            band.reach = []
            band.canWalkFurther = false
        }

        function test_lines_that_fit_take_their_own_height() {
            show(entry("Reset main", 2, true), 800)
            tryVerify(() => partsView().contentHeight > 0 && partsView().height === partsView().contentHeight)
            verify(band.height < Theme.recoverBandMaxHeight, "under the ceiling: " + band.height)
            verify(!partsView().clip, "and nothing cut, so a press's wash keeps its whole height")
        }
        function test_many_lines_stop_at_the_ceiling_and_scroll() {
            show(entry("Rebased thirty", 30, false), 800)
            tryVerify(() => overflowing() && band.height === Theme.recoverBandMaxHeight,
                      undefined, "the band at the ceiling: " + band.height)
            verify(partsView().clip)
        }
        function test_the_heading_and_the_controls_stay_as_the_lines_scroll() {
            show(entry("Rebased thirty", 30, false), 800)
            tryVerify(() => overflowing() && band.height === Theme.recoverBandMaxHeight)
            const closeAt = topOf(closeMark())
            const pressAt = topOf(entryPress("Restore"))
            const heading = under(band, item => item.lineHeight !== undefined && item.text === "Rebased thirty")
            const headingAt = topOf(heading)
            const lineAt = topOf(partWords())
            partsView().contentY = 60
            compare(topOf(partWords()), lineAt - 60, "the lines moved")
            compare(topOf(closeMark()), closeAt)
            compare(topOf(entryPress("Restore")), pressAt)
            compare(topOf(heading), headingAt)
            compare(band.height, Theme.recoverBandMaxHeight)
        }
        function test_another_entry_starts_at_the_top_of_its_lines() {
            show(entry("Rebased thirty", 30, false), 800)
            tryVerify(() => overflowing())
            partsView().contentY = 60
            band.entry = entry("Rebased twenty", 20, false)
            compare(partsView().contentY, 0)
        }
        function test_the_same_entry_read_again_keeps_the_readers_place() {
            show(entry("Rebased thirty", 30, false), 800)
            tryVerify(() => overflowing())
            partsView().contentY = 60
            band.entry = entry("Rebased thirty", 30, false)
            compare(partsView().contentY, 60)
        }
        function test_the_bar_stands_only_while_lines_are_cut_off() {
            show(entry("Reset main", 2, true), 800)
            tryVerify(() => partsView().contentHeight > 0 && partsView().height === partsView().contentHeight)
            verify(!partsView().ScrollBar.vertical.visible)
            show(entry("Rebased thirty", 30, false), 800)
            tryVerify(() => overflowing() && partsView().ScrollBar.vertical.visible)
        }
        function test_a_wheel_moves_the_lines() {
            show(entry("Rebased thirty", 30, false), 800)
            tryVerify(() => overflowing())
            mouseWheel(partsView(), 40, 20, 0, -120)
            tryVerify(() => partsView().contentY > 0, undefined, "the lines moved")
        }
        function test_a_heading_the_cap_cuts_says_its_whole_under_the_hand() {
            const long = "Rebased " + "stack/layer-with-a-long-name, ".repeat(40)
            show(entry(long, 2, false), 800)
            const heading = under(band, item => item.lineHeight !== undefined && item.text === long)
            tryVerify(() => heading.clipped && heading.height > 0)
            mouseMove(heading, 20, 8)
            tryVerify(() => heading.tip.asked, undefined, "a hand on the heading asks for the whole of it")
            compare(heading.tip.words, long)
            mouseMove(root, root.width - 1, root.height - 1)
            tryVerify(() => !heading.tip.asked)
            show(entry("Reset main", 2, true), 800)
            const short = under(band, item => item.lineHeight !== undefined && item.text === "Reset main")
            tryVerify(() => short !== null && !short.clipped && short.height > 0)
            mouseMove(short, 20, 8)
            tryVerify(() => short.pointed)
            verify(!short.tip.asked, "a heading whole on the band asks for nothing")
            mouseMove(root, root.width - 1, root.height - 1)
        }
        function test_the_heading_stops_at_its_rows() {
            const long = "Rebased " + "stack/layer-with-a-long-name, ".repeat(40)
            show(entry(long, 2, false), 800)
            const heading = under(band, item => item.lineHeight !== undefined && item.text === long)
            tryVerify(() => heading.clipped && heading.height === Theme.recoverBandHeadRows * heading.lineHeight)
            verify(band.height <= Theme.recoverBandMaxHeight)
        }
        function test_the_not_loaded_line_stays_out_of_the_scroll() {
            show(entry("Rebased thirty", 30, false), 800)
            band.reach = band.parts.map(() => false)
            const reachWords = under(band, item => item.lineHeight !== undefined
                                                   && item.text === qsTr("Not loaded on the graph yet"))
            tryVerify(() => reachWords.visible && overflowing()
                      && topOf(reachWords) === topOf(partsView()) + partsView().height + Theme.spaceXs)
            verify(band.height <= Theme.recoverBandMaxHeight)
        }
        function test_a_parts_press_stands_on_its_words_baseline_on_one_line() {
            show(entry("Reset main", 2, true), 800)
            tryVerify(() => partWords().lineHeight > 0 && partPress() !== null && partPress().height > 0)
            compare(partWords().implicitHeight, partWords().lineHeight, "one line at this width")
            verify(Math.abs(topOf(partPress()) + partPress().baselineOffset
                            - topOf(partWords()) - partWords().baselineOffset) <= 1)
        }
        function test_a_parts_press_stands_on_the_middle_of_words_that_wrap() {
            show(entry("Reset main", 2, true), 336)
            tryVerify(() => partWords().lineHeight > 0 && partWords().implicitHeight > partWords().lineHeight
                      && partPress() !== null && partPress().height > 0)
            // Its word, not its box: a press's box hangs off its baseline.
            const wordAt = topOf(partPress()) + partPress().baselineOffset
            const linesAt = topOf(partWords()) + partWords().baselineOffset
                            + (partWords().implicitHeight - partWords().lineHeight) / 2
            verify(Math.abs(wordAt - linesAt) <= 1, wordAt + " against " + linesAt)
        }
        function test_restore_all_stands_between_the_parts_presses() {
            show(entry("Reset main", 2, true), 800)
            const press = entryPress("Restore all")
            tryVerify(() => partsView().height > 0 && partsView().height === partsView().contentHeight
                      && Math.abs(middleOf(press) - middleOf(partsView())) <= 1)
        }
    }
}
