import QtQuick
import QtTest
import platitude.ui

// How the tab strip hands its run out, and what a tab drawn under the answer does with it
// (デザイン規約 §ウィンドウの縁 の譲る順).
//
// **The arithmetic and the tab, and nothing between them.** Which widths go in is the font's and the model's business
// and the headless run proves it (`PGG_AUTO_ACT=tab-widths`); what is left once the widths are in hand is a function
// of numbers and a delegate drawn under its answer — both of them here, where every case costs a function call
// instead of a window and one repository per tab. The verb used to walk the counts 1, 6, 8, 9, 12 and 16 to reach the
// states below, and each of those raised an app against real repositories to read six numbers off one report line.
Item {
    id: root
    width: 600
    height: 200

    /// The costs the strip works in, named here rather than measured: what a boundary case is about is the
    /// arithmetic on either side of it, and a cost read off the platform's UI font moves the boundary between
    /// machines — which is what made these cases a window's to answer in the first place. They are the shape of the
    /// real ones (Windows offscreen, rules-refs/app-ui.md), not a claim about any machine's.
    readonly property real padL: 4
    readonly property real roomFull: 17
    readonly property real roomMin: 4
    readonly property real floorW: 68
    readonly property real easeFullW: 72

    TabShare {
        id: share
        padL: root.padL
        markRoomFull: root.roomFull
        markRoomMin: root.roomMin
        runParts: 3
        easeShare: 0.5
        minW: root.floorW
        easeFullW: root.easeFullW
    }

    /// The real one, with the real font behind it — the seam's other side.
    TabMetrics {
        id: metrics
    }

    /// The strip's own model, as much of it as a tab reads: which row is in front.
    QtObject {
        id: tabsModel
        property int currentIndex: -1
    }

    TabItemDelegate {
        id: tab
        index: 0
        tab_id: 1
        title: "platitude-gg"
        copy_path: "/home/someone/src/platitude-gg"
        copy_name: ""
        tabsModel: tabsModel
        metrics: metrics
        bandColor: Theme.bgBase
        stripHeight: 40
        titleCap: 400
        titleMinW: root.floorW
        titleEaseW: 72
        markRoom: root.roomFull
        fadeW: 30
    }

    /// The same tab standing in a linked working copy, so the run naming that copy is drawn after the name
    /// (デザイン規約 §タブの所作). Its own instance: what the run costs is what half these cases are about, and a
    /// property flipped back and forth on the one tab above would leave every case before it reading a different tab.
    TabItemDelegate {
        id: standing
        index: 1
        tab_id: 2
        title: "platitude-gg"
        copy_path: "/home/someone/src/trees/topic"
        copy_name: "topic"
        tabsModel: tabsModel
        metrics: metrics
        bandColor: Theme.bgBase
        stripHeight: 40
        titleCap: 400
        titleMinW: root.floorW
        titleEaseW: 72
        markRoom: root.roomFull
        fadeW: 30
    }

    TestCase {
        name: "TabWidths"
        when: windowShown

        // ---- the run handed out -------------------------------------------------
        //
        // Six names of one width, so what moves between the cases is the run and nothing else. 100 is over
        // `easeFullW`, which puts every name past the length the strip helps along — the easing has a case of its
        // own below, and mixing it into these would hide which rule answered.

        function sixAt(run) {
            return share.settle([100, 100, 100, 100, 100, 100], run)
        }

        /// A strip with no tabs in it still answers, and answers with the whole run: a band that came up empty
        /// hands its names nothing, and the ceiling it would cap one at is the run itself.
        function test_an_empty_strip_hands_out_the_whole_run() {
            const none = share.settle([], 900)
            compare(none.cap, 900)
            compare(none.maxW, 900)
            compare(none.markRoom, root.roomFull, "nothing is standing, so nothing has given the mark's room up")
            compare(none.wantNames, 0, "and it asks for no width on the names' behalf")
        }

        /// One tab has the run to itself — the ceiling is a share of the band, and a band nobody else is standing in
        /// is not cut into three (`TabShare.ceiling`).
        function test_one_tab_is_not_held_to_a_third_of_a_band_it_stands_in_alone() {
            const one = share.settle([64], 900)
            compare(one.maxW, 900)
            compare(one.cap, 900)
            compare(one.markRoom, root.roomFull)
            // Its name is short, so it is eased: half of what it falls short of `easeFullW` by, and that air is
            // what the strip asks for on top of the name and the tab's own furniture.
            compare(one.wantNames, 64 + 4 + root.padL + root.roomFull)
        }

        /// **The mark's room goes first, and the whole of it goes before one name is cut** (規約 §ウィンドウの縁).
        /// Six pixels off the run here, and the cap does not move at all — what pays is the room every tab keeps for
        /// its `✕`, and every tab pays the same.
        function test_the_marks_stand_down_before_a_single_name_is_cut() {
            const ample = sixAt(731)
            compare(ample.markRoom, root.roomFull, "the last run at which the mark keeps all of its room")
            compare(ample.cap, ample.maxW, "and nothing is cut")

            const tighter = sixAt(725)
            compare(tighter.markRoom, 16, "the first pixel the strip takes back is the mark's")
            compare(tighter.cap, tighter.maxW, "still nothing cut")

            const tight = sixAt(653)
            compare(tight.markRoom, root.roomMin, "down to the near step, which is as far as it folds")
            compare(tight.cap, tight.maxW, "and the names have still not been asked")
        }

        /// The boundary the cap is decided at, from both sides and on it: with the marks already folded, 648 is the
        /// run six 100-pixel names fit in exactly.
        function test_the_names_give_way_at_the_run_that_no_longer_holds_them() {
            const fits = sixAt(648)
            compare(fits.markRoom, root.roomMin)
            compare(fits.cap, fits.maxW, "exactly enough, so the ceiling is still what caps them")

            const spare = sixAt(649)
            compare(spare.cap, spare.maxW, "a pixel to spare reads the same")

            const tooTight = sixAt(647)
            compare(tooTight.markRoom, root.roomMin, "the mark has nothing left to give")
            compare(tooTight.cap, 99, "so the names come down, and come down together")
            verify(tooTight.cap < tooTight.maxW, "which is the reading that says the band has no leftover")
        }

        /// And the floor under that, which is where the strip stops cutting and starts scrolling.
        function test_the_cap_stops_at_the_floor_and_does_not_go_under_it() {
            compare(sixAt(462).cap, 69, "a pixel above the floor is still shared out")
            compare(sixAt(456).cap, root.floorW, "and the floor is reached exactly")
            compare(sixAt(455).cap, root.floorW, "past it the answer holds, and the strip scrolls instead")
            compare(sixAt(200).cap, root.floorW, "however far past")
            // **The ceiling is floored at the same width**, and it has to be said separately: the cap is clamped on
            // the way out, so a ceiling that sank under the floor would leave every cap right and only `maxW` — the
            // number the band's leftover is read against — saying the strip may draw a name shorter than the
            // shortest it allows.
            compare(sixAt(200).maxW, root.floorW, "a third of this run is under the floor, so the floor answers")
            compare(share.ceiling(200, 6), root.floorW)
            compare(share.ceiling(900, 6), 300, "and above it the share is what answers")
        }

        /// **A short name keeps its own width, and does not lend the long one its leftover** (同§). The ceiling is
        /// what caps the strip: without it, one tab called `ui` beside one called after a path would let that path
        /// take nearly the whole band.
        function test_a_short_name_beside_a_long_one_keeps_its_width_and_the_ceiling_caps_the_long_one() {
            const mixed = share.settle([20, 500], 900)
            compare(mixed.maxW, 450, "two tabs, so the ceiling is half the band")
            compare(mixed.cap, 450, "and the long name is held to it though 800 pixels stood unclaimed")
            compare(mixed.markRoom, root.roomFull)

            // Crowded, the short ones are still not the ones that give way: the cap comes down to what the long
            // name is cut at, and a name already under it is untouched.
            const crowded = share.settle([20, 20, 500], 400)
            verify(crowded.cap > 20, "the cap never cuts a name that was already inside its share")
            verify(crowded.cap < 500, "and the long one is the one that gave way")
        }

        /// The easing, which is the other thing a name shorter than `easeFullW` gets: half of what it falls short by.
        /// **It is spent whatever the cap comes to**, so it is in the width the strip asks for.
        function test_a_short_name_is_given_half_of_what_it_falls_short_by() {
            compare(share.ease(40, 400, root.easeFullW), 16)
            compare(share.ease(72, 400, root.easeFullW), 0, "at the length it stops being helped along")
            compare(share.ease(200, 400, root.easeFullW), 0, "and past it")
            compare(share.ease(40, 50, 50), 5, "never past the cap — air there is paid for in letters")
        }

        /// Where that air is set down: half and half while the mark keeps its room, and onto the mark's side as the
        /// strip takes that room back (`TabShare.easeRight`).
        function test_the_eased_air_moves_to_the_marks_side_as_its_room_is_taken_back() {
            compare(share.easeRight(20, root.roomFull), 10, "half and half while the mark is whole")
            compare(share.easeRight(20, 7), 10, "the room it lost is inside that half, so nothing moves yet")
            compare(share.easeRight(20, root.roomMin), 13, "past it the air covers what the mark gave up")
            compare(share.easeRight(4, root.roomMin), 4, "and never more than there is")
        }

        /// Walking one band down from ample to short: **the run is given up in one order and never taken back**.
        /// The sweep is the claim the cases above are points on — a rule that folded the names first, or let a cap
        /// grow as the band narrowed, passes none of it.
        function test_the_band_gives_things_up_in_one_order_all_the_way_down() {
            let lastCap = Number.MAX_VALUE
            let lastRoom = root.roomFull
            let cutBelow = 0
            for (let run = 900; run >= 150; run -= 1) {
                const at = sixAt(run)
                verify(at.markRoom <= lastRoom, "the mark's room never comes back: run " + run)
                verify(at.cap <= lastCap, "and no name grows as the band narrows: run " + run)
                verify(at.cap >= root.floorW, "the floor holds all the way down: run " + run)
                verify(at.maxW >= root.floorW, "and the ceiling never sinks under it: run " + run)
                verify(at.cap <= at.maxW, "the ceiling caps the cap: run " + run)
                if (at.cap < at.maxW) {
                    compare(at.markRoom, root.roomMin,
                            "no name is cut while the mark still has room to give: run " + run)
                    cutBelow = Math.max(cutBelow, run)
                }
                lastCap = at.cap
                lastRoom = at.markRoom
            }
            compare(cutBelow, 647, "and the sweep reached the run the names start giving way at")
        }

        /// The seam: the real object, with the real font's answers pushed into the arithmetic. What this fixes is
        /// the wiring — a `settle` that never priced the share would answer off a floor of zero, which no boundary
        /// case above can see.
        function test_the_real_metrics_hand_the_fonts_answers_to_the_arithmetic() {
            const floorW = metrics.titleMinW()
            verify(floorW > 0, "the font answers with a floor")
            const settled = metrics.settle([1000, 1000, 1000], 120)
            compare(settled.minW, floorW, "and that floor is the one the arithmetic was given")
            compare(settled.cap, floorW, "a band this narrow is held at it")
            compare(metrics.share.easeFullW, metrics.titleEaseFullW())
        }

        // ---- the tab drawn under the answer -------------------------------------

        function init() {
            tabsModel.currentIndex = -1
            tab.title = "platitude-gg"
            tab.titleCap = 400
            // The tab's own costs are the real object's here — what is being asked below is a tab drawn under an
            // answer, not the answer.
            tab.markRoom = metrics.markRoomFull
            tab.titleEaseW = metrics.titleEaseFullW()
        }

        /// The tab is drawn as wide as what it is called, up to the cap, plus what it costs either side of the name.
        /// The strip sizes its run off the same expression, so the two agreeing is what keeps a settled strip from
        /// scrolling by the pixels they disagree about.
        function test_a_tab_is_as_wide_as_its_name_and_what_stands_beside_it() {
            const name = nameIn(tab)
            const beside = metrics.tabPadL + tab.markRoom
            verify(name.implicitWidth < tab.titleCap, "the cap is over this name, so the name is the width")
            compare(tab.width, name.implicitWidth + beside + tab.titleEase)

            tab.titleCap = 60
            verify(name.implicitWidth > 60, "and now under it")
            compare(tab.width, 60 + beside + tab.titleEase,
                    "so the tab comes down to the cap")

            // **A short name's tab is wider than the name**, by the air the strip eases it with — the reading that
            // separates a tab sized off its name from one sized off name-plus-air. `titleEase` is 0 for every name
            // at or over the length the strip stops helping along, so a case on a long name alone is blind to it.
            tab.title = "ui"
            tab.titleCap = 400
            verify(tab.titleEase > 0, "a name this short is given air on top of itself")
            compare(tab.width, name.implicitWidth + beside + tab.titleEase)
            verify(tab.width > name.implicitWidth + beside, "and the tab is that much wider than its name")
        }

        /// **Something of the name is always drawn.** A tab cut to the mark alone is the one failure the strip's own
        /// numbers cannot say — the tab is as wide as its name, so every width reads exactly as it should, and the
        /// picture is a narrow tab with a mark in it, which is what a short name looks like too.
        function test_the_name_survives_every_cap_down_to_the_floor() {
            const floorW = metrics.titleMinW()
            for (let cap = 400; cap >= floorW; cap -= 2) {
                tab.titleCap = cap
                verify(tab.nameKept, "nothing of the name is drawn at cap " + cap)
            }
            tab.titleCap = floorW
            verify(tab.nameKept, "and the floor itself still says something")
        }

        /// Cut in the middle, with both ends kept: a repository is told apart by the end of its name as much as by
        /// its start (デザイン規約 §タブの所作).
        function test_a_name_over_its_cap_is_cut_in_the_middle_and_keeps_both_ends() {
            const name = nameIn(tab)
            verify(!name.cutting, "room to spare, so nothing is taken out")

            tab.titleCap = Math.floor(name.implicitWidth / 2)
            verify(name.cutting, "under the cap the name is cut")
            verify(name.headText.length > 0, "and kept at the start")
            verify(name.tailText.length > 0, "and at the end")
            verify(name.headText.length + name.tailText.length < tab.title.length,
                   "with something actually taken out")
            verify(name.inkWidth <= tab.width, "and none of it drawn past the tab")
        }

        /// **A tab is as wide as its name, so writing the name moves the column the name is cut against** — from
        /// inside the very pass that is deciding the cut (rules-refs/app-ui.md `CutName`). The nested pass settles
        /// the new name in the column it now has; a pass that had read the old column first lays its own answer back
        /// over that one, and what is left is a name cut for a box narrower than the one it is drawn in — the mark
        /// standing alone where the whole name fits. Coming to the front is the same move by another road: the front
        /// tab's name is set heavier, which widens it, which moves the column.
        ///
        /// **Neither the widths nor the picture says this happened**: the tab is the width it should be, and a tab
        /// with a mark and no name in it looks like a short name in a narrow tab.
        function test_a_name_is_cut_against_the_column_it_ends_up_in_not_the_one_it_replaced() {
            tab.titleCap = 400
            tab.title = "ui"
            const name = nameIn(tab)
            const narrow = tab.width
            verify(!name.cutting, "a short name in a cap this wide is not cut")

            tab.title = "platitude-gg"
            verify(tab.width > narrow, "the tab grew with the name, which is the column moving mid-pass")
            verify(!name.cutting, "and the name is whole in the column it grew into")
            verify(tab.nameKept)

            tabsModel.currentIndex = 0
            verify(tab.current, "in front, where the name is set heavier")
            verify(tab.nameKept)
            tabsModel.currentIndex = -1
            verify(tab.nameKept)
        }

        /// The fade is the strip saying the name has run on under the mark, and it is out exactly while that is
        /// true: the room the mark gave up, less what the name's own eased air already covers.
        function test_the_name_goes_quiet_only_once_it_reaches_under_the_mark() {
            tab.titleCap = 400
            verify(!tab.nameUnderMark, "the mark keeps its whole room, so nothing is under it")

            tab.markRoom = metrics.markRoomMin
            verify(tab.nameUnderMark, "the mark stands over the name once its room is taken back")

            tab.title = "ui"
            verify(!tab.nameUnderMark,
                   "a short name spends its eased air on that side and still stops short of the mark")
        }

        // ---- the copy a tab is standing in --------------------------------------
        //
        // The second run a tab draws, and the order the cap is spent in: the repository's name first, the copy's run
        // out of what is left, and that run gone whole before a letter of the name is cut
        // (デザイン規約 §ウィンドウの縁 の譲る順 2 段目).

        /// The arithmetic on its own, where every boundary is a number: 100 of name, 40 of run, 30 the least the run
        /// is worth drawing at.
        function test_the_cap_is_spent_on_the_name_first_and_the_copy_takes_what_is_left() {
            split(200, 100, 40, "room for both, so both are whole")
            split(140, 100, 40, "and exactly the room for both")
            split(135, 100, 35, "short of it, the run gives way and the name does not")
            split(130, 100, 30, "down to the least it is worth drawing at")
            split(129, 100, 0, "under that the whole run goes, the mark with it")
            split(80, 80, 0, "and only then is the name cut")
        }

        /// A copy whose name is shorter than that floor is all or nothing: cutting `b` is cutting nothing, and a
        /// floor above the run's own width would drop a run that fits (`TabTreeMark.floorWidth`).
        function test_a_run_shorter_than_the_floor_is_drawn_whole_or_not_at_all() {
            const fits = share.splitName(100, 20, 20, 120)
            compare(fits.titleW, 100)
            compare(fits.treeW, 20, "it fits, so it is drawn")
            const cramped = share.splitName(100, 20, 20, 119)
            compare(cramped.titleW, 100)
            compare(cramped.treeW, 0, "and a pixel short it is gone")
        }

        /// One cap spent between a 100-wide name and a 40-wide run worth drawing down to 30.
        function split(cap, titleW, treeW, why) {
            const got = share.splitName(100, 40, 30, cap)
            compare(got.titleW, titleW, why)
            compare(got.treeW, treeW, why)
        }

        /// A tab standing in the repository's own copy has no run to draw and is the width it always was.
        function test_a_tab_in_the_repositorys_own_copy_draws_nothing_after_its_name() {
            const name = nameIn(tab)
            compare(tab.treeW, 0)
            compare(tab.width, name.implicitWidth + metrics.tabPadL + tab.markRoom + tab.titleEase)
        }

        /// One standing in a linked copy is wider by that run, and the run is what a crowded strip takes back first:
        /// the name is still whole at a cap that has taken the whole of it away.
        function test_the_run_naming_the_copy_is_given_up_before_the_name_is_cut() {
            const name = nameIn(standing)
            standing.titleCap = 400
            verify(standing.treeW > 0, "room to spare, so the copy is named")
            compare(standing.width,
                    name.implicitWidth + standing.treeW + metrics.tabPadL + standing.markRoom + standing.titleEase)
            const named = standing.width

            standing.titleCap = Math.ceil(name.implicitWidth)
            compare(standing.treeW, 0, "a cap that holds the name alone draws no run")
            verify(standing.width < named, "and the tab comes in by the run it gave up")
            verify(!name.cutting, "and the name is whole in it")
            verify(standing.nameKept)

            standing.titleCap = Math.floor(name.implicitWidth / 2)
            compare(standing.treeW, 0, "still none once the name itself is being cut")
            verify(name.cutting)
            verify(standing.nameKept, "and something of the name is drawn at every cap")
            standing.titleCap = 400
        }

        /// The one child of a tab that draws its name, found by what it is rather than by name.
        function nameIn(item) {
            for (let i = 0; i < item.children.length; i++) {
                const child = item.children[i]
                if (child.headText !== undefined && child.cutAt !== undefined)
                    return child
            }
            fail("the tab draws no name")
            return null
        }
    }
}
