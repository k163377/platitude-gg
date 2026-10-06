import QtQuick
import QtTest
import platitude.ui

// The real `BandStateGroup`, driven by its own inputs: the wiring between the share-out and what the band draws
// (デザイン規約 §ウィンドウの縁 の譲る順). Why `tst_bandshare.qml` alone is not enough: rules-refs/app-ui.md
// 「単体だけでは足りない」.
//
// Nothing here recomputes a cap or a floor: the numbers come off the group, and a second `BandStateMetrics` outside
// it witnesses that the floor it reports is a real measurement.
Item {
    id: root
    width: 600
    height: 100

    /// A stopped merge with conflicts: two badges, in the page's shape the group reads (`curPage.pageWorkingTree` / …).
    /// The identity and old-git badges read `AppBackend`, staged empty for these runs (`xtask::qmltest`), so they
    /// stay down.
    QtObject {
        id: workingTree
        property string opText: "MERGING"
        property string opAlso: ""
        property int opStep: 0
        property int opSteps: 0
        property bool hasConflicts: true
        property int lfsNeeded: 0
    }
    QtObject {
        id: graph
        property bool failed: false
        property bool stale: false
    }
    QtObject {
        id: tab
        property bool identityReady: true
    }
    QtObject {
        id: page
        property var pageWorkingTree: workingTree
        property var pageGraph: graph
        property var pageTab: tab
    }

    BandStateGroup {
        id: group
        curPage: page
        // The floor and the strip's crowd held off (the strip wants a quarter of its run), so only the cap can fold.
        windowAtFloor: false
        tabContentWidth: 100
        tabRunAvail: 400
        tabCount: 1
        controlPadding: 6
        cellFolded: false
        // Off the pixel grid, as a name's centred line can stand: the boxes have to land on it whole.
        lineBaseline: 25.6
        width: 400
        height: Theme.toolbarHeight
    }

    /// The floor measured again outside the group: a group handing its share-out some other number would report
    /// that number as its own.
    BandStateMetrics {
        id: witness
        stateWorkingTree: workingTree
        hasAlso: false
        hasStep: false
        minChars: group.stateMinChars
    }

    TestCase {
        name: "BandGroup"
        when: windowShown

        /// The page's inputs too: a case failing part-way would hand the next the state it stopped in.
        function init() {
            group.width = 400
            group.room = Qt.binding(() => group.width)
            group.windowAtFloor = false
            group.cellFolded = false
            group.tabContentWidth = 100
            group.tabRunAvail = 400
            group.tabCount = 1
            workingTree.opText = "MERGING"
            workingTree.hasConflicts = true
            workingTree.lfsNeeded = 0
            graph.failed = false
            graph.stale = false
        }

        function badgesStanding() {
            for (let i = 0; i < group.children.length; i++) {
                const row = group.children[i]
                if (!row.visible || row.children === undefined)
                    continue
                let badges = []
                for (let j = 0; j < row.children.length; j++) {
                    const badge = row.children[j]
                    if (badge.visible && badge.naturalW !== undefined)
                        badges.push(badge)
                }
                if (badges.length > 0)
                    return badges
            }
            return []
        }

        function badgesDrawn() {
            return badgesStanding().map(badge => badge.width)
        }

        /// The first word drawn in a badge: the one item in its words' row with a baseline.
        function firstWord(badge) {
            for (let i = 0; i < badge.children.length; i++) {
                const row = badge.children[i]
                for (let j = 0; row.children !== undefined && j < row.children.length; j++) {
                    if (row.children[j].visible && row.children[j].baselineOffset > 0)
                        return row.children[j]
                }
            }
            return null
        }

        /// The widest width at which the group has folded — swept, since the boundary in pixels is the platform
        /// font's.
        function widestFolded() {
            for (let w = 400; w >= 16; w--) {
                group.width = w
                if (group.stateMarkShown)
                    return w
            }
            return -1
        }

        /// The reported floor first: had it drifted from the measurement, every case below would agree with itself.
        function test_the_floor_the_group_reports_is_a_real_measurement() {
            verify(witness.minW > 0, "the measurer answered")
            compare(group.stateBadgeMinW, witness.minW)
        }

        /// Both sides of the boundary, which has to sit on the floor the group reports.
        function test_the_words_go_where_the_measured_floor_is() {
            const folded = widestFolded()
            verify(folded > 0, "the group never gave its words up at any width down to 16")

            group.width = folded
            verify(!group.stateWordsShown, "at the fold the words are gone")
            verify(group.stateMarkShown, "and the mark is standing in their place")
            verify(group.stateCapW >= 0, "which the cap got to say, not the floor or the strip")
            verify(group.stateCapW < group.stateBadgeMinW,
                   "the fold sits under the measured floor, not somewhere else: cap "
                   + group.stateCapW + " against " + group.stateBadgeMinW)

            group.width = folded + 1
            verify(group.stateWordsShown, "one pixel wider the words are back")
            verify(!group.stateMarkShown, "and the mark is gone")
            verify(group.stateCapW >= group.stateBadgeMinW,
                   "the last width with words is the last at or above the floor: cap "
                   + group.stateCapW + " against " + group.stateBadgeMinW)
        }

        /// The cap reaches the boxes, not just the report.
        function test_the_narrowed_badges_are_drawn_at_the_cap() {
            const folded = widestFolded()
            // One pixel over the fold: the words kept, the badges cut.
            group.width = folded + 1
            const drawn = badgesDrawn()
            compare(drawn.length, 2, "the stopped operation and its conflicts")
            for (const w of drawn)
                compare(w, group.stateCapW, "every badge comes down to the one width")

            // With room to spare nothing is cut (`-1`), and the boxes are at their measured widths.
            group.width = 400
            compare(group.stateCapW, -1)
            compare(badgesDrawn(), [group.opBadgeW, group.conflictBadgeW])
        }

        /// Folded, the group is laid out at its mark alone (`TopBar`), so a fold read off its own width would never
        /// come undone (rules-refs/app-ui.md「畳んだ群は印のセルだけを取る」).
        function test_the_fold_reads_the_room_handed_in_not_the_width_drawn() {
            const folded = widestFolded()
            verify(folded > 0, "the group never gave its words up at any width down to 16")
            group.room = folded
            group.width = group.foldedWidth
            verify(group.stateMarkShown, "short of room, the mark stands")
            verify(group.markFitted, "and the group is drawn at the mark and nothing more")

            group.room = 400
            verify(group.stateWordsShown, "handed room, the words come back while the group is still drawn narrow")
            verify(!group.stateMarkShown)
        }

        function test_a_crowd_on_the_strip_folds_the_group_at_a_width_that_was_not_short() {
            compare(group.width, 400)
            verify(group.stateWordsShown, "room for every word")
            const wide = group.width

            // Six tabs wanting three times the run they have: two of them stand, and three is the floor.
            group.tabContentWidth = 600
            group.tabRunAvail = 200
            group.tabCount = 6
            compare(group.width, wide, "the window did not move")
            verify(!group.stateWordsShown, "the words went with the strip's crowd")
            verify(group.stateMarkShown)
            compare(group.stateCapW, -1, "and the group itself was never short")

            // The same crowd given room for three tabs is not a crowd.
            group.tabRunAvail = 300
            compare(group.width, wide)
            verify(group.stateWordsShown, "the words come back without the window moving either")
            verify(!group.stateMarkShown)
        }

        /// Files that need Git LFS where git cannot run it raise their own badge, drawn after the rest of the page's
        /// (only `OLD GIT` would follow it), with room asked for it — and in yellow: the mark turns red only for a
        /// conflict or a stale graph (デザイン規約 §ウィンドウの縁).
        function test_files_that_need_git_lfs_raise_their_own_badge_last_among_the_pages() {
            verify(!group.lfsBadgeShown, "nothing needs Git LFS")
            const without = group.naturalWidth

            workingTree.lfsNeeded = 2
            verify(group.lfsBadgeShown, "two files need it")
            compare(group.naturalWidth, without + Theme.spaceXs + group.lfsBadgeW, "and the group asks for its room")
            compare(badgesDrawn(), [group.opBadgeW, group.conflictBadgeW, group.lfsBadgeW],
                    "after the operation and its conflicts")

            workingTree.opText = ""
            workingTree.hasConflicts = false
            verify(group.stateShown, "it stands the group up on its own")
            compare(badgesDrawn(), [group.lfsBadgeW])
            group.windowAtFloor = true
            verify(group.stateMarkShown)
            verify(Qt.colorEqual(group.stateMarkColor, Theme.warning), "folded, its mark is yellow")

            workingTree.lfsNeeded = 0
            verify(!group.lfsBadgeShown)
            verify(!group.stateShown)
        }

        /// Four of the six badges read the page in front, so a page state handed in answers them; that a real
        /// repository's state reaches the model is the headless runs' half.
        function test_which_badge_stands_is_the_page_state_handed_in() {
            verify(group.opBadgeShown, "a stopped operation")
            verify(group.conflictBadgeShown, "and the conflicts under it")
            verify(!group.staleBadgeShown, "and a graph that is this repository's")
            verify(group.stateShown, "so the group stands")

            graph.stale = true
            verify(group.staleBadgeShown, "a graph that has gone out of date says so")
            graph.stale = false
            graph.failed = true
            verify(group.staleBadgeShown, "and so does a walk that gave up part-way — one badge, two ways in")
            graph.failed = false
            verify(!group.staleBadgeShown)

            workingTree.hasConflicts = false
            verify(!group.conflictBadgeShown)
            workingTree.opText = ""
            verify(!group.opBadgeShown)
            verify(!group.stateShown, "with nothing the matter the group is not there at all")
            verify(!group.visible)
        }

        /// The folded mark's frame as drawn: the one bordered box in a cell standing in the group (folded, the badges'
        /// row is down).
        function markFrame() {
            for (let i = 0; i < group.children.length; i++) {
                const cell = group.children[i]
                if (!cell.visible || cell.children === undefined)
                    continue
                for (let j = 0; j < cell.children.length; j++) {
                    const box = cell.children[j]
                    if (box.border !== undefined && box.border.width > 0)
                        return box
                }
            }
            return null
        }

        /// The words stand on the band's line, a whole pixel, and each box is cut round the capitals: as much air
        /// over them as under the baseline (規約 §ウィンドウの縁「バッジの箱」). The baseline is the drawn word's own;
        /// the cap height is the second measurer's.
        function test_the_words_stand_on_the_bands_line_in_a_box_cut_round_the_capitals() {
            const badges = badgesStanding()
            compare(badges.length, 2, "the stopped operation and its conflicts")
            const line = Math.round(group.lineBaseline)
            for (const badge of badges) {
                const word = firstWord(badge)
                verify(word !== null, "the badge draws a word")
                compare(word.mapToItem(group, 0, word.baselineOffset).y, line, "on the band's line: " + word.text)
                const top = badge.mapToItem(group, 0, 0).y
                compare(top, Math.round(top), "the box on whole pixels: " + word.text)
                const over = line - witness.capRows - (top + Theme.borderWidth)
                const under = top + badge.height - Theme.borderWidth - line
                compare(over, under, "as much air over the capitals as under the baseline: " + word.text)
                verify(badge.height <= Theme.iconXl, "within the band's box step: " + badge.height)
            }
        }

        /// Folded, the frame is the badges' box where they stood, in the band's own cell and in an end cell alike: a
        /// frame the cell's depth lies along the band's top and bottom edges (規約 §ウィンドウの縁「畳んだ `…` の箱」).
        /// Read against the second measurer, not the group's `foldedDepth`.
        function test_the_folded_mark_keeps_the_badges_box_clear_of_the_bands_edges() {
            const standing = badgesStanding()[0].mapToItem(group, 0, 0).y
            group.windowAtFloor = true
            verify(group.stateMarkShown)
            for (const endCell of [false, true]) {
                group.cellFolded = endCell
                const frame = markFrame()
                verify(frame !== null, "the folded mark draws its frame (end cell: " + endCell + ")")
                compare(frame.height, witness.depth, "a badge's depth (end cell: " + endCell + ")")
                compare(frame.mapToItem(group, 0, 0).y, standing,
                        "where the badges stood (end cell: " + endCell + ")")
                verify(frame.height < group.height, "clear of the band's edges")
                verify(group.markDeep, "and the run's reading agrees")
            }
        }

        function test_the_window_on_its_floor_takes_the_words() {
            verify(group.stateWordsShown)
            group.windowAtFloor = true
            verify(!group.stateWordsShown)
            verify(group.stateMarkShown)
            compare(group.stateCapW, -1, "nothing here was about the group being short")
        }
    }
}
