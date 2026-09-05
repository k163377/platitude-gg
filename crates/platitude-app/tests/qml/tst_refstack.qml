import QtQuick
import QtTest
import platitude.ui

// What the sheets behind a chip's front card come to, held against records spelled out whole. **Only the built stack
// can answer this**: the rule reads each record's colour through the card's own `kindKeyOf`, and what comes out is a
// list the item builds — no Rust test reaches either.
//
// The records are the fixed-width flags plus a name (`encode::labels`): kind, is-head, on-a-remote, has-a-PR, here,
// held elsewhere. Spelled out rather than assembled, so a flag that moves is a test that fails.
Item {
    id: root
    width: 400
    height: 200

    readonly property string head: "H10010HEAD"
    readonly property string current: "L10010main"
    readonly property string local2: "L00010hotfix"
    readonly property string held: "L00011spike"
    readonly property string remote: "R00000origin/preview"
    readonly property string tagHere: "T00010v1.0"
    readonly property string tagHere2: "T00010v1.1"
    readonly property string tagAway: "T01000v2.0"

    RefChipStack {
        id: stack
        maxWidth: 200
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
            compare(stack.layout[0].gap, stack.sameGap)
        }

        // Every other colour is one sheet however many names wear it — only the sheet against the card may repeat it.
        function test_a_colour_behind_the_card_is_one_sheet_however_many_wear_it() {
            stack.records = [root.current, root.tagHere, root.tagHere2, "T00010v1.2"]
            compare(stack.sheets, ["tag"])
            compare(stack.layout[0].gap, 0)
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
            compare(stack.layout[0].gap, 0)
            stack.records = [root.held, root.local2]
            compare(stack.sheets, ["local"])
        }

        function test_the_detached_head_marker_is_its_own_colour() {
            stack.records = [root.head, root.current, root.tagHere]
            compare(stack.sheets, ["local", "tag"])
        }

        // Every colour at once, which is as many cards as a commit can come to: the marker cannot repeat, so the
        // deepest row a detached HEAD can draw is one sheet per other colour.
        function test_every_colour_at_once_is_one_sheet_apiece() {
            stack.records = [root.head, root.current, root.local2, root.held,
                             root.remote, root.tagHere, root.tagAway]
            compare(stack.sheets, ["local", "held", "remote", "tag", "tagdim"])
            compare(stack.sheets.length, stack.maxSheets)
        }

        // The everyday deep row: the branch this checkout is on, another local, a remote of its own, and a tag.
        function test_the_row_with_a_second_local_on_it() {
            stack.records = [root.current, root.local2, root.remote, root.tagHere]
            compare(stack.sheets, ["local", "remote", "tag"])
            compare(stack.layout[0].gap, stack.sameGap)
        }

        // The slope flattens as the fan deepens and the card rises with it, and neither is allowed to reach a
        // neighbouring row: the fan stops on this row's own floor, and the card keeps a border's worth above it.
        function test_the_slope_flattens_and_the_card_rises_with_it() {
            const depths = []
            const lifts = []
            const rows = [[root.current, root.tagHere],
                          [root.current, root.remote, root.tagHere],
                          [root.current, root.local2, root.remote, root.tagHere],
                          [root.current, root.local2, root.held, root.remote, root.tagHere],
                          [root.current, root.local2, root.held, root.remote, root.tagHere, root.tagAway]]
            for (let i = 0; i < rows.length; ++i) {
                stack.records = rows[i]
                depths.push(stack.fanDepth)
                lifts.push(stack.lift)
                // The fan stays on this row: its deepest sheet stops on the floor, and the card keeps its head above
                // the ceiling by a border.
                verify(stack.fanRoom - stack.lift + stack.fanDepth <= 2 * stack.fanRoom,
                       "sheet " + i + " reaches the row below")
                verify(stack.fanRoom - stack.lift >= 1, "sheet " + i + " reaches the row above")
            }
            // Deeper every time, and never by more than the steep step.
            for (let j = 1; j < depths.length; ++j) {
                verify(depths[j] > depths[j - 1], "depth " + j + " did not grow")
                verify(depths[j] - depths[j - 1] <= stack.steepStep + stack.sameGap,
                       "depth " + j + " grew too fast")
            }
            // Nothing rises until the third sheet, and then it does.
            compare(lifts[0], 0)
            compare(lifts[1], 0)
            verify(lifts[2] > 0, "the third sheet did not lift the card")
            verify(lifts[4] > lifts[2], "the deepest row did not lift further")
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
}
