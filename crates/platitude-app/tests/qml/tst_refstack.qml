import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// What the sheets behind a chip's front card come to, and what the card itself spends on the count in front of them,
// held against records spelled out whole. **Only the built stack can answer this**: the rule reads each record's
// colour through the card's own `kindKeyOf`, and what comes out is a list the item builds — no Rust test reaches
// either. The card's own arithmetic is here for the same reason: `furnitureW` is a sum of what its labels and marks
// came out to, which nothing outside a laid-out chip knows.
//
// The records are the fixed-width flags plus a name (`encode::labels`): kind, is-head, on-a-remote, has-a-PR, here,
// held elsewhere. Spelled out whole, so a flag that moves is a test that fails.
Item {
    id: root
    width: 400
    height: 200

    readonly property string head: "H10010HEAD"
    readonly property string current: "L10010main"
    readonly property string local2: "L00010hotfix"
    readonly property string held: "L00011spike"
    readonly property string remote: "R00000origin/preview"
    /// A working copy standing on this commit with no branch out — a marker, and a colour of its
    /// own (デザイン規約 §ref の種別).
    readonly property string copy: "W00010rig"
    readonly property string tagHere: "T00010v1.0"
    readonly property string tagHere2: "T00010v1.1"
    readonly property string tagAway: "T01000v2.0"

    /// A branch another working copy holds that is also on a remote — the worst-dressed card there is, which is the
    /// one the column's floor is measured on.
    readonly property string dressed: "L01011feature/a-name-far-too-long-for-any-column"

    RefChipStack {
        id: stack
        maxWidth: 200
    }
    /// A card of the kind the chip's list is made of: one name, wrapped, and a room handed to it from
    /// outside. The list hands over a room of its own as it opens and reads the width back in that same turn, which is
    /// the whole of what `RefChipRoom` is about.
    RefChip {
        id: roomChip
        wrapped: true
    }
    // The column arithmetic that has to keep back what the card above spends. Handed a pane so its own two required
    // properties are answered; nothing here reads the lanes.
    GraphColumnMetrics {
        id: columns
        paneW: 800
        maxLanes: 4
    }
    /// The deepest count a real repository draws, set the way the column's floor prices one. **The family is named
    /// here, as it is on the floor's own probe**: in the app every label takes it from the window (`Main`), and
    /// neither this nor the floor's probe has a window over it — a test that measured one against a chip standing in
    /// the runner's own default font would be comparing two families (it does, and they differ: the runner's default
    /// came out wider than `Noto Sans CJK JP` on the container and narrower on Windows).
    Label {
        id: deepestCount
        visible: false
        text: "+41"
        font.family: Theme.uiFamily
        font.pixelSize: Theme.fontSm
    }

    TestCase {
        name: "RefChipStack"
        when: windowShown

        function test_one_record_stands_alone() {
            stack.records = [root.current]
            compare(stack.sheets.length, 0)
        }

        function test_one_sheet_per_colour_in_the_row_s_own_order() {
            stack.records = [root.current, root.remote, root.tagHere]
            compare(stack.sheets, ["remote", "tag"])
        }

        // The measured shape of nearly every multi-ref row in a real repository: one commit wearing several tags.
        function test_the_front_card_s_own_colour_is_the_one_that_may_come_twice() {
            stack.records = [root.tagHere, root.tagHere2, "T00010v1.2", "T00010v1.3"]
            compare(stack.sheets, ["tag"])
            // And it stands out by the step every other sheet stands out by: the repeat sits one step from the
            // card it repeats.
            compare(stack.layout[0].x, stack.step)
        }

        // Every other colour is one sheet however many names wear it — only the sheet against the card may repeat it.
        function test_a_colour_behind_the_card_is_one_sheet_however_many_wear_it() {
            stack.records = [root.current, root.tagHere, root.tagHere2, "T00010v1.2"]
            compare(stack.sheets, ["tag"])
        }

        // Hue says the kind, lightness says where it is (デザイン規約 §ref の種別) — two colours, so two sheets.
        function test_a_tag_only_the_remote_has_is_its_own_colour() {
            stack.records = [root.current, root.tagHere, root.tagAway]
            compare(stack.sheets, ["tag", "tagdim"])
        }

        // A branch another working copy holds is nowhere a move can go, which is what the reader has to see first.
        function test_a_held_branch_is_its_own_colour() {
            stack.records = [root.local2, root.held]
            compare(stack.sheets, ["held"])
            stack.records = [root.held, root.local2]
            compare(stack.sheets, ["local"])
        }

        // And a copy standing here with no branch reads as that same chip: another working copy is on this commit,
        // said once and in one colour (デザイン規約 §ref の種別).
        function test_a_copy_with_no_branch_reads_as_the_branch_a_copy_holds() {
            stack.records = [root.copy]
            compare(stack.sheets, [])
            stack.records = [root.local2, root.copy]
            compare(stack.sheets, ["held"])
            stack.records = [root.held, root.copy, root.local2]
            compare(stack.sheets, ["held", "local"], "the two are one colour, so one sheet")
        }

        function test_the_detached_head_marker_is_its_own_colour() {
            stack.records = [root.head, root.current, root.tagHere]
            compare(stack.sheets, ["local", "tag"])
        }

        // Every colour at once, which is as many cards as a commit can come to: the marker cannot repeat, so the
        // deepest row a detached HEAD can draw is one sheet per other colour.
        function test_every_colour_at_once_is_one_sheet_apiece() {
            stack.records = [root.head, root.current, root.local2, root.held,
                             root.remote, root.copy, root.tagHere, root.tagAway]
            // **The working copy's marker wears the frame a branch another copy holds wears**, and a sheet is a
            // colour (デザイン規約 §重ね表示).
            compare(stack.sheets, ["local", "held", "remote", "tag", "tagdim"])
            compare(stack.sheets.length, stack.maxSheets)
        }

        // The everyday deep row: the branch this checkout is on, another local, a remote of its own, and a tag.
        function test_the_row_with_a_second_local_on_it() {
            stack.records = [root.current, root.local2, root.remote, root.tagHere]
            compare(stack.sheets, ["local", "remote", "tag"])
            // One step apiece, all the way back: the sheet of the card's own colour steps out the same as the
            // two behind it.
            for (let i = 0; i < stack.layout.length; ++i)
                compare(stack.layout[i].x, (i + 1) * stack.step)
        }

        /// One row of every depth there is, shallowest first: the fan behind these carries one sheet more each time,
        /// up to the deepest a row can wear.
        function depths() {
            return [[root.current, root.tagHere],
                    [root.current, root.remote, root.tagHere],
                    [root.current, root.local2, root.remote, root.tagHere],
                    [root.current, root.local2, root.held, root.remote, root.tagHere],
                    [root.current, root.local2, root.held, root.remote, root.tagHere, root.tagAway]]
        }

        // One slope for every row, at every depth: a sheet is a step right and the row's own drop down wherever
        // it stands, the fan fits the row it is drawn in, and the row centres the card and the fan
        // together.
        function test_every_sheet_takes_the_same_step_and_the_row_centres_the_stack() {
            const rows = depths()
            for (let i = 0; i < rows.length; ++i) {
                stack.records = rows[i]
                // A step and a drop apiece, all the way back.
                for (let s = 0; s < stack.layout.length; ++s) {
                    compare(stack.layout[s].x, (s + 1) * stack.step)
                    compare(stack.layout[s].y, (s + 1) * stack.drop)
                }
                compare(stack.fanDepth, stack.sheets.length * stack.drop)
                // The whole stack stands in this row and in the middle of it, with the row's odd pixel above it.
                const above = stack.fanRoom - stack.lift
                const below = stack.fanRoom + stack.lift - stack.fanDepth
                verify(below >= 1, "row " + i + " reaches the row below")
                verify(above >= 1, "row " + i + " reaches the row above")
                compare(above - below, stack.fanDepth % 2)
            }
            // And the last row of them is the deepest one there is, so what the loop read of it is what the deepest
            // fan there is comes to.
            compare(stack.sheets.length, stack.maxSheets)
        }

        // How far the fan falls is the row's own, and how many sheets it carries is what decides it: the steepest
        // whole step that many of them fit the row at, capped at the step they stand out by (デザイン規約 §重ね表示).
        // **The rule is what is held** — every row takes the whole step it has the room for, and a deeper fan
        // falls no further than a shallower one, so a token that moves the row or the chip moves the answers
        // without moving the test.
        function test_the_count_decides_how_far_the_fan_falls() {
            const rows = depths()
            const drops = []
            for (let i = 0; i < rows.length; ++i) {
                stack.records = rows[i]
                const n = stack.sheets.length
                compare(n, i + 1, "the row of depth " + (i + 1) + " built " + n + " sheets")
                verify(stack.drop >= Theme.borderWidth, "a fan of " + n + " fell short of an edge")
                verify(stack.drop <= stack.step, "a fan of " + n + " fell further than it stepped out")
                verify(n * stack.drop <= stack.fanDepthMax, "a fan of " + n + " outgrew its row")
                verify(stack.drop === stack.step || n * (stack.drop + 1) > stack.fanDepthMax,
                       "a fan of " + n + " left a whole step of room standing")
                drops.push(stack.drop)
            }
            // And across the rows: a deeper fan never falls further than a shallower one. **A flat drop at every
            // depth is already caught above** — a row that had the room for a whole step more and did not take it
            // fails there — so what is left to say here is the shape the reader sees going down the column.
            for (let d = 1; d < drops.length; ++d)
                verify(drops[d] <= drops[d - 1],
                       "a fan of " + (d + 1) + " fell further than one of " + d)
        }

        // The card standing on the chip is what the stack unfolded into: nothing may peek out from under it.
        function test_an_unstacked_chip_draws_no_sheets() {
            stack.records = [root.current, root.remote, root.tagHere]
            stack.unstacked = true
            compare(stack.sheets.length, 0)
            stack.unstacked = false
            compare(stack.sheets.length, 2)
        }
    }

    // What the front card counts, and what counting it costs the name. The sheets behind it say which colours the row
    // carries and this says how many names there are (デザイン規約 §重ね表示) — the two are separate channels, so a row
    // whose fan does not grow still moves this number.
    TestCase {
        name: "RefChipCount"
        when: windowShown

        /// A row of `n` tags on one commit, which is the shape nearly every deep row in a real repository takes.
        function tags(n) {
            const out = []
            for (let i = 0; i < n; ++i)
                out.push("T00010v1." + i)
            return out
        }

        // A card with one name on it spends nothing beside it: no count, and neither mark. The seat comes and goes
        // with what is drawn, or every name on the graph stands one count to the left of where it belongs.
        function test_a_row_with_one_name_counts_nothing() {
            stack.records = [root.current]
            verify(!stack.chipItem.hasCount)
            compare(stack.chipItem.furnitureW, 0)
        }

        // What the count costs is its own label and the half gap that follows it — the same term shape the two marks
        // take (デザイン規約 §余白).
        function test_the_count_takes_its_own_ink_and_half_a_gap() {
            stack.records = tags(12)
            const chip = stack.chipItem
            verify(chip.hasCount)
            fuzzyCompare(chip.furnitureW, chip.countW + Theme.spaceXs / 2, 0.01)
        }

        // The fan cannot answer this one: a colour is one sheet however many names wear it, so both of these rows draw
        // the single repeated sheet and only the count tells them apart.
        function test_the_count_tells_apart_two_rows_the_fan_draws_alike() {
            stack.records = tags(2)
            compare(stack.sheets, ["tag"])
            const shallow = stack.chipItem.countW
            stack.records = tags(42)
            compare(stack.sheets, ["tag"])
            verify(stack.chipItem.countW > shallow, "a two-digit count took no more room than a one-digit one")
        }

        // Every term the frame spends beside the name is in `furnitureW`, so the contents of the worst-dressed card
        // come to exactly the room inside its frame. A forgotten term is a name handed room the frame does not
        // have, and the frame clips its own right-hand end — the count first, since it stands last, and a `+41`
        // cut to `+4` is a wrong number.
        function test_the_dressed_card_s_contents_stop_at_its_frame() {
            stack.records = [root.dressed].concat(tags(41))
            const chip = stack.chipItem
            verify(chip.hasCount && chip.hasBadge && chip.recHeld, "the card is not the worst-dressed one")
            // The row inside the frame is a positioner and the name inside it elides: both answer their width in a
            // pass after the records land. **Waited out** — a read taken at the first pass that answers anything
            // at all catches the name at its unelided width.
            verify(waitForRendering(stack), "the stack was laid out and drawn")
            fuzzyCompare(chip.contentW, chip.maxWidth - 2 * Theme.spaceXs, 0.01)
            compare(chip.width, chip.maxWidth)
        }

        // The badge is this ref's own state — it is on a remote, or it has a PR open — and the count is how many
        // *others* the row carries behind it, so the badge stands with the name it describes and the count after both
        // (デザイン規約 §重ね表示). **Nothing else holds this**: the frame's contents come to the same width either way,
        // so the two can be swapped without a single sum in the card moving.
        function test_the_badge_stands_with_the_name_and_the_count_behind_them() {
            stack.records = [root.dressed].concat(tags(41))
            const chip = stack.chipItem
            verify(chip.hasBadge && chip.hasCount, "the card is not wearing both")
            // The positioner answers in a pass of its own, and it skips what is not drawn: an x read before it has run
            // is every seat's own default.
            verify(waitForRendering(stack), "the stack was laid out and drawn")
            verify(chip.badgeX < chip.countX,
                   "the count at " + chip.countX + " is not behind the badge at " + chip.badgeX)
        }

        // The floor keeps room for a count the deepest real row can reach, measured off a label of its own: an
        // unparented one still answers, which is the whole reason this term may be priced here at all.
        function test_the_column_floor_keeps_room_for_a_two_digit_count() {
            verify(columns.chipCountInk.implicitWidth > 0, "the count probe measured nothing")
            verify(columns.chipCountInk.implicitWidth >= deepestCount.implicitWidth,
                   "the floor prices the count under what the deepest real row draws")
            verify(columns.chipFurnitureW > columns.chipFan.fanMaxW + columns.chipCountInk.implicitWidth,
                   "the floor left the count out")
        }

        // **The column opens at `Metrics.labelColW`** (`GraphColumnMetrics.labelW` — only a width a hand dragged
        // is held inside the two ends), so a floor that rose past it would open every window with the chip
        // already crushed. Every term added to the furniture is spent out of this margin.
        function test_the_column_opens_no_narrower_than_its_own_floor() {
            verify(columns.labelColWMin <= Metrics.labelColW,
                   "the floor " + columns.labelColWMin + " is past the width the column opens at "
                   + Metrics.labelColW)
        }
    }

    // What a chip answers when the room it is given moves. **The card that unfolds a chip measures its rows in the
    // turn it hands them that room** (`RefListPopup.layOutRows`) — it has to, because a card that grows after it is
    // shown cannot tell that the hand walking into it is already inside — so what those rows answer in that turn is
    // what the card comes out at.
    TestCase {
        id: refChipRoom
        name: "RefChipRoom"
        when: windowShown

        /// The name a graph column has to cut and a card has room to show whole.
        readonly property string cutInTheColumn: "L00010feature/a-name-far-too-long-for-any-column"

        // A chip handed a wider room answers with the width it had before it, until it is asked to lay out: the frame's
        // contents are a positioner, and a positioner sums itself in the polish after the turn its children moved in.
        // Left unasked, the card came out at the width the graph's column had left the chip and cut the name off
        // inside its own frame (measured: 152 read back off a chip the card was about to draw at 1095).
        function test_a_chip_given_more_room_answers_in_that_turn() {
            roomChip.maxWidth = Metrics.labelColW
            roomChip.records = [refChipRoom.cutInTheColumn]
            verify(waitForRendering(roomChip), "the chip was laid out and drawn")
            compare(roomChip.width, Metrics.labelColW, "the name did not fill the column it was cut to")

            // The room the card hands over as it opens, and the reading it takes in that same turn.
            roomChip.maxWidth = 4 * Metrics.labelColW
            roomChip.layOutNow()
            const asRead = roomChip.width
            verify(waitForRendering(roomChip), "the chip was drawn at its new room")
            compare(asRead, roomChip.width, "the chip answered with a width from the pass before")
            verify(asRead > Metrics.labelColW,
                   "the chip kept the column's width in a room four times as wide")
        }
    }
}
