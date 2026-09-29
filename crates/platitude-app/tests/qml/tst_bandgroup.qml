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

    /// A stopped merge with conflicts: two badges, in the page's shape the group reads (`curPage.pageWt` / …).
    /// The identity and old-git badges read `AppBackend`, staged empty for these runs (`xtask::qmltest`), so they
    /// stay down.
    QtObject {
        id: workTree
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
        property var pageWt: workTree
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
        controlHeight: 24
        cellFolded: false
        width: 400
        height: 24
    }

    /// The floor measured again outside the group: a group handing its share-out some other number would report
    /// that number as its own.
    BandStateMetrics {
        id: witness
        stateWt: workTree
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
            group.tabContentWidth = 100
            group.tabRunAvail = 400
            group.tabCount = 1
            workTree.opText = "MERGING"
            workTree.hasConflicts = true
            workTree.lfsNeeded = 0
            graph.failed = false
            graph.stale = false
        }

        function badgesDrawn() {
            for (let i = 0; i < group.children.length; i++) {
                const row = group.children[i]
                if (!row.visible || row.children === undefined)
                    continue
                let widths = []
                for (let j = 0; j < row.children.length; j++) {
                    const badge = row.children[j]
                    if (badge.visible && badge.naturalW !== undefined)
                        widths.push(badge.width)
                }
                if (widths.length > 0)
                    return widths
            }
            return []
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

            workTree.lfsNeeded = 2
            verify(group.lfsBadgeShown, "two files need it")
            compare(group.naturalWidth, without + Theme.spaceXs + group.lfsBadgeW, "and the group asks for its room")
            compare(badgesDrawn(), [group.opBadgeW, group.conflictBadgeW, group.lfsBadgeW],
                    "after the operation and its conflicts")

            workTree.opText = ""
            workTree.hasConflicts = false
            verify(group.stateShown, "it stands the group up on its own")
            compare(badgesDrawn(), [group.lfsBadgeW])
            group.windowAtFloor = true
            verify(group.stateMarkShown)
            verify(Qt.colorEqual(group.stateMarkColor, Theme.warning), "folded, its mark is yellow")

            workTree.lfsNeeded = 0
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

            workTree.hasConflicts = false
            verify(!group.conflictBadgeShown)
            workTree.opText = ""
            verify(!group.opBadgeShown)
            verify(!group.stateShown, "with nothing the matter the group is not there at all")
            verify(!group.visible)
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
