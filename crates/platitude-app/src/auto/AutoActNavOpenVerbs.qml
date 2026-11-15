pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui.hover

/// The left menu's rows answering a hand: the tooltip a row puts out, the lines a row opens under itself, and
/// where those lines lead.
// An `Item` only because `QtObject` has no default property to hold the timers below.
Item {
    id: acts

    /// `var` because naming its type would be a cycle: the driver is the file that builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var graphModel: driver.graphModel
    readonly property var branchesModel: driver.branchesModel
    readonly property var tagsModel: driver.tagsModel
    readonly property var graphPane: driver.graphPane
    readonly property var sidebarPane: driver.sidebarPane
    readonly property var navProbe: driver.navProbe
    readonly property var refMenu: driver.refMenu
    readonly property var refTagCard: driver.refTagCard

    /// Runs `act` if it is this family's and says whether it was; `AutoActDriver` asks each family in turn, so a verb
    /// belongs to one.
    function run(act, arg) {
        if (act === "nav-tip" || act === "nav-open") {
            // `<section>:<row>`, or `head` for the current branch's sticky stand-in (`navTipTimer`).
            //
            // `nav-open`: the lines (`NavRowFacts`) are part of the row, not a popup, so app.png holds them and no
            // scene.png is made. Aimed at a row that opens nothing — a folder row (a folder is no ref) or a remote's
            // own row (`remote:0`) — it waits out the watchdog. On a WORKTREES row the path's tip comes up as well and
            // is in the picture (`tip=true text=<path>`; the claim stays on `says=`); its padlocks and `!` are
            // app.png's, and the sidebar's 180 floor for the width is set up as for `nav-tip`.
            navTipTimer.begin(arg)
        } else if (act === "nav-open-foot") {
            // The same rest on the last row of an overflowing section scrolled to its end, with nowhere to open into:
            // the list has to move (`NavList.revealOpenRow`). `away` then leaves, and the list gives back what it took;
            // `bar` takes the list's bar instead, which closes the row and gives it back the same way (デザイン規約
            // §hover のツールチップ「スクロールバーを掴んだら、hover で開いたものは閉じる」). Run on `--preset stack`,
            // the one preset whose BRANCHES has more rows than it shows (30 branches): elsewhere the list has nowhere
            // to scroll and the fields read nothing.
            navTipTimer.begin("branch")
        } else if (act === "nav-drag-open") {
            // A press on a closed row's line that starts to move: the lines come out at once and the drag carries on
            // into them (デザイン規約 §左メニューの所作).
            const spot = acts.rowArg(arg, "branch", 2)
            dragOpenTimer.kind = spot.kind
            dragOpenTimer.row = spot.row
            dragOpenTimer.held = spot.filter
            dragOpenTimer.order = dragOpenTimer.held !== "" ? ["filtered", "dragged"] : ["dragged"]
            dragOpenTimer.start()
        } else if (act === "nav-open-tip") {
            // The rest that opens a row, held on: a rest later its supplement follows from the same unmoved hand.
            const spot = acts.rowArg(arg, "worktree", 3)
            openTipTimer.kind = spot.kind
            openTipTimer.row = spot.row
            openTipTimer.start()
        } else if (act === "nav-open-held") {
            // The name box first, then a hand on a row: nothing may open — it would move the rows under the box.
            sidebarPane.beginRename("branch", navProbe.tipNameAt("branch", 2), navProbe.tipNameAt("branch", 2))
            openHeldTimer.start()
        } else if (act === "nav-open-then") {
            // The open row, then the hand's next move: `away` leaves it, `edit` opens the name box on that row, `menu`
            // opens a menu elsewhere, `filter` types a filter that takes the row out, `rightclick-name` raises the
            // row's menu on its own line, and `rightclick` / `sweep` / `tap` act on the open lines themselves.
            // `rightclick` / `rightclick-name` are read once the menu stands and the layout is still, with no hand:
            // that the row stays open after the menu takes the hand is beyond a hand-less run (the real pointer's
            // `tst_openrowholdshand`). `sweep` takes the first line's words, not the name — the name sits in the row's
            // seat, which `nav-drag-open` takes.
            navTipTimer.begin("branch:2")
        } else if (act === "nav-follow" || act === "nav-follow-lit") {
            // `<section>:<row>[:<filter>]`: the open row's first line that goes somewhere, pressed (the graph goes
            // there and the panel stays — デザイン規約 §左メニューの所作「段の名前は、その名前が立つコミットへの道」),
            // or with `-lit` rested on. The argument is required: a row that never opens waits out the ceiling.
            followTimer.begin(arg, act === "nav-follow-lit")
        } else if (act === "nav-open-tag") {
            // Named, not numbered: tags sort newest-first with remote-only names last, so the row a name lands on is
            // the preset's business.
            navOpenTagTimer.begin(arg, -1)
        } else if (act === "tag-line-menu") {
            // `<tag>:<line>`: the same row opened, then the menu asked for on that line of it — a carrier's line names
            // its remote on its own, so the TAG card acts on it (`NavRowFacts.lineMenu`).
            const at = arg.lastIndexOf(":")
            navOpenTagTimer.begin(arg.slice(0, at), Number(arg.slice(at + 1)))
        } else {
            return false
        }
        return true
    }
    /// `<section>:<row>[:<filter>]` as `{ kind, row, filter }`, with the verb's own `kind` for an empty section and
    /// its own `row` for none named.
    function rowArg(arg, kind, row) {
        const parts = ("" + arg).split(":")
        return { "kind": parts[0] === "" ? kind : parts[0], "row": parts.length > 1 ? Number(parts[1]) : row,
                 "filter": parts.length > 2 ? parts[2] : "" }
    }
    // No rest first: the gesture skips it (`tipDelayMs`), and a run that rested on the row would be reading the other
    // path. What the drag takes is the full name in the row's own seat (`NameCell`), not the lines' `SweepPad`; the
    // default `branch:2` is `main`, a name the row does not cut — a name that fits is taken as well.
    PageStage {
        id: dragOpenTimer
        name: "nav_drag_open"
        order: ["dragged"]
        property string kind: "branch"
        property int row: 2
        /// A filter typed before the hand arrives, for a row that is folded away without one.
        property string held: ""
        onTriggered: {
            const op = dragOpenTimer
            // The filter, typed again until it takes; then the row, pressed again until it is there to press: a miss
            // reads like a row that refused to open (`nav-open` の同じ歩き).
            if (op.held !== "" && op.at("start")) {
                op.input("filtered", () => navProbe.typeFilter(op.held) === op.held)
                return
            }
            if (op.at("start") || op.at("filtered")) {
                if (!Awaited.all("nav_drag_open_row", { "row": navProbe.tipNameAt(op.kind, op.row) !== "" }))
                    return
                op.input("dragged", () => {
                    const name = navProbe.tipNameAt(op.kind, op.row)
                    if (!navProbe.dragRow(op.kind, op.row))
                        return false
                    op.witness("rowName", name)
                })
                return
            }
            // The beat after the press, on the page it leaves.
            op.complete({
                "name": "nav_drag_open_dragged",
                "when": () => ({ "page": PageSettled.settled(page) }),
                "report": () => {
                    const took = navProbe.rowNameTook(op.kind, op.row)
                    return "nav_drag_open section=" + op.kind
                        + " row=" + op.row + " name=" + op.witnessed("rowName")
                        + " open=" + navProbe.rowFactsOpen
                        + " caret=" + navProbe.rowNameCaret(op.kind, op.row)
                        + " copied=" + (took !== "")
                        // A drag that took the name is not a click — a second click on the row opens a name box
                        // (`NavItemDelegate.lineClicked`).
                        + " clicked=" + (sidebarPane.activeKey !== "")
                        + " took=" + took
                } })
        }
    }
    // _PGG_AUTO_ACT=nav-open-tip: the worktree's path the open row keeps for the hand that opened it — the whole path,
    // which the lines never carry (デザイン規約 §左メニューの所作). Run on `--preset worktrees` (row 3 by default). The
    // picture shows the words (overlay.png / scene.png); `same=` holds them against the path the open row asks for
    // (`NavProbe.factsSays`), so a tip drawn from anywhere else fails. A rest on the lines themselves is the real
    // pointer's (`tst_factstipsteal.qml`).
    PageStage {
        id: openTipTimer
        name: "nav_open_tip"
        property string kind: "worktree"
        property int row: 3
        /// The row open, and its supplement up.
        function shown() {
            return { "open": navProbe.rowFactsOpen, "tip": Tips.box.visible }
        }
        onTriggered: {
            const op = openTipTimer
            // Re-applied every beat until it answers (`nav-open` の同じ歩き).
            if (op.at("start") && !op.input("rested", () => {
                    navProbe.pointTipAt(op.kind, op.row)
                    const shown = op.shown()
                    return shown.open && shown.tip
                }))
                return
            op.complete({
                "name": "nav_open_tip",
                "when": () => op.shown(),
                "report": () => {
                    const tip = Tips.box
                    return "nav_open_tip section=" + op.kind
                        + " row=" + op.row
                        + " open=" + navProbe.rowFactsOpen
                        + " tip=" + tip.visible
                        // An empty tip answers `tip=true` as well as a full one (`NavRowFacts.said`); weighed in the
                        // run, since the path is this machine's own.
                        + " same=" + (tip.text === navProbe.factsSays())
                        // Last, since a path carries anything.
                        + " says=" + navProbe.rowFactsWords()
                        + " text=" + tip.text
                } })
        }
    }
    // The row still has to be the one under the hand — a run that never reached it would answer the same way.
    PageStage {
        id: openHeldTimer
        name: "nav_open_held"
        /// The row there under the hand, and the box still standing.
        function held() {
            return { "row": navProbe.tipNameAt("branch", 2) !== "", "box": sidebarPane.editKey !== "" }
        }
        onTriggered: {
            const op = openHeldTimer
            // The stand-in opens outright where a hand would sit out a rest, so a row that was going to open is open
            // by the time the completion reads it. Re-applied every beat until the row is there.
            if (op.at("start") && !op.input("rested", () => {
                    navProbe.pointTipAt("branch", 2)
                    const held = op.held()
                    return held.row && held.box
                }))
                return
            op.complete({
                "name": "nav_open_held",
                "when": () => op.held(),
                "report": () => "nav_open_held row=2 name=" + navProbe.tipNameAt("branch", 2)
                                + " box=true open=" + navProbe.rowFactsOpen
                                + " lit=" + navProbe.rowWashLit("branch", 2) })
        }
    }
    // _PGG_AUTO_ACT=nav-follow / nav-follow-lit. The landing is read off the graph, not the press: the page's selection
    // is only the click's bookkeeping, so `landed=` is the graph's row for that commit, lit and laid out.
    //
    // No `order`: a row whose lines go nowhere completes where it opened, one with a line that goes somewhere after
    // aiming at it — which of the two is the repository's to say.
    //
    // A line on the row's own commit is part of the row and is not pressed (`NavFacts.joined`), so the line taken is
    // the next that goes somewhere. On `--preset tracked-elsewhere`: `remote:1:topic-a` goes to the local branch (it
    // and the worktree holding it share one band), `branch:1` skips the worktree's line to the upstream, and
    // `worktree:1` skips the branch's line to the upstream.
    PageStage {
        id: followTimer
        name: "nav_follow"
        property string kind: "remote"
        property int row: 0
        property string filter: ""
        property bool lit: false
        // Witnessed: `rowName`, the row's name read as it opened; `line` and `to`, the first of the open lines that
        // goes somewhere and where, read as the hand aims at it once the lines stand still.
        function begin(arg, lit) {
            const spot = acts.rowArg(arg, "remote", 0)
            followTimer.kind = spot.kind
            followTimer.row = spot.row
            followTimer.filter = spot.filter
            followTimer.lit = lit
            followTimer.start()
        }
        function said(line) {
            return " section=" + followTimer.kind + " row=" + followTimer.row + " line=" + line
        }
        /// The open row's supplement is up, where it keeps one (a WORKTREES row's path, `NavRowFacts.says`): it comes
        /// up a rest after the lines, and every end below waits it out, or the picture holds the box or not by the
        /// machine's pace. A term of each end's `when`, so a run stalled here names it.
        function tipRested() {
            const lines = sidebarPane.openFactsItem()
            return lines === null || lines.says === "" || lines.tipShown
        }
        /// Where the line pressed went: the graph's row for that commit, selected and laid out — a commit the walk
        /// never reached is an answer; a row the model has but the view has not laid out is waited for — and the
        /// supplement rested.
        function landed() {
            const want = followTimer.witnessed("to").oid
            const landing = graphModel.rowOf(want)
            return { "selected": page.selectedOid === want,
                     "card": driver.cardSettled,
                     "laid": landing < 0 || !!graphPane.view.itemAtIndex(landing),
                     "tip": followTimer.tipRested() }
        }
        onTriggered: {
            const op = followTimer
            // A filter, once: the way a leaf under a folded folder is brought into the list.
            if (op.filter !== "" && op.at("start")) {
                // The page's rows in before the filter rebuilds them (a section's list stands only once peeked, so
                // the list itself is nothing to wait for here).
                if (!Awaited.all("nav_follow_filter", { "page": PageSettled.settled(page) }))
                    return
                op.input("filtered", () => {
                    navProbe.typeFilter(op.filter)
                })
                return
            }
            // The rest, every beat until the row is open (`nav-open` の同じ歩き).
            if (op.at("start") || op.at("filtered")) {
                if (!op.input("opened", () => {
                        navProbe.pointTipAt(op.kind, op.row)
                        if (!navProbe.rowFactsOpen)
                            return false
                        op.witness("rowName", navProbe.tipNameAt(op.kind, op.row))
                    }))
                    Awaited.all("nav_follow_open", { "open": navProbe.rowFactsOpen })
                return
            }
            // Nothing is aimed until the lines stop moving: laid out a pass after they are built, a press worked out
            // from a line still at its origin lands on the row's mark.
            if (op.at("opened")) {
                if (!Awaited.all("nav_follow_geom", { "still": op.still("nav_follow_geom",
                                                                        sidebarPane.rowFactsGeom()) }))
                    return
                if (navProbe.factsGoesTo(navProbe.factsFirstGoing()) === null) {
                    // Lines that go nowhere are this row's answer — waiting on for a landing would spend the ceiling
                    // and take no picture.
                    op.complete({
                        "name": op.lit ? "nav_follow_lit" : "nav_follow",
                        "when": () => ({ "tip": op.tipRested() }),
                        "report": () => (op.lit ? "nav_follow_lit" : "nav_follow") + op.said(navProbe.factsFirstGoing())
                                        + " followed=false name=" + op.witnessed("rowName") })
                    return
                }
                op.input("aimed", () => {
                    const line = op.witness("line", navProbe.factsFirstGoing())
                    op.witness("to", navProbe.factsGoesTo(line))
                    if (op.lit)
                        navProbe.pointFactsWords(line, true)
                    else
                        navProbe.followFactsLine(line)
                })
                return
            }
            if (op.lit) {
                // `aimed=` is the band as drawn, `hand=` the cursor the area took — two parts, so either alone splits.
                op.complete({
                    "name": "nav_follow_lit",
                    "when": () => ({ "tip": op.tipRested() }),
                    "report": () => "nav_follow_lit" + op.said(op.witnessed("line"))
                                    + " aimed=" + navProbe.factsLineAimed(op.witnessed("line"))
                                    + " hand=" + navProbe.factsHand()
                                    + " open=" + navProbe.rowFactsOpen
                                    + " to=" + op.witnessed("to").key + " name=" + op.witnessed("rowName") })
                return
            }
            op.complete({
                "name": "nav_follow_landed",
                "when": () => op.landed(),
                "report": () => {
                    const want = op.witnessed("to").oid
                    const at = graphModel.rowOf(want)
                    const item = at >= 0 ? graphPane.view.itemAtIndex(at) : null
                    return "nav_follow" + op.said(op.witnessed("line"))
                           + " followed=true"
                           + " landed=" + (!!item && item.oid_hex === want && item.selected)
                           // The panel stays: the click mark is on the open row, not the one the line named
                           // (デザイン規約「行き先はグラフ」).
                           + " marked=" + (sidebarPane.activeKey === sidebarPane.openKey)
                           + " box=" + (sidebarPane.editKey !== "")
                           + " open=" + navProbe.rowFactsOpen
                           + " to=" + op.witnessed("to").key + " at=" + at + " name=" + op.witnessed("rowName")
                } })
        }
    }
    // _PGG_AUTO_ACT=nav-tip / nav-open / nav-open-then / nav-open-foot. Every run first opens a control row — the first
    // WORKTREES row, the one section every repository has — so an empty overlay still says the pointer arrived
    // somewhere. The hand leaves it on the beat it opened, before its supplement could show, so it raises nothing.
    //
    // `nav-tip <section>:<row>[:<filter>]` (`branch` / `remote` / `stash` / `tag`) reads a row that answers with the
    // shared tooltip: stash leaves, every section's fold parents (a folder row's tip is its full path), and REMOTES'
    // own remote row (`remote:0`, the one row whose tip is a sentence about its role). Leaves of BRANCHES / REMOTES /
    // WORKTREES, the pinned current branch and tag leaves open under themselves instead (`nav-open` / `nav-open-tag`;
    // a worktree's states, locked or folder gone, are pictured there) and answer `wants=false tip=false`, so the
    // registered lines come from rows that do not open. Those claim `lit=true wants=true tip=true` and
    // `open=false aside=true` (`aside=`: the tip stands beside the target row, over neither neighbour). `lit=` says
    // only that the injected pointer arrived; a run claiming `tip=true` is what says the shared instance is alive. The
    // row wears its hover wash too, so the picture shows where the pointer was; the tip is overlay.png's. `branch:0
    // --preset basic` (`feature`) is the design picture. `branch:2 --preset nested` pins `text=team/backend`: a
    // top-level folder spells its path and its shown name alike, so a tip built from the shown name instead of the fold
    // key passes there. Widths are checked at the sidebar's floor — `sidebar_width = 180` in the `state.toml` of
    // `--config-dir` (core takes anything under 180 as out of range and falls back to 260). The folded rail's peek
    // (`nav-peek worktree`) draws the WORKTREES rows with the same delegate (`NavItemDelegate`, `SectionPeekPopup`),
    // so a worktree row's marks change in both pictures; read the two together.
    PageStage {
        id: navTipTimer
        name: "nav_tip"
        property string kind: "branch"
        property int row: 0
        property bool head: false
        /// Whether the run reads the lines a row opens (_PGG_AUTO_ACT=nav-open) instead of the shared tooltip, and
        /// what the hand does once the row is open (_PGG_AUTO_ACT=nav-open-then / nav-open-foot: the argument).
        readonly property bool opens: Harness.autoAct === "nav-open" || Harness.autoAct === "nav-open-then"
                                      || Harness.autoAct === "nav-open-foot"
        readonly property string then: Harness.autoAct === "nav-open-then" || Harness.autoAct === "nav-open-foot"
                                       ? Harness.autoActArg : ""
        /// For `head`: a filter the current branch does not answer to, putting the stand-in on screen with no row
        /// anywhere (`HeadPinRow.seated`), at the list's head. The presets this argument runs on never overflow a
        /// section, so the half where the list scrolled out from under the row is `nav-pin-edge`'s. `head:topic
        /// --preset basic` keeps `feature/topic-a` in the list, so the stand-in has its seat; a filter no row answers
        /// to closes the section and cuts the stand-in to the 1px left.
        property string hide: ""
        /// The other way in: the BRANCHES folder row to click shut over the current branch's row (`NavProbe.clickRow`);
        /// -1 for none. The only side that shows where the seat is: the stand-in sits right under the folder shut
        /// (`head::1 --preset nested`: row 1 is `team`, over HEAD's `team/backend/api/fix-auth`; the same preset's
        /// `head:main` is the filter side, the tree flattened and the seat at the list's head). A fold that does not
        /// close over the current branch (`team/web`) leaves its row in the list, no stand-in comes, and the run waits
        /// out the ceiling — that case is `view_tests`' `the_stand_in_follows_whichever_fold_closed_over_it`.
        property int fold: -1
        /// Whether the row rested on is the last of its section, the list taken to its end first
        /// (_PGG_AUTO_ACT=nav-open-foot). Scrolled once — not re-applied under a hand that has arrived.
        readonly property bool foot: Harness.autoAct === "nav-open-foot"
        /// Whether the hand moves on once the row is open (`afterOpen`).
        readonly property bool moves: navTipTimer.opens && navTipTimer.then !== ""
        // Witnessed: `lit`, whether the control row was open as the hand left it; at the foot, `row`, the section's
        // last row, and `restY`, where the list stood still before the hand arrived — the run's own note, held against
        // what closing gives back rather than the list's (`NavList.openRestY`); and as the hand moves on, `rested`, the
        // name rested on, and for `bar`, `shut` (`NavProbe.afterOpen`).
        function begin(arg) {
            const parts = ("" + arg).split(":")
            navTipTimer.head = parts[0] === "head"
            const spot = acts.rowArg(arg, "branch", 0)
            navTipTimer.kind = navTipTimer.head ? "branch" : spot.kind
            // The filter takes the stand-in's own row out of the list, or brings a leaf under a folded folder into it
            // (a filtered row stands under its full name — `SidebarFilterRow`). REMOTES keeps the remote's own row
            // (`origin`) as row 0 under a filter too, so its leaves count from 1.
            navTipTimer.hide = navTipTimer.head ? (parts.length > 1 ? parts[1] : "") : spot.filter
            // `head:<filter>:<row>` — the folder to shut over the current branch; the filter is then usually empty.
            navTipTimer.fold = navTipTimer.head && parts.length > 2 ? Number(parts[2]) : -1
            navTipTimer.row = navTipTimer.head ? 0 : spot.row
            // The control row first; then the fold or the scroll asked for; then the rest on the row; then, where one
            // is asked, the hand's move.
            navTipTimer.order = ["lit"].concat(navTipTimer.fold >= 0 ? ["folded"] : [],
                                               navTipTimer.foot ? ["footed"] : [],
                                               ["rested"],
                                               navTipTimer.moves ? ["acted"] : [])
            navTipTimer.start()
        }
        function target() {
            if (navTipTimer.head)
                return branchesModel.headRow
            return navTipTimer.foot ? navTipTimer.witnessed("row") : navTipTimer.row
        }
        /// The name under the hand as it is read — for the stand-in, which has no tip, the current branch's.
        function nameNow() {
            return navTipTimer.head ? branchesModel.headName
                                    : navProbe.tipNameAt(navTipTimer.kind, navTipTimer.target())
        }
        /// The name rested on: read as the hand moved on, where it does — a filter may take the row out after — else
        /// as it is read.
        function restedName() {
            return navTipTimer.moves ? navTipTimer.witnessed("rested") : navTipTimer.nameNow()
        }
        /// The rest landed. An opening row is read on its own lines; elsewhere a row with words waits for the shared
        /// tip to be up, one without for the control's tip to have left.
        function restTerms() {
            const words = navTipTimer.head ? navProbe.headPinWords
                                           : navProbe.tipWordsAt(navTipTimer.kind, navTipTimer.target())
            return { "row": navTipTimer.nameNow() !== "",
                     "pin": !navTipTimer.head || navProbe.headPinLit,
                     "open": !navTipTimer.opens || navProbe.rowFactsOpen,
                     "tip": navTipTimer.opens || (words !== "") === Tips.box.visible }
        }
        function restHolds() {
            const terms = navTipTimer.restTerms()
            return terms.row && terms.pin && terms.open && terms.tip
        }
        /// What the completion stands on: the rest, until the hand moves on — nothing is waited on after, since what
        /// opened is inside the row and goes with the pointer — and the supplement (a worktree's path) answered either
        /// way: it comes up a rest after the lines, and read in between the picture has it half out on one run and
        /// not at all on the next. And the page: a row the hand pressed selects its commit, whose read is the page's.
        function landed() {
            const moved = navTipTimer.moves
            const rest = navTipTimer.restTerms()
            return { "row": moved || rest.row, "pin": moved || rest.pin, "open": moved || rest.open,
                     "tip": moved || rest.tip,
                     "said": !navTipTimer.opens || moved || (navProbe.factsSays() !== "") === Tips.box.visible,
                     "page": PageSettled.settled(page) }
        }
        /// The list is the subject at the foot, so it has a line of its own: `shown=` is the row inside what the list
        /// shows, `back=` the list's position against the run's note from before the hand arrived.
        function footLine() {
            const at = navProbe.listContentY(navTipTimer.kind)
            const restY = navTipTimer.witnessed("restY")
            return "nav_open_foot what=" + navTipTimer.then
                + " row=" + navTipTimer.target() + " name=" + navTipTimer.restedName()
                + " open=" + navProbe.rowFactsOpen
                + " shown=" + navProbe.rowFactsShown()
                + " back=" + (Math.round(at) === Math.round(restY))
                // `bar`: closed at the grab itself, not by the run's ask after it (`NavProbe.afterOpen`).
                + (navTipTimer.then === "bar" ? " shut=" + navTipTimer.witnessed("shut") : "")
                + " rest=" + Math.round(restY) + " at=" + Math.round(at)
                + " seat=" + sidebarPane.rowFactsGeom()
        }
        function thenLine() {
            const target = navTipTimer.target()
            return "nav_open_then what=" + navTipTimer.then
                + " row=" + target + " name=" + navTipTimer.restedName()
                + " open=" + navProbe.rowFactsOpen
                + " lit=" + navProbe.rowWashLit(navTipTimer.kind, target)
                + " box=" + (sidebarPane.editKey !== "")
                + " menu=" + sidebarPane.menuOpen
                + " rows=" + navProbe.listOf(navTipTimer.kind).count
                // A drag takes words and the row hears nothing; a press that never moved is the row's
                // (`NavRowFacts.handClicked`).
                + " caret=" + navProbe.factsCaret()
                + " copied=" + (navProbe.factsTook() !== "")
                + " clicked=" + (sidebarPane.activeKey !== "")
                // A press in a line's band leaves the row the click mark too (`SidebarRowGestures.followLine`),
                // so the commit the graph went to tells the two apart, not the key.
                + " own=" + (page.selectedOid === navProbe.listOf(navTipTimer.kind).rowOidAt(target))
                // The two above as words — the repository's business, not the claim's.
                + " took=" + navProbe.factsTook()
                + " active=" + sidebarPane.activeKey
        }
        function tipLine() {
            const target = navTipTimer.target()
            const tip = Tips.box
            const words = navTipTimer.head ? navProbe.headPinWords : navProbe.tipWordsAt(navTipTimer.kind, target)
            return "nav_tip section=" + (navTipTimer.head ? "head" : navTipTimer.kind)
                + " row=" + target + " name=" + navTipTimer.restedName()
                + " lit=" + navTipTimer.witnessed("lit") + " wants=" + (words !== "")
                + " tip=" + tip.visible + " open=" + navProbe.rowFactsOpen
                // Where the stand-in rested once a fold took its row: `sat=` weighs its own `y` against the holding
                // row's seat, so other arithmetic shows; `at=` / `gap=` are the raw pair. Its words at rest are
                // `nav-pin-seat`'s — this run lights it.
                + (navTipTimer.fold >= 0
                   ? " under=" + navProbe.headPinUnder
                     + " sat=" + (Math.round(navProbe.headPinY) === Math.round(navProbe.headPinSeatY()))
                     + " at=" + Math.round(navProbe.headPinY)
                     + " gap=" + Math.round(navProbe.headPinSeatY())
                   : "")
                // Whether what opened is inside what the list shows (true wherever there was room under the row).
                + (navTipTimer.opens ? " shown=" + navProbe.rowFactsShown() : "")
                // Off its rows (`AutoActDriver.tipAside`); after the fold and the facts, which read as one run each.
                + " aside=" + driver.tipAside(tip)
                + " text=" + (tip.visible ? tip.text : "")
                // After a text that may carry anything, so the judged four above stay one substring. `seat=` is never
                // judged: the reader's witness that the lines sit under the row's line (`lines=` at `line=`'s y is
                // lines drawn over the name); its `view=` / `room=` say whether the section grew or the list scrolled.
                + (navTipTimer.opens ? " seat=" + sidebarPane.rowFactsGeom() : "")
                + (navTipTimer.opens ? " says=" + navProbe.rowFactsWords() : "")
                + (navTipTimer.head ? " pin=" + navProbe.headPinLit : "")
        }
        onTriggered: {
            const op = navTipTimer
            if (op.at("start")) {
                // Re-applied every beat until the row opens: the delegate arrives on a later layout than the rows the
                // model got, and a miss reads exactly like a row that answers a rest with nothing (`NavList.clickRow`).
                if (!op.input("lit", () => {
                        navProbe.pointTipAt("worktree", 0)
                        if (!navProbe.rowFactsOpen)
                            return false
                        op.witness("lit", navProbe.rowFactsOpen)
                        navProbe.pointTipAt("worktree", -1)
                        // The filter goes in only now: it would take the control row out along with the stand-in's.
                        if (op.hide !== "")
                            navProbe.typeFilter(op.hide)
                        if (op.head)
                            sidebarPane.headPinPointed = true
                    }))
                    Awaited.all("nav_tip_control", { "open": navProbe.rowFactsOpen })
                return
            }
            // The fold, once the view has built its row: BRANCHES arrives on a read of its own, and a click on row 0
            // of a still-empty list folds nothing while answering as an already-folded row. A beat is given back.
            if (op.fold >= 0 && op.at("lit")) {
                op.input("folded", () => navProbe.clickRow("branch", op.fold))
                return
            }
            if (op.foot && op.at("lit")) {
                // BRANCHES may still be empty — the control's WORKTREES is a read of its own — and its last row would
                // be -1, which is also "the pointer is on no row". And the control row shut: WORKTREES giving its room
                // back makes BRANCHES taller, which pulls the list's end up under a scroll taken before.
                const list = navProbe.listOf("branch")
                if (!Awaited.all("nav_tip_foot", { "rows": !!list && list.count > 0, "shut": !navProbe.rowFactsOpen }))
                    return
                op.input("footed", () => {
                    navProbe.scrollBranchesToEnd()
                    op.witness("row", list.count - 1)
                })
                return
            }
            // Where the list stands, noted once the scroll has stood across a polish pass: the shut row's room and the
            // rows the scroll built are laid out only there, and an end that moves after the note reads as a give-back
            // that missed.
            if (op.foot && op.at("footed") && !op.kept("restY")) {
                const at = navProbe.listContentY("branch")
                if (Awaited.all("nav_tip_footed", { "still": op.still("nav_tip_foot_y", at) }))
                    op.witness("restY", at)
                return
            }
            // The rest, re-applied every beat until it lands — and not once the hand has moved on: re-applying would
            // put the pointer back on the row it walked off (`nav-open-then away`).
            if (!op.at("rested") && !op.at("acted")) {
                if (!op.input("rested", () => {
                        if (!op.head)
                            navProbe.pointTipAt(op.kind, op.target())
                        return op.restHolds()
                    })) {
                    Awaited.all("nav_tip_rested", op.restTerms())
                    return
                }
            }
            if (op.moves && op.at("rested")) {
                op.input("acted", () => {
                    op.witness("rested", op.nameNow())
                    const shut = navProbe.afterOpen(op.then, op.kind, op.target())
                    if (op.then === "bar")
                        op.witness("shut", shut)
                })
                return
            }
            // Nothing is read until the layout stops moving: the lines are measured on a layout and the section's
            // height follows that, so on the opening pass neither has arrived and the lines read as out of view. The
            // geometry standing across a polish pass is the list standing still (`StageTimer.still`).
            if (!Awaited.all("nav_tip_laid", {
                    "laid": !op.opens || op.still("nav_tip_geom", sidebarPane.rowFactsGeom()) }))
                return
            op.complete({
                "name": "nav_tip_landed",
                "when": () => op.landed(),
                "report": () => op.foot ? op.footLine() : op.then !== "" ? op.thenLine() : op.tipLine() })
        }
    }
    // _PGG_AUTO_ACT=nav-open-tag: the lines a TAGS row opens under itself, one per remote carrying the name.
    //
    // The readings must be in first: nothing local records a remote's `refs/tags/` (`remote::tags::list_tags`), so a
    // row rested on before `ls-remote` answered looks like nobody else's tag. Waits on the answer itself, as
    // `AutoActRefVerbs.tagMenuTimer` does. Run on `--preset tagremotes`, the one preset with three remotes
    // (`--preset tags` has origin alone, so a row opens on one line whatever it carries).
    PageStage {
        id: navOpenTagTimer
        name: "nav_open_tag"
        property string tag: ""
        /// What has to be known first: `remote` waits until some remote carries the name, `drift` until one carries it
        /// somewhere else. Empty is a name nobody out there has — an empty row looks the same before the reading, so
        /// that run waits on its own fetch being read through (`AutoActDriver.wroteAndSettled`).
        property string wants: ""
        /// Where the hand goes on to rest once the row is open: `line` = the first of the lines it opened (the first on
        /// purpose: searching for the one with a supplement would ask the answer where to find itself; lines are in
        /// name order, so which it is is the preset's — `--preset tagremotes`: `fork`); `name` = the row's own name,
        /// which keeps why the copy here wears the warning; empty = nowhere. On a place with nothing to add the run
        /// hits the ceiling. The name wears `warning` where the copy here stands on another commit than the reference
        /// remote's (read with `here_apart=true`). A run's tip stands at the head of the lines (`TipSeat.seatY`), so
        /// `:tip` and `:name` frame in the same seat — the argument and the line say which rest it was.
        property string rest: ""
        /// _PGG_AUTO_ACT=tag-line-menu: the line the menu is asked for on, -1 for a run that only opens the row.
        property int menuLine: -1
        function begin(arg, menuLine) {
            const parts = ("" + arg).split(":")
            const last = parts[parts.length - 1]
            const rest = parts.length > 1 && (last === "tip" || last === "name") ? last : ""
            navOpenTagTimer.tag = parts[0]
            navOpenTagTimer.wants = parts.length > 1 && parts[1] !== rest ? parts[1] : ""
            navOpenTagTimer.rest = rest === "tip" ? "line" : rest
            navOpenTagTimer.menuLine = menuLine
            // The row opened; then the second rest where one is asked, or the menu on a line and its card.
            navOpenTagTimer.order = ["opened"].concat(menuLine >= 0 ? ["rested", "card"]
                                                      : navOpenTagTimer.rest !== "" ? ["rested"] : [])
            // Waited on by its ask's id (`rename-tag-remote` の同じ待ち).
            driver.pressFetch()
            navOpenTagTimer.start()
        }
        /// Whether what this run waits on has arrived, asked of the lookups the lines are drawn from
        /// (`NavSectionModel`) — no count of fetches says whether the readings are in.
        function ready() {
            if (navOpenTagTimer.wants === "drift")
                return tagsModel.remoteTagDrift(navOpenTagTimer.tag, repoTab.defaultRemote) !== ""
            if (navOpenTagTimer.wants === "remote") {
                const sides = tagsModel.tagSides(navOpenTagTimer.tag)
                return sides === "remote" || sides === "both"
            }
            return driver.wroteAndSettled()
        }
        /// The row open on the readings this run waits for.
        function opened() {
            const idle = !(repoTab.busyCount !== 0)
            const read = idle && navOpenTagTimer.ready()
            const row = read ? tagsModel.rowOfName(navOpenTagTimer.tag) : -1
            return { "ready": idle, "read": read, "row": row >= 0, "open": row >= 0 && navProbe.rowFactsOpen }
        }
        /// What the completion stands on: the row open, and the menu or the second rest's tip where one is asked.
        function landed() {
            const opened = navOpenTagTimer.opened()
            return { "ready": opened.ready, "read": opened.read, "row": opened.row, "open": opened.open,
                     "menu": navOpenTagTimer.menuLine < 0 || refMenu.opened,
                     "tip": navOpenTagTimer.menuLine >= 0 || navOpenTagTimer.rest === "" || Tips.box.visible }
        }
        onTriggered: {
            const op = navOpenTagTimer
            const opened = op.opened()
            if (op.at("start")) {
                if (!Awaited.all("nav_open_tag_row", { "ready": opened.ready, "read": opened.read, "row": opened.row }))
                    return
                // Asked again every beat until the row opens (`nav-open` の同じ歩き).
                op.input("opened", () => {
                    navProbe.pointTipAt("tag", tagsModel.rowOfName(op.tag))
                    return navProbe.rowFactsOpen
                })
                return
            }
            // Nothing is read until the layout stops moving (`nav-open` の同じ待ち).
            if (!Awaited.all("nav_open_tag_ready", {
                    "ready": opened.ready,
                    "read": opened.read,
                    "row": opened.row,
                    "open": opened.open,
                    "laid": opened.open && op.still("nav_open_tag_geom", sidebarPane.rowFactsGeom()) }))
                return
            // Or the menu on one line, asked once — its answer is the card, not the line.
            if (op.menuLine >= 0) {
                if (op.at("opened")) {
                    op.input("rested", () => navProbe.factsLineMenu(op.menuLine))
                    return
                }
                if (op.at("rested")) {
                    if (!Awaited.all("nav_open_tag_menu", { "menu": refMenu.opened }))
                        return
                    op.input("card", () => {
                        refMenu.openSub(refTagCard)
                    })
                }
                op.complete({
                    "name": "nav_open_tag_menu",
                    "when": () => op.landed(),
                    "report": () => driver.tagReachWords(refTagCard) })
                return
            }
            // Then the second rest, on the first line or the name, asked again until it takes: the lines are built a
            // pass after the row that opened them.
            if (op.rest !== "" && op.at("opened")) {
                op.input("rested", () => op.rest === "name" ? navProbe.pointFactsName(true)
                                                             : navProbe.pointFactsLine(0, true))
                return
            }
            // `sides=` (what the row's cloud is drawn from) and `says=` (the carriers the lines name) are the claim as
            // a pair: a cloud over no lines and lines naming an unheard-of remote frame alike.
            op.complete({
                "name": op.rest !== "" ? "nav_open_tag_tip" : "nav_open_tag_ready",
                "when": () => op.landed(),
                "report": () => {
                    const tip = Tips.box
                    return "nav_open_tag tag=" + op.tag
                        + " row=" + tagsModel.rowOfName(op.tag)
                        + " sides=" + tagsModel.tagSides(op.tag)
                        + " open=" + navProbe.rowFactsOpen
                        + " shown=" + navProbe.rowFactsShown()
                        + " seat=" + sidebarPane.rowFactsGeom()
                        + " says=" + navProbe.rowFactsWords()
                        // The line's supplement, off the shared instance — last, after a sentence with free fields.
                        + " tip=" + tip.visible + " text=" + (tip.visible ? tip.text : "")
                } })
        }
    }
}
