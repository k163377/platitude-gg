import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

// The sheets behind a chip's front card, and what the card spends on its count. Only the built stack can answer
// this: the colour comes through the card's own `kindKeyOf`, and `furnitureW` sums what the laid-out labels and marks
// came to — no Rust test reaches either.
//
// The records are the chips `encode::chips_of` hands the rows, every field spelled out, so a field that moves fails.
Item {
    id: root
    width: 400
    height: 200

    /// Every mark down unless `marks` raises it; `here` by default, as `RefChip.noChip` reads.
    function chip(kind, name, marks) {
        return Object.assign({ "kind": kind, "name": name, "isHead": false, "hasRemote": false, "hasPr": false,
                               "here": true, "held": false, "locked": false, "remote": "", "key": kind + ":" + name },
                             marks === undefined ? {} : marks)
    }

    readonly property var head: root.chip("head", "HEAD", { "isHead": true })
    readonly property var current: root.chip("branch", "main", { "isHead": true })
    readonly property var local2: root.chip("branch", "hotfix")
    readonly property var held: root.chip("branch", "spike", { "held": true })
    readonly property var heldLocked: root.chip("branch", "spike", { "held": true, "locked": true })
    readonly property var remote: root.chip("remote", "origin/preview", { "here": false })
    /// A working copy on this commit with no branch out (デザイン規約 §ref の種別).
    readonly property var copy: root.chip("worktree", "rig")
    readonly property var copyLocked: root.chip("worktree", "rig", { "locked": true })
    readonly property var tagHere: root.chip("tag", "v1.0")
    readonly property var tagHere2: root.chip("tag", "v1.1")
    readonly property var tagAway: root.chip("tag", "v2.0", { "hasRemote": true, "here": false })

    /// Held by a locked copy and on a remote: the worst-dressed card, both marks at once, which the column's floor is
    /// measured on.
    readonly property var dressed: root.chip("branch", "feature/a-name-far-too-long-for-any-column",
                                             { "hasRemote": true, "held": true, "locked": true })

    RefChipStack {
        id: stack
        maxWidth: 200
    }
    /// A card as the chip's list draws it: wrapped, with a room handed in from outside (`RefChipRoom`).
    RefChip {
        id: roomChip
        wrapped: true
    }
    /// A list card whose name reads something: the ref it reads goes under it in the same frame (`RefChip.mate`).
    RefChip {
        id: readingChip
        maxWidth: 400
    }
    // The column floor that has to keep back what the card spends. `paneW` / `maxLanes` only answer its required
    // properties.
    GraphColumnMetrics {
        id: columns
        paneW: 800
        maxLanes: 4
    }
    /// The deepest count a real repository draws, set the way the column's floor prices one. The family is named, as
    /// on the floor's probe: labels take it from the window (`Main`) and neither has one, so the runner's default
    /// font would make them two different families.
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

        // The commonest multi-ref row: one commit wearing several tags.
        function test_the_front_card_s_own_colour_is_the_one_that_may_come_twice() {
            stack.records = [root.tagHere, root.tagHere2, root.chip("tag", "v1.2"), root.chip("tag", "v1.3")]
            compare(stack.sheets, ["tag"])
            // The repeat sits one step from the card, like every other sheet.
            compare(stack.layout[0].x, stack.step)
        }

        function test_a_colour_behind_the_card_is_one_sheet_however_many_wear_it() {
            stack.records = [root.current, root.tagHere, root.tagHere2, root.chip("tag", "v1.2")]
            compare(stack.sheets, ["tag"])
        }

        // Hue says the kind, lightness says where it is (デザイン規約 §ref の種別) — two colours, so two sheets.
        function test_a_tag_only_the_remote_has_is_its_own_colour() {
            stack.records = [root.current, root.tagHere, root.tagAway]
            compare(stack.sheets, ["tag", "tagdim"])
        }

        // A held branch says where that copy is — what the reader has to see first.
        function test_a_held_branch_is_its_own_colour() {
            stack.records = [root.local2, root.held]
            compare(stack.sheets, ["worktree"])
            stack.records = [root.held, root.local2]
            compare(stack.sheets, ["local"])
        }

        // Another working copy on this commit is one colour, with a branch out or not (デザイン規約 §ref の種別).
        function test_a_copy_with_no_branch_reads_as_the_branch_a_copy_holds() {
            stack.records = [root.copy]
            compare(stack.sheets, [])
            stack.records = [root.local2, root.copy]
            compare(stack.sheets, ["worktree"])
            stack.records = [root.held, root.copy, root.local2]
            compare(stack.sheets, ["worktree", "local"], "the two are one colour, so one sheet")
        }

        // The front card's own colour is the one a fan may repeat (デザイン規約 §重ね表示). Read with the front card's
        // key: a single `worktree` sheet is also what one copy plus one other colour comes to.
        function test_two_copies_on_one_commit_put_a_green_sheet_behind_a_green_card() {
            stack.records = [root.held, root.copy]
            compare(stack.chipItem.kindKeyOf(stack.records[0]), "worktree", "the card in front is not a copy's")
            compare(stack.sheets, ["worktree"], "the second copy drew no sheet of its own colour")
            // A third is the same single sheet; the card's `+N` says how many.
            stack.records = [root.held, root.heldLocked, root.copy]
            compare(stack.sheets, ["worktree"])
        }

        // The tree mark belongs to the name, not the state: a held branch's name is still a branch's
        // (デザイン規約 §ref の種別).
        function test_only_the_chip_naming_a_folder_wears_the_tree_mark() {
            stack.records = [root.copy]
            verify(stack.chipItem.hasTree, "the copy's own name came out with nothing saying it is a folder")
            // The padlock takes the seat: only a working copy can be locked, so it says what the tree says, and the
            // frame is too short for both.
            stack.records = [root.copyLocked]
            verify(stack.chipItem.hasLock && !stack.chipItem.hasTree, "a locked copy wore both marks")
            stack.records = [root.held]
            verify(!stack.chipItem.hasTree, "a branch is not a folder, whoever has it out")
            stack.records = [root.current]
            verify(!stack.chipItem.hasTree)
        }

        // The padlock is the copy's, on either shape a copy takes (デザイン規約 §ref の種別). The colour does not move
        // with it: a locked copy is still a copy.
        function test_the_padlock_stands_on_a_locked_copy_and_nowhere_else() {
            stack.records = [root.held]
            verify(!stack.chipItem.hasLock, "an unlocked holder put a padlock on the card")
            stack.records = [root.heldLocked]
            verify(stack.chipItem.hasLock, "the locked holder's card wears no padlock")
            stack.records = [root.copy]
            verify(!stack.chipItem.hasLock)
            stack.records = [root.copyLocked]
            verify(stack.chipItem.hasLock, "the marker for a locked copy wears no padlock")
            stack.records = [root.local2, root.heldLocked, root.copyLocked]
            compare(stack.sheets, ["worktree"], "the lock is not a colour of its own")
        }

        function test_the_detached_head_marker_is_its_own_colour() {
            stack.records = [root.head, root.current, root.tagHere]
            compare(stack.sheets, ["local", "tag"])
        }

        // The deepest row a detached HEAD can draw: the marker cannot repeat, so one sheet per other colour.
        function test_every_colour_at_once_is_one_sheet_apiece() {
            stack.records = [root.head, root.current, root.local2, root.held,
                             root.remote, root.copy, root.tagHere, root.tagAway]
            compare(stack.sheets, ["local", "worktree", "remote", "tag", "tagdim"])
            compare(stack.sheets.length, stack.maxSheets)
        }

        function test_the_row_with_a_second_local_on_it() {
            stack.records = [root.current, root.local2, root.remote, root.tagHere]
            compare(stack.sheets, ["local", "remote", "tag"])
            for (let i = 0; i < stack.layout.length; ++i)
                compare(stack.layout[i].x, (i + 1) * stack.step)
        }

        /// A row of every depth, shallowest first, one sheet more each, up to the deepest.
        function depths() {
            return [[root.current, root.tagHere],
                    [root.current, root.remote, root.tagHere],
                    [root.current, root.local2, root.remote, root.tagHere],
                    [root.current, root.local2, root.held, root.remote, root.tagHere],
                    [root.current, root.local2, root.held, root.remote, root.tagHere, root.tagAway]]
        }

        function test_every_sheet_takes_the_same_step_and_the_row_centres_the_stack() {
            const rows = depths()
            for (let i = 0; i < rows.length; ++i) {
                stack.records = rows[i]
                for (let s = 0; s < stack.layout.length; ++s) {
                    compare(stack.layout[s].x, (s + 1) * stack.step)
                    compare(stack.layout[s].y, (s + 1) * stack.drop)
                }
                compare(stack.fanDepth, stack.sheets.length * stack.drop)
                // The stack stands inside the row and centred, the odd pixel above.
                const above = stack.fanRoom - stack.lift
                const below = stack.fanRoom + stack.lift - stack.fanDepth
                verify(below >= 1, "row " + i + " reaches the row below")
                verify(above >= 1, "row " + i + " reaches the row above")
                compare(above - below, stack.fanDepth % 2)
            }
            compare(stack.sheets.length, stack.maxSheets)
        }

        // The drop is the steepest whole step the sheet count fits in the row, capped at the step out
        // (デザイン規約 §重ね表示). The rule is held, not the numbers, so a token that moves the row or the chip does not
        // move the test.
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
            // Across the rows: a deeper fan never falls further (a flat drop is already caught by the loop above).
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

    // What the front card's count costs the name. The fan says which colours, the count how many names
    // (デザイン規約 §重ね表示) — separate channels.
    TestCase {
        name: "RefChipCount"
        when: windowShown

        function tags(n) {
            const out = []
            for (let i = 0; i < n; ++i)
                out.push(root.chip("tag", "v1." + i))
            return out
        }

        // The seat comes and goes with what is drawn, or every name on the graph stands one count too far left.
        function test_a_row_with_one_name_counts_nothing() {
            stack.records = [root.current]
            verify(!stack.chipItem.hasCount)
            compare(stack.chipItem.furnitureW, 0)
        }

        // The same term shape the two marks take (デザイン規約 §余白).
        function test_the_count_takes_its_own_ink_and_half_a_gap() {
            stack.records = tags(12)
            const chip = stack.chipItem
            verify(chip.hasCount)
            fuzzyCompare(chip.furnitureW, chip.countW + Theme.spaceXs / 2, 0.01)
        }

        function test_the_count_tells_apart_two_rows_the_fan_draws_alike() {
            stack.records = tags(2)
            compare(stack.sheets, ["tag"])
            const shallow = stack.chipItem.countW
            stack.records = tags(42)
            compare(stack.sheets, ["tag"])
            verify(stack.chipItem.countW > shallow, "a two-digit count took no more room than a one-digit one")
        }

        // Every term beside the name is in `furnitureW`: a forgotten one hands the name room the frame lacks, and the
        // frame clips its right end — the count first, and `+41` cut to `+4` is a wrong number.
        function test_the_dressed_card_s_contents_stop_at_its_frame() {
            stack.records = [root.dressed].concat(tags(41))
            const chip = stack.chipItem
            verify(chip.hasCount && chip.hasBadge && chip.recHeld && chip.hasLock,
                   "the card is not the worst-dressed one")
            // The positioner and the eliding name answer their widths a pass after the records land; an earlier read
            // catches the name unelided.
            verify(waitForRendering(stack), "the stack was laid out and drawn")
            fuzzyCompare(chip.contentW, chip.maxWidth - 2 * Theme.spaceXs, 0.01)
            compare(chip.width, chip.maxWidth)
        }

        // The badge is this ref's own state and the count is the others behind it, so the badge stands with the name
        // (デザイン規約 §重ね表示). Nothing else holds this: swapped, every sum in the card stays the same.
        function test_the_badge_stands_with_the_name_and_the_count_behind_them() {
            stack.records = [root.dressed].concat(tags(41))
            const chip = stack.chipItem
            verify(chip.hasBadge && chip.hasCount, "the card is not wearing both")
            // An x read before the positioner's pass is every seat's own default.
            verify(waitForRendering(stack), "the stack was laid out and drawn")
            verify(chip.badgeX < chip.countX,
                   "the count at " + chip.countX + " is not behind the badge at " + chip.badgeX)
        }

        // Measured off a label of its own: an unparented label still answers, which is what lets the floor price it.
        function test_the_column_floor_keeps_room_for_a_two_digit_count() {
            verify(columns.chipCountInk.implicitWidth > 0, "the count probe measured nothing")
            verify(columns.chipCountInk.implicitWidth >= deepestCount.implicitWidth,
                   "the floor prices the count under what the deepest real row draws")
            verify(columns.chipFurnitureW > columns.chipFan.fanMaxW + columns.chipCountInk.implicitWidth,
                   "the floor left the count out")
        }

        // The column opens at `Metrics.labelColW` (`GraphColumnMetrics.labelW` clamps only a dragged width), so a floor
        // past it would open every window with the chip crushed. Every furniture term spends this margin.
        function test_the_column_opens_no_narrower_than_its_own_floor() {
            verify(columns.labelColWMin <= Metrics.labelColW,
                   "the floor " + columns.labelColWMin + " is past the width the column opens at "
                   + Metrics.labelColW)
        }
    }

    // The card that unfolds a chip measures its rows in the turn it hands them their room (`RefListPopup.layOutRows`)
    // — a card growing after it is shown cannot tell the hand is already inside — so what they answer in that turn is
    // the card's width.
    TestCase {
        id: refChipRoom
        name: "RefChipRoom"
        when: windowShown

        /// The name a graph column has to cut and a card has room to show whole.
        readonly property var cutInTheColumn: root.chip("branch", "feature/a-name-far-too-long-for-any-column")

        // The frame's contents are a positioner, which sums itself in the polish after its children moved: until asked
        // to lay out (`layOutNow`) the chip answers with its old width, and the card cuts the name inside its frame.
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

    // A chip whose name reads something says so inside its frame
    // (デザイン規約 §グラフ行のダブルクリック「カードの行は、その名前が読んでいる相手を同じ枠の中で言う」). No picture can answer
    // these: a line box and a two-pixel seat are not readable off a screenshot.
    TestCase {
        id: refChipReading
        name: "RefChipReading"
        when: windowShown

        /// A branch chip's mate: its reading, with the branch's counts (`NavFacts.readingLine` via
        /// `RowHoverHost.mateOf`).
        readonly property var reads: ({ "mark": "remote", "markTint": Theme.textSecondary,
                                        "text": "origin/preview", "tone": Theme.textSecondary,
                                        "ahead": 1, "behind": 0 })
        /// A remote-tracking chip's mate: the branch reading it, with its counts (`NavFacts.branchLine`).
        readonly property var readBy: ({ "mark": "branch", "markTint": Theme.accent,
                                         "text": "hotfix", "tone": Theme.textSecondary,
                                         "ahead": 1, "behind": 1 })

        function init() {
            readingChip.mate = null
            readingChip.maxWidth = 400
        }

        // The graph's own chips read nothing and stay one line: a graph row has no room for a second
        // (デザイン規約 §グラフ行のダブルクリック).
        function test_a_chip_that_reads_nothing_is_one_line() {
            readingChip.records = [root.current]
            compare(readingChip.height, Theme.fontChipLine + 2 * Theme.borderWidth)
        }

        // One more line box, not the words' height — that would read as padding that lost its bottom half.
        function test_the_reading_is_a_line_box_of_its_own() {
            readingChip.records = [root.current]
            readingChip.mate = refChipReading.reads
            compare(readingChip.height, 2 * Theme.fontChipLine + 2 * Theme.borderWidth)
        }

        // On the chip's own name where the chip is the branch; on the line under it where the chip is what the branch
        // reads.
        function test_the_measure_stands_on_the_line_that_names_the_branch() {
            readingChip.records = [root.current]
            readingChip.mate = refChipReading.reads
            verify(readingChip.trackOnName, "a branch's own chip put the measure under its name")
            readingChip.records = [root.remote]
            readingChip.mate = refChipReading.readBy
            verify(!readingChip.trackOnName, "a remote's chip put the measure on the name it is not measuring")
        }

        // The count's seat is set by the ink, not the box: on the box a `fontSm` digit sits low against a `fontChip`
        // name.
        function test_the_measure_takes_the_count_s_own_seat() {
            readingChip.records = [root.current, root.tagHere]
            readingChip.mate = refChipReading.reads
            verify(waitForRendering(readingChip), "the chip was laid out and drawn")
            verify(readingChip.hasCount, "the card is not wearing the count this seat is read off")
            compare(readingChip.trackY, readingChip.countY)
        }

        // A tag standing on another commit than the reference says so as that line, in the warning a gone reading
        // wears there (`NavFacts.apartLine` via `RowHoverHost.mateOf`): nothing can hover over the card. The tag's
        // own name keeps the colour that says where it is.
        function test_a_tag_standing_apart_says_so_under_its_name() {
            readingChip.records = [root.tagHere]
            readingChip.mate = NavFacts.apartLine(NavFacts.apartNote("origin"))
            readingChip.layOutNow()
            compare(readingChip.height, 2 * Theme.fontChipLine + 2 * Theme.borderWidth)
            const words = []
            const walk = item => {
                if (item.text === "Differs from origin")
                    words.push(item)
                for (let i = 0; i < item.children.length; i++)
                    walk(item.children[i])
            }
            walk(readingChip)
            compare(words.length, 1, "the sentence is drawn once, as the line")
            compare(words[0].color, Theme.warning)
            compare(readingChip.nameColor, Theme.textPrimary, "the name still says it is held here")
            verify(!readingChip.mateGoes, "it names no ref, so it goes nowhere")
        }

        // The line is inside the frame, so it widens the card (`RefListPopup.layOutRows` reads this back).
        function test_a_reading_longer_than_the_name_widens_the_frame() {
            readingChip.records = [root.current]
            const alone = readingChip.width
            readingChip.mate = refChipReading.reads
            readingChip.layOutNow()
            verify(readingChip.width > alone,
                   "the frame stayed at " + alone + " with a longer name under it")
        }
    }
}
