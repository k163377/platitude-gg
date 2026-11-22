import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// Who gives way on the details pane's date line when the row is short (`CommitAuthorRow.parentRoom` / `textRoom`):
// the parent line beside it first, down to one link at three digits and the count (`←9dc… +12`), then the credit's
// name is cut, then folded to the face and its count, then the date gives up its time — a unit at a time, cut back to
// its hour, then its day — and only a day that would not stand beside that one link takes the line to its arrow and
// mark (`←…`), beside which the date stands at its day at most. Whatever gives, the line's words stay clear of the
// parent line beside them. The row is set here by width alone; the pane's floor is the app's (`details-align`).
//
// A control, to hand the row the window's font as `Main` does: the count and the commas are priced in the face their
// labels draw in.
Control {
    id: root
    width: 800
    height: 100
    font.family: Theme.uiFamily
    font.pixelSize: Theme.fontMd

    readonly property QtObject commit: QtObject {
        id: commit
        property string authorName: "Alexandria Montgomery-Vanderbilt"
        property string authorEmail: "demo@example.com"
        property int authorTime: 1790751420
        property int avatar: 3
        property string avatarUrl: ""
        property string sha8: "3c1b3390"
        property string shaHex: "3c1b3390aaaabbbbccccddddeeeeffff00001111"
        property var coAuthors: []
        property var parentHexes: []
        property string parentHex: parentHexes.length > 0 ? parentHexes[0] : ""
    }

    CommitAuthorRow {
        id: row
        details: commit
        paneWidth: row.width
    }

    TestCase {
        name: "AuthorRowRoom"
        when: windowShown

        function init() {
            const mates = []
            for (let i = 1; i <= 12; i++)
                mates.push({ name: "Pair Programmer " + i, email: "pair" + i + "@example.com", face: i })
            commit.coAuthors = mates
            const parents = []
            for (let i = 0; i < 13; i++)
                parents.push(i + "a1b2c3d4aaaabbbbccccddddeeeeffff000044" + (10 + i))
            commit.parentHexes = parents
        }

        /// The row handed `width` and laid out to it: the date and the credit stand at the rooms they were given, and
        /// the own hash at its digits' width — a layout pass after the digits change, the name's line follows a pass
        /// later. All three, since any can hold still across a step while another moves.
        function settleAt(width) {
            row.width = width
            tryVerify(() => {
                const hash = row.fieldFor("hash")
                const wants = row.plate.shaDigits >= row.plate.wholeDigits ? hash.implicitWidth
                                                                           : row.plate.shaWidth(row.plate.shaDigits)
                return Math.abs(row.dateWidth - row.dateRoom) < 0.5
                    && Math.abs(row.creditWidth - row.creditRoom) < 0.5
                    && Math.abs(hash.width - wants) < 0.5
                    && Math.abs(row.plate.firstLineWidth
                                - Math.ceil(row.plate.firstLineWidthAt(row.plate.shaDigits))) < 1
            })
        }
        function apart() {
            return row.baselines().apart
        }
        /// Narrows the row step by step until `reached` holds, checking at every step that nothing met.
        function narrowUntil(reached) {
            for (let width = root.width; width > 100; width -= 4) {
                settleAt(width)
                verify(apart() >= Theme.spaceXs, "the line's words ran into the arrow at " + width + ": " + apart())
                if (reached())
                    return width
            }
            return -1
        }

        // Room enough: the credit named, the date whole.
        function test_a_a_wide_row_cuts_nothing() {
            settleAt(root.width)
            verify(!row.matesFolded)
            verify(!row.dateClipped)
            verify(apart() >= Theme.spaceXs)
        }

        // Shorter, the credit's name folds away before the date loses a character; the count stays.
        function test_b_the_credit_folds_before_the_date_is_cut() {
            verify(narrowUntil(() => row.matesFolded) > 0, "the credit folded somewhere on the way down")
            verify(!row.dateClipped, "and the date was still whole when it did")
            compare(row.matesSaid, "+11", "the count is the same folded")
        }

        // Shorter still, the date gives up its time, the credit folded and the parents down to one link — not to their
        // mark, which waits for the day.
        function test_c_past_the_fold_the_date_is_cut_and_nothing_meets() {
            verify(narrowUntil(() => row.dateClipped) > 0, "the date gave way at last")
            verify(row.matesFolded)
            compare(row.plate.parentsShown, 1, "one link still stood when the date gave up its time")
            compare(row.plate.parentDigits, row.plate.cutDigits)
            compare(row.plate.moreSaid, "+12")
        }

        // The parents give way before the credit does, down to one link: by the time the credit's name is cut, the line
        // is that link at three digits and the count.
        function test_b2_the_parents_go_to_one_link_before_the_credit_is_cut() {
            verify(narrowUntil(() => row.matesClipped) > 0, "the credit was cut somewhere on the way down")
            compare(row.plate.parentsShown, 1, "and the parents were down to one link by then")
            compare(row.plate.parentDigits, row.plate.cutDigits)
        }

        // The line goes to its mark only under the day: a step wider, the one link stood beside the date at its day.
        function test_c2_the_parents_go_to_their_mark_only_once_the_day_would_not_stand() {
            const width = narrowUntil(() => row.plate.parentsShown === 0)
            verify(width > 0, "the parents went to their mark somewhere on the way down")
            compare(row.plate.moreSaid, "…")
            verify(row.textAcross - row.plate.keptLineWidth < row.dateFloorNeed,
                   "the day would not have stood beside the one link at " + width)
            verify(row.dateSaid === "day" || row.dateSaid === "cut",
                   "the time stayed given up beside the mark: the date came out " + row.dateSaid)
            settleAt(width + 4)
            compare(row.plate.parentsShown, 1, "a step wider the one link stood")
            compare(row.dateSaid, "day", "beside the date at its day")
        }

        // The time goes a unit at a time: from whole the date steps to its hour and the mark, then to its day and the
        // mark, and stands at each for as long as the line can spare that much — never at a width that leaves a figure
        // of a unit behind. How many widths each holds comes from the face (the units' own widths), so the walk ends
        // where the room does.
        function test_d_the_date_gives_up_its_time_a_unit_at_a_time() {
            settleAt(root.width)
            const whole = row.dateRoom
            const first = narrowUntil(() => row.dateClipped)
            verify(first > 0)
            verify(row.dateDayRoom < row.dateHourRoom && row.dateHourRoom < whole,
                   "the day, the hour and the stamp, each with the mark, widen in turn")
            const stood = { "hour": 0, "day": 0 }
            for (let width = first; width > 100; width -= 2) {
                settleAt(width)
                // Past here the line goes to its mark, the date held at its day beside it.
                if (row.dateSpare < row.dateDayRoom || row.plate.parentsShown === 0)
                    break
                const unit = row.dateSpare >= row.dateHourRoom ? "hour" : "day"
                compare(row.dateSaid, unit, "at " + width)
                // Asked of the field, not of the room that set its width: the mark stands where the unit's last
                // figure ends, which a unit measured in another face than the field's would miss.
                const date = row.fieldFor("date")
                const room = unit === "hour" ? row.dateHourRoom : row.dateDayRoom
                verify(Math.abs(room - date.markWidth - date.markAt) < 1,
                       "at " + width + " the mark stands at " + date.markAt + ", the " + unit + " was measured "
                       + (room - date.markWidth) + " wide")
                verify(row.matesFolded, "the credit stays folded beside a date short of its time, at " + width)
                stood[unit]++
            }
            // The hour's span is the face's to say (the units' own widths): the sweep below is where the hour is seen
            // to stand.
            verify(stood.day > 1, "the day stood across more than one width")
        }

        // The date's tip asks only for a date drawn short of its stamp, and says the whole stamp: over a whole date it
        // would say what is already there. Hover reaches the handler in the move that brings the pointer.
        function test_g_the_date_s_tip_asks_only_for_a_short_date() {
            settleAt(root.width)
            const date = row.fieldFor("date")
            verify(!row.dateClipped)
            let at = date.mapToItem(root, date.width / 2, date.height / 2)
            mouseMove(root, at.x, at.y)
            compare(row.dateTip.asked, false, "a whole date asks for nothing")
            verify(narrowUntil(() => row.dateClipped) > 0, "the date gave way somewhere on the way down")
            at = date.mapToItem(root, date.width / 2, date.height / 2)
            mouseMove(root, at.x, at.y)
            compare(row.dateTip.asked, true, "a short date asks for its tip")
            compare(row.dateTip.words, date.text, "with the whole stamp")
            mouseMove(root, 1, 1)
            compare(row.dateTip.asked, false, "off the date, the ask is gone")
        }

        // A long name takes the own hash's room before it is cut (user decision): the hash gives up a digit at a time
        // down to three, and only then is the name cut. Wherever the hash stands short of whole, the name's line has
        // less room left than one more digit would take.
        function test_f_the_hash_gives_way_to_a_long_name() {
            settleAt(root.width)
            compare(row.plate.shaDigits, row.plate.wholeDigits, "room enough: the hash whole")
            verify(narrowUntil(() => row.nameClipped) > 0, "the name was cut somewhere on the way down")
            compare(row.plate.shaDigits, row.plate.cutDigits, "the hash was down to three digits by then")
            tryVerify(() => row.fieldFor("hash").clipped, undefined, "and is drawn cut")
            for (let width = root.width; width >= 160; width -= 2) {
                settleAt(width)
                const digits = row.plate.shaDigits
                if (digits < row.plate.wholeDigits)
                    verify(row.textAcross - row.nameNeed < row.plate.firstLineWidthAt(digits + 1),
                           "at " + width + " the hash stood at " + digits + " beside room for one more digit")
            }
        }

        // Never short beside room it could take (user decision): wherever the date gives up a unit, or is cut under its
        // day, the line has less room left than that unit — its minutes, its hour, the rest of its day — but for the
        // time beside the parents' mark, which stays given up. Swept across
        // one parent, two and an octopus, with and without a credit, at every other pixel down to the pane's least.
        function test_e_the_date_is_never_short_beside_room_it_could_take() {
            const configs = []
            for (const parents of [1, 2, 13]) {
                for (const mates of [0, 12])
                    configs.push({ "parents": parents, "mates": mates })
            }
            let hourStood = false
            for (const config of configs) {
                const mates = []
                for (let i = 1; i <= config.mates; i++)
                    mates.push({ name: "Pair Programmer " + i, email: "pair" + i + "@example.com", face: i })
                commit.coAuthors = mates
                const parents = []
                for (let i = 0; i < config.parents; i++)
                    parents.push(i + "a1b2c3d4aaaabbbbccccddddeeeeffff000044" + (10 + i))
                commit.parentHexes = parents
                for (let width = 400; width >= 160; width -= 2) {
                    row.width = width
                    // A row with nobody credited lays no credit out: its hidden line keeps whatever width it had.
                    tryVerify(() => Math.abs(row.dateWidth - row.dateRoom) < 0.5
                                    && (config.mates === 0 || Math.abs(row.creditWidth - row.creditRoom) < 0.5))
                    const credit = config.mates > 0 ? Theme.spaceXs + row.creditWidth : 0
                    const left = row.textRoom - row.dateWidth - credit
                    const said = row.dateSaid
                    const date = row.fieldFor("date")
                    const next = said === "hour" ? date.implicitWidth - row.dateHourRoom
                               : said === "day" && row.parentsMarked ? Number.POSITIVE_INFINITY
                               : said === "day" ? row.dateHourRoom - row.dateDayRoom
                               : said === "cut" ? row.dateDayRoom - row.dateWidth : Number.POSITIVE_INFINITY
                    verify(left < next, JSON.stringify(config) + " at " + width + ": the date came out " + said
                           + " with " + left + "px left on its line, and the next unit takes " + next)
                    // The parents beside a date line that gave way: one link at three digits, or the mark — and the
                    // mark only where the day would not stand beside that link.
                    const plate = row.plate
                    if (said !== "whole" || row.matesClipped)
                        verify(plate.parentsShown === 0
                               || (plate.parentsShown === 1 && plate.parentDigits === plate.cutDigits),
                               JSON.stringify(config) + " at " + width + ": " + plate.parentsShown + " link(s) at "
                               + plate.parentDigits + " beside a date line that gave way")
                    if (plate.parentsShown === 0) {
                        verify(row.textAcross - plate.keptLineWidth < row.dateFloorNeed,
                               JSON.stringify(config) + " at " + width + ": the mark beside room for the day")
                        verify(said === "day" || said === "cut",
                               JSON.stringify(config) + " at " + width + ": the time came back beside the mark")
                    }
                    if (said === "cut")
                        compare(plate.parentsShown, 0, JSON.stringify(config) + " at " + width
                                + ": a link beside a date cut under its day")
                    hourStood = hourStood || said === "hour"
                }
            }
            verify(hourStood, "the date stood at its hour somewhere on the way down")
        }
    }
}
