import QtQuick
import QtTest
import platitude.ui

// The real `BandStateGroup`, driven by its own inputs: **the wiring between the share-out and what the band draws**
// (デザイン規約 §ウィンドウの縁 の譲る順).
//
// `tst_bandshare.qml` asks the arithmetic, and asks it of a `BandStateShare` it sets up itself — so it says nothing
// about whether the group hands that object the floor it measured, or reads the answer back into anything visible.
// Cut the group's `badgeMinW` wiring to a constant 0 and that file stays green; so do the two headless runs left,
// which both stand in a band with room for every word, and so does `old-git-fold`, which folds through
// `windowAtFloor` and never asks the cap at all.
//
// **So this file drives the product component and reads the product's own display properties.** Nothing here
// recomputes a cap or a floor: the numbers come off the group, the flip is required to sit on the floor the group
// reports, and the floor the group reports is required to be a real measurement — a second `BandStateMetrics`,
// standing outside the group, is the witness for that.
Item {
    id: root
    width: 600
    height: 100

    /// A stopped merge with conflicts under it, which is what puts two badges in the group. Written as the page's own
    /// shape because that is what the group reads (`curPage.pageWt` / `pageGraph` / `pageTab`).
    ///
    /// **Two badges and no more.** The identity and old-git conditions read `AppBackend`, which is the application's
    /// own Rust-backed module and is staged empty for these runs (`xtask::qmltest`) — their bindings answer false
    /// here, and nothing below asks them to be anything else. What is under test is the share-out reaching the
    /// drawing, and two badges narrowing together is the whole of that.
    QtObject {
        id: workTree
        property string opText: "MERGING"
        property string opAlso: ""
        property int opStep: 0
        property int opSteps: 0
        property bool hasConflicts: true
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
        // Both of the other two ways into the fold are held open, so what is left to close it is the cap
        // (規約 §ウィンドウの縁). The strip wants a quarter of the run it has, which is no crowd at any width below.
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

    /// The floor, measured again outside the group. **The witness is the product's own measurer**, not a second copy
    /// of what it works out: a group that handed its share-out some other number would still report that number as
    /// its own, and a flip checked against it alone would sit exactly where it was told to.
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

        /// Every input back where it started, the page's included: the cases share one group, and a case that
        /// fails part-way would otherwise hand the next one the state it stopped in.
        function init() {
            group.width = 400
            group.room = Qt.binding(() => group.width)
            group.windowAtFloor = false
            group.tabContentWidth = 100
            group.tabRunAvail = 400
            group.tabCount = 1
            workTree.opText = "MERGING"
            workTree.hasConflicts = true
            graph.failed = false
            graph.stale = false
        }

        /// The widths of the badges the band is actually drawing, found by what they are rather than by name: the
        /// boxes carrying a natural width, inside whichever child of the group is standing.
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

        /// The widest window at which the group has given its words up, swept for rather than worked out — what the
        /// boundary is in pixels is the platform font's answer, and this file is not the place that knows it.
        function widestFolded() {
            for (let w = 400; w >= 16; w--) {
                group.width = w
                if (group.stateMarkShown)
                    return w
            }
            return -1
        }

        /// **The floor the group acts on is the one it measured.** The reported floor first — a group whose own
        /// property had drifted from the measurement would let every case below agree with itself.
        function test_the_floor_the_group_reports_is_a_real_measurement() {
            verify(witness.minW > 0, "the measurer answered")
            compare(group.stateBadgeMinW, witness.minW)
        }

        /// **Where the words go and the mark comes, in the real group.** Both sides of the boundary, and the boundary
        /// required to sit on the floor the group reports: a narrowing that never reached the floor, or one that
        /// reached some other number, lands one of these two.
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

        /// **The cap reaches the boxes, not just the report.** At a width that narrows them, every badge standing is
        /// drawn at the cap — which is the whole point of narrowing them *together* (同§), and the half the two
        /// booleans above cannot carry.
        function test_the_narrowed_badges_are_drawn_at_the_cap() {
            const folded = widestFolded()
            // Wide enough to keep the words, narrow enough to have cut them: one pixel over the fold.
            group.width = folded + 1
            const drawn = badgesDrawn()
            compare(drawn.length, 2, "the stopped operation and its conflicts")
            for (const w of drawn)
                compare(w, group.stateCapW, "every badge comes down to the one width")

            // And with room to spare none of them is cut: `-1` is the group saying it narrowed nothing, and the
            // boxes are then at the widths the measurer gave them.
            group.width = 400
            compare(group.stateCapW, -1)
            compare(badgesDrawn(), [group.opBadgeW, group.conflictBadgeW])
        }

        /// **The fold is read off the room the group is handed, not the width it is drawn at.** Folded, the band lays
        /// the group out at its mark and nothing more (`TopBar`), so its own width then says nothing about whether the
        /// words would fit again: a group that read it would stay folded however wide the window grew — and one that
        /// kept the words' width to avoid that stands empty band beside its mark.
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

        /// **The strip's crowd reaches the group without the window moving.** The width stays where it was and the
        /// group folds anyway, which is the second of the three ways in and the one that arrives from outside
        /// (規約 §ウィンドウの縁). A group whose fold read only its own width would sit here with its words showing.
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

        /// **Which badge stands is the model's value, and it is an input.** Three of the five conditions read the
        /// page in front (`curPage.pageWt` / `pageGraph`), so they are answered here by handing the group a
        /// different page state — no git, no window. What a headless run is still for is the other half: that a real
        /// repository's state reaches the model at all.
        ///
        /// The remaining two (identity and old-git) read `AppBackend` rather than the page, so they cannot be handed
        /// in — that is the shape of the wiring, not a debt to real git.
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

        /// The floor takes them whatever else is true, which is the third way in and the one `old-git-fold`
        /// photographs (規約 §ウィンドウの縁).
        function test_the_window_on_its_floor_takes_the_words() {
            verify(group.stateWordsShown)
            group.windowAtFloor = true
            verify(!group.stateWordsShown)
            verify(group.stateMarkShown)
            compare(group.stateCapW, -1, "nothing here was about the group being short")
        }
    }
}
