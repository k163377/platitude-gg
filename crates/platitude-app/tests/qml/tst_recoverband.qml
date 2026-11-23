import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui
import platitude.ui.words

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
            return item.mapToItem(band, Qt.point(0, 0)).y
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
            band.room = Theme.recoverBandMaxHeight
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
        function test_a_shorter_column_takes_only_what_it_needs_off_the_lines() {
            show(entry("Rebased thirty", 30, false), 800)
            tryVerify(() => overflowing() && band.height === Theme.recoverBandMaxHeight)
            band.room = 120
            tryVerify(() => band.height === 120, undefined, "the band at the room it has: " + band.height)
            band.room = 500
            tryVerify(() => band.height === Theme.recoverBandMaxHeight, undefined, "never past its ceiling")
        }
        function test_a_column_with_no_room_leaves_one_line_of_each_kind() {
            const long = "Rebased " + "stack/layer-with-a-long-name, ".repeat(40)
            show(entry(long, 30, false), 800)
            const heading = under(band, item => item.lineHeight !== undefined && item.text === long)
            tryVerify(() => overflowing() && heading.height === Theme.recoverBandHeadRows * heading.lineHeight)
            band.room = 0
            tryVerify(() => heading.height === heading.lineHeight, undefined, "the heading down to its first line")
            tryVerify(() => partsView().height > 0 && partsView().height < 2 * partWords().lineHeight,
                      undefined, "the parts down to a line: " + partsView().height)
            verify(partsView().ScrollBar.vertical.visible, "the rest a scroll away")
            // The margins, the heading's line and the date's over the press standing on the middle of the parts' line:
            // the face's numbers, whichever face this machine has.
            const date = under(band, item => item.lineHeight !== undefined && item.text === band.when)
            const least = 2 * Theme.spaceSm + heading.height + Theme.spaceXs + date.height + Theme.spaceXs
                          + (Theme.controlHeight + date.lineHeight) / 2
            tryVerify(() => Math.abs(band.height - least) <= 1,
                      undefined, "the band at its least: " + band.height + " against " + least)
            band.room = Theme.recoverBandMaxHeight
            tryVerify(() => heading.height === Theme.recoverBandHeadRows * heading.lineHeight
                      && band.height === Theme.recoverBandMaxHeight, undefined, "all back as the room comes back")
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
        /// The line saying the press cannot be had stands under the parts, and the press on the middle of both: a
        /// column short of room still gets the band down to exactly the room it has, the parts giving up what the line
        /// takes.
        function test_a_band_with_the_not_loaded_line_comes_down_to_its_room() {
            show(entry("Rebased thirty", 30, false), 800)
            band.reach = band.parts.map(() => false)
            tryVerify(() => overflowing() && band.height === Theme.recoverBandMaxHeight && band.leastHeight > 0)
            // Just over its least, where the press stands on the middle of a parts' line shorter than itself.
            const room = band.leastHeight + 6
            band.room = room
            tryCompare(band, "height", room, undefined, "the band at the room it has")
        }
        /// At its least the band keeps one line of the parts, and a part's line sets the name it brings back as in
        /// git's face beside the words — the least is that line, not one of words alone.
        function test_the_least_keeps_a_line_of_parts_that_set_a_name() {
            const named = entry("Rebased thirty", 30, false)
            named.parts = named.parts.map(part => Object.assign({}, part, {
                restoreMarks: [[3, "branch", Theme.accent, part.restore.length - 3]] }))
            show(named, 800)
            tryVerify(() => overflowing() && partWords().lineHeight > 0)
            band.room = 0
            tryVerify(() => partsView().height > 0 && partsView().height < 2 * partWords().lineHeight)
            verify(partsView().height >= partWords().lineHeight - 0.5,
                   "a whole line of a part: " + partsView().height + " against " + partWords().lineHeight)
        }
        /// The way further stands beside the words where both fit on one line, on their baseline, and under them where
        /// they do not, its word starting where theirs do — the words never wrap beside it.
        function test_the_way_further_goes_under_words_it_would_crowd() {
            show(entry("Rebased two", 2, false), 800)
            band.step = 500
            band.canWalkFurther = true
            band.reach = band.parts.map(() => false)
            const words = under(band, item => item.lineHeight !== undefined
                                               && item.text === qsTr("Not loaded on the graph yet"))
            const press = under(band, item => item.font !== undefined && item.text === Words.loadFurther(500))
            tryVerify(() => words.visible && press.visible && press.height > 0 && words.lineHeight > 0)
            tryVerify(() => Math.abs(topOf(press) + press.baselineOffset - topOf(words) - words.baselineOffset) <= 1,
                      undefined, "beside the words, on their baseline")
            compare(words.height, words.lineHeight, "the words on one line")

            band.width = 336
            // A line's pitch under the words, as the lines over them are apart: its box would open the lines apart.
            tryVerify(() => Math.abs(topOf(press) + press.baselineOffset - topOf(words) - words.baselineOffset
                                     - words.lineHeight - Theme.spaceXs) <= 1,
                      undefined, "under the words, a line's pitch on")
            compare(words.height, words.lineHeight, "the words still on one line")
            fuzzyCompare(press.mapToItem(band, Qt.point(press.leftPadding, 0)).x,
                         words.mapToItem(band, Qt.point(0, 0)).x, 1, "its word starts where theirs do")
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

        // Coming back in place of a bar that went in the same frame (`BarTravel`): the band stands at that bar's
        // height and travels to its own, so the rows under it never jump.
        function test_the_band_takes_a_bars_place_at_its_height() {
            show(entry("Reset main", 1, false), 800)
            tryVerify(() => band.ownHeight > 0 && band.implicitHeight === band.ownHeight)
            band.visible = false
            const from = band.ownHeight - 30
            band.enterFrom = from
            band.visible = true
            compare(band.implicitHeight, from, "at the gone bar's height, in the frame it went")
            verify(band.clip, "its lines short of their room stay inside it")
            // Its own height as it stands now: shown again, its lines are laid out again.
            tryVerify(() => !band.travelling && band.implicitHeight === band.ownHeight)
            verify(!band.clip)
        }
        // A bar of its own height before it: nothing travels, and nothing is left travelling — no arrival would put it
        // down, and the next pick would animate.
        function test_the_band_taking_a_place_at_its_own_height_does_not_travel() {
            show(entry("Reset main", 1, false), 800)
            tryVerify(() => band.ownHeight > 0 && band.implicitHeight === band.ownHeight)
            band.visible = false
            band.enterFrom = band.ownHeight
            band.visible = true
            compare(band.implicitHeight, band.ownHeight)
            verify(!band.travelling)
            verify(!band.clip)
        }
        // A pick has no bar before it: the band stands at once.
        function test_the_band_with_nothing_before_it_stands_at_once() {
            show(entry("Reset main", 1, false), 800)
            band.visible = false
            band.visible = true
            compare(band.implicitHeight, band.ownHeight)
            verify(!band.travelling)
        }
        // A height handed over for a turn the band did not come back in is put down with that turn, so a later pick
        // does not travel from it.
        function test_a_height_the_band_did_not_take_is_put_down() {
            show(entry("Reset main", 1, false), 800)
            band.enterFrom = 10
            tryCompare(band, "enterFrom", 0)
            band.visible = false
            band.visible = true
            compare(band.implicitHeight, band.ownHeight)
        }
    }
}
