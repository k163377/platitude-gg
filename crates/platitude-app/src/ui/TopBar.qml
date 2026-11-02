pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// The window's chrome: two rows of fixed height — the tab band (`toolbarHeight`) and the operation panel
// (`opsBarHeight`) — with no `splitterWidth` band between them (the panel's own hairline edges aside): that band
// parts a toolbar from the panes, and the panel has a ground of its own (`bgRaised`), so a band in that ink would
// only make the panel 4px deeper.
//
// **The band** is the window's title bar where `captionMerged` (`Main.captionMerged`). Its two ends are built alike:
// the ☰ and the window's three are `railWidth` cells the band's whole height (`WindowButton`). In order: tab strip →
// state badges (`BandStateGroup`) → window buttons, **no upright line between them** (user decision — ask before
// changing): the strip keeps its grab run behind the tabs, so a line would stand before empty band in most windows
// and part nothing. Pressable boxes stand the band's `spacing` (`spaceXs`) apart; a window button keeps its own air
// inside its cell. **An empty run that takes no press is the window's grip** — the run past the tabs
// (`TabStrip`'s `grabRun`) and the seam before the window's buttons (`seamRun`; user decision); a new gap in the
// band follows the same rule. The hit test is answered here (`winframe`), so the grip is too: left to Qt, every point
// nothing takes would answer HTCAPTION and every gap between pressable boxes would grab. So a new empty run in the
// band is a decision whether to report it, in `seamRun`'s shape. **The right end is flush with the window's edge**
// (user decision): the ✕ reaches it as the ☰ does on the left — a frameless window's client area is the drawn window,
// so a margin would be a gap right of the ✕. Only the panel's find stands in (`findRightMargin`).
//
// **The panel** says where the window stands and opens the roads from there: the names on the left, the actions on
// the right. The band names the repository only, so the worktree and the branch are this row's. Each name is a door
// (`OpsPicker`): a press drops its card right under the name — not at the pointer, so one name always opens in one
// place — and while the card stands the name stays lit and its chevron points down (user decision): the hand moves
// into the card, and a name letting go of its light leaves the card hanging from nothing. No upright line between the
// names or before the actions: each name is headed by its chevron. The cards are filed and worded as the left menu's
// sections, and their rows go down the left menu's roads (`switchToRef`, `openRepositoryPathRequested`), so the two
// never answer differently; they list everything whatever the left menu folds or filters to — a card is a road, not
// the list being read (`nav::card`). Each card is as wide as its rows (user decision; `widthFloor`).
//
// Narrowing, the panel gives way in one order: the find's word → the three actions' words → the four buttons' width
// → the worktree's name → the repository's → the upstream → the branch's (`findFolded` / `actionsFolded` /
// `opsRepoCut` / `OpsPicker.given`).
Rectangle {
    id: topBar

    required property var tabsModel
    // The RepoPage of the active tab (null while no tab is open).
    property var curPage: null
    /// Whether this band is the window's title bar. When it is, the band carries the window's own buttons and tells the
    /// platform where its empty runs sit, so the gestures on those are the platform's own
    /// (`AppBackend.setCaptionStrips`).
    property bool captionMerged: false
    /// Which shape the middle button is in.
    property bool windowMaximized: false

    /// Automation: what the band came out to — a layout change can lose the window's buttons or the grab run, or push
    /// either off the end, and a screenshot where the platform draws no buttons shows none of it
    /// (`WindowAutoActDriver`).
    readonly property real bandGrabRun: tabStrip.grabRun
    /// …and the band's other run: the seam before the window's buttons is empty band in a picture whether or not the
    /// hit test was told about it.
    readonly property real bandSeamRun: seamRun.width
    readonly property real bandButtonsX: minimizeButton.x
    readonly property real bandRightMargin: bandRow.anchors.rightMargin
    readonly property real bandTabsWidth: tabStrip.tabsWidth
    readonly property int bandTabCount: tabStrip.tabCount
    readonly property real bandTabRun: tabStrip.runAvail
    readonly property real bandTabContent: tabStrip.contentWidth
    readonly property bool bandTabScrolls: tabStrip.contentWidth > tabStrip.tabsWidth
    /// …and the stand-in for the tab in front, which a picture of a scrolled strip cannot be read for
    /// (`TabStrip.tabPinShown`).
    readonly property bool tabPinShown: tabStrip.tabPinShown
    readonly property bool tabPinRidesLeft: tabStrip.tabPinRidesLeft
    readonly property bool tabPinNameKept: tabStrip.tabPinNameKept
    readonly property bool frontTabWhole: tabStrip.frontTabWhole
    readonly property bool tabRunTravelling: tabStrip.runTravelling
    /// Automation: the state group's readings, handed on (`BandStateGroup`).
    readonly property bool opBadgeShown: stateGroup.opBadgeShown
    readonly property bool conflictBadgeShown: stateGroup.conflictBadgeShown
    readonly property bool identityBadgeShown: stateGroup.identityBadgeShown
    readonly property bool oldGitBadgeShown: stateGroup.oldGitBadgeShown
    readonly property bool staleBadgeShown: stateGroup.staleBadgeShown
    readonly property bool lfsBadgeShown: stateGroup.lfsBadgeShown
    readonly property real stateBadgeMinW: stateGroup.stateBadgeMinW

    /// Automation: which of the group's three shapes is on screen, what the badges were narrowed to, and what the card
    /// came back with (`PGG_AUTO_ACT=badges` / `badges-hover`).
    readonly property bool stateWordsShown: stateGroup.stateWordsShown
    readonly property bool stateMarkShown: stateGroup.stateMarkShown
    readonly property color stateMarkColor: stateGroup.stateMarkColor
    readonly property int stateCapW: stateGroup.stateCapW
    readonly property int stateGroupW: stateGroup.stateGroupW
    /// …and whether a folded group is as wide as its mark and no wider (`BandStateGroup.markFitted`).
    readonly property bool stateMarkFitted: stateGroup.markFitted
    /// …and whether its frame is a badge's depth (`BandStateGroup.markDeep`).
    readonly property bool stateMarkDeep: stateGroup.markDeep
    /// …and whether the band laid out with the words (`bandAsked`) hands the strip and the group together what the
    /// real one does — the two rows share every other cell, so a cell the shadow does not mirror is the difference.
    readonly property bool bandShadowAgrees:
        Math.abs(stripAsked.width + (groupAsked.visible ? groupAsked.width : 0)
                 - tabStrip.width - (stateGroup.visible ? stateGroup.width : 0)) < 1
    readonly property bool stateCardOpen: stateGroup.stateCardOpen
    /// Whether the row has ever placed the group — one of the three things the stand-in pointer needs before a card
    /// comes up (`BandStateGroup.asking`), and the only one a verb cannot otherwise read: a run that saw no
    /// card is a different fault on either side of it.
    readonly property bool statePlaced: stateGroup.placed
    readonly property string stateCardRows: stateGroup.stateCardRows
    readonly property string stateCardSize: stateGroup.stateCardSize
    readonly property bool stateCardLaidOut: stateGroup.stateCardLaidOut
    /// Whether the window is standing on its floor. Handed in: the floor is the larger of this band's and the page's,
    /// and only the body that seats both has it (`WindowBody`).
    property bool windowAtFloor: false
    /// Stands in for the pointer where headless cannot put one (`badges-hover` / `identity-tip`). The real hover writes
    /// this same property — hover cannot be injected, so unless the card answers this one question the headless run
    /// proves nothing about it.
    property bool statePointedAt: false
    /// Whether the fetch button can be pressed — the edge `fetch-tip` waits on before it reads.
    readonly property bool fetchLive: fetchButton.enabled

    /// How far the state card's sentences stand off their badges' words (`BandStateCard.wordsOffLevel`). A call: it
    /// maps laid-out items, which no binding follows.
    function stateCardWordsOff() { return stateGroup.stateCardWordsOff() }
    /// A press on either name. **Each is a toggle**, like the ☰ (`AppMenuButton`): `open()` on a card already up does
    /// nothing, which reads as a name that cannot be pressed twice. Each card is assembled as it opens
    /// (`NavSectionModel.worktreeCard` / `OpsBranchMenu.offerFrom`); each says whether it opened a card.
    function pressRepoName() {
        if (standMenu.opened) {
            standMenu.close()
            return false
        }
        worktreeSub.rows = topBar.curPage === null ? [] : topBar.curPage.pageWorktrees.worktreeCard()
        return standMenu.offerHere()
    }
    function pressBranchName() {
        if (branchMenu.opened) {
            branchMenu.close()
            return false
        }
        return branchMenu.offerFrom()
    }
    /// Automation: the panel's two doors, opened at the press's own handler (a press cannot be injected — verify-ui
    /// verbs.md「press は注入できない」). Each says whether the card came up: a picture cannot tell no card from one
    /// opened off screen.
    function openStandMenu() { return topBar.pressRepoName() }
    function openStandRepos() { return standMenu.openSub(repoSub) }
    function openStandWorktrees() { return standMenu.openSub(worktreeSub) }
    function openBranchMenu() { return topBar.pressBranchName() }
    /// …and a folder of the branch card, opened the way resting on its row opens it (`OpsBranchMenu.openFolder`).
    function openBranchFolder(path) { return branchMenu.openFolder(path) }
    /// …and what each of them came out holding. A card is assembled from a listing that may not have landed yet, so
    /// a run that photographed an empty one has to be able to say so (`AppMenu.offeredRows`).
    readonly property bool standMenuOpen: standMenu.opened
    readonly property bool branchMenuOpen: branchMenu.opened
    /// **Which card is standing** — the one thing neither the counts nor the picture answers: a tier asked for and
    /// not opened leaves the card above it on screen, and a run that judged only that something was open would go
    /// green on a card it was never about.
    readonly property string standDoor:
        branchMenu.folderStanding !== "" ? "folder"
        : branchMenu.opened ? "branch"
        : repoSub.opened ? "repos"
        : worktreeSub.opened ? "worktrees"
        : standMenu.opened ? "stand" : "none"
    readonly property int standRepoRows: repoSub.offeredRows
    readonly property int standWorktreeRows: worktreeSub.offeredRows
    readonly property int branchMenuRows: branchMenu.offeredRows
    /// Which folder of the branch card is standing, by its path (`OpsBranchMenu.folderStanding`).
    readonly property string branchFolderOpen: branchMenu.folderStanding
    /// Automation: what the two names are washed in and which way their chevrons point, read on **the output side**
    /// (the wash's colour, the mark's turn): a lit name and a name under a pointer are the same picture
    /// (`HoverToolButton.washColor`).
    readonly property bool repoNameLit: Qt.colorEqual(repoPick.washColor, Theme.bgHover)
    readonly property bool branchNameLit: Qt.colorEqual(branchPick.washColor, Theme.bgHover)
    readonly property bool repoNameTurned: repoPick.foldTurn === 90
    /// …and where the branch's counts came out against the two lines (`OpsPicker.trackPlace`).
    readonly property string branchTrackPlace: branchPick.trackPlace
    readonly property bool branchNameTurned: branchPick.foldTurn === 90
    /// Automation: a row of each card pressed through the row's own `triggered` — what a click emits, so everything
    /// after it is the wiring a hand goes through (`AppMenuButton.clickCloneRow`). Each answers whether it found an
    /// offered row: the listing may not have landed, and a press at nothing is not one to wait on.
    function pickBranchRow(name) {
        // Through the card's own lookup: a folder's rows are made as its card opens (`OpsBranchMenu.rowFor`).
        const row = branchMenu.rowFor(name)
        if (row === null || !row.offered)
            return false
        row.triggered()
        return true
    }
    function pickWorktreeRow(leaf) {
        return topBar.pickIn(worktreeSub, row => GitFacts.pathLeaf(row.full) === leaf)
    }
    function pickRepoRow(index) {
        return topBar.pickIn(repoSub, row => row.index === index)
    }
    /// The row `matches` names, looked for down every card this one folds.
    function pickIn(menu, matches) {
        for (let i = 0; i < menu.count; i++) {
            const row = menu.itemAt(i)
            // A blocked row takes a hand's press and drops it (`AppMenuItem.blocked`), so it is no row to press.
            if (!row || !row.offered || row.blocked === true)
                continue
            if (row.subMenu) {
                if (topBar.pickIn(row.subMenu, matches))
                    return true
            } else if (matches(row)) {
                row.triggered()
                return true
            }
        }
        return false
    }

    signal openRepositoryRequested()
    signal cloneRepositoryRequested()
    signal identityEditRequested()
    signal settingsRequested()
    signal maximizeToggleRequested()
    signal minimizeRequested()
    signal closeRequested()
    /// The grab-run moved or changed size in this band's own layout. `Main` folds in the shifts this band cannot see
    /// from here (the maximised inset, the window resizing) and reports the strip on.
    signal captionStripMoved()
    /// The two runs themselves, for `Main` to measure in scene coordinates.
    readonly property Item grabRunItem: tabStrip.grabRunItem
    readonly property Item seamRunItem: seamRun
    /// Whether the ☰'s card is standing (`TabStrip`), for automation (`PGG_AUTO_ACT=app-menu`).
    readonly property bool appMenuOpen: tabStrip.appMenuOpen

    /// The narrowest this chrome can be laid out at — one of the two numbers the window's floor is the larger of
    /// (`Main.floorWidth`). **The larger of its two rows**: the band's read off its layout minimum, the panel's off its
    /// parts at their floors, so a control added to either is counted without anybody remembering to. The band's
    /// counts the state group folded to its `…` (its `Layout.minimumWidth`), so any number of states raises it by one
    /// mark; the tab strip at two tabs (`TabStrip.tabStripFloorW`). Never what the cells ask for: a floor at the
    /// asked widths would keep the window from narrowing.
    ///
    /// `bandRow.Layout.minimumWidth` is a sum the layout writes in its own pass: in the beat a child's `foldedWidth`
    /// moves it still holds the pass before's (a `RowLayout` has no `forceLayout()`). A run that reports this floor
    /// reads it a stage after its wait (`WindowFrameActs`' `window-floor`) — read in the same beat it comes out one
    /// mark short, only on a machine whose status is slow, and never in a picture.
    readonly property real floorWidth:
        Math.max(bandRow.Layout.minimumWidth + bandRow.anchors.rightMargin,
                 topBar.picksFloor + Theme.spaceLg + topBar.actionsFoldedWidth
                 + topBar.opsRightAir)
    /// What the three actions and the find come to once every one is a mark in an end cell — the panel's half of the
    /// floor above. Folded, never with their words: whether the words stay is the panel's width's to decide, so a floor
    /// priced with them would be a loop.
    readonly property real actionsFoldedWidth:
        topBar.actionCells * Theme.railWidth
        + (actionSeat.children.length - 1) * actionSeat.spacing

    /// Keeps the two actions' waiting visual up past the operation that raised it, for a picture of the wait: a fast
    /// remote is done before one can be grabbed. False unless written from outside (デザイン規約 §進行中・回数押しの定数).
    property bool holdPushBusy: false
    property bool holdFetchBusy: false
    readonly property string pushMode: pushButton.mode
    /// Automation: what the Stash button read the worktree as, and the edge `stash-state` waits on — every
    /// refusal photographs as the same dim button.
    readonly property string stashMode: stashButton.mode
    readonly property bool stashLive: stashButton.enabled
    /// Whether either action has a string to open under a pointer. **The string itself** — a disabled control takes
    /// hover and asks for the window's tip like any other (rules-refs の `hoverEnabled` の行), so what tells the two
    /// sides apart is whether there is anything to open (`fetch-tip`).
    readonly property bool stashTipShown: stashButton.tip !== ""
    readonly property bool fetchTipShown: fetchButton.tip !== ""
    /// Automation: the pointer's stand-in on the fetch button, and the two things the button answers it with — the tip
    /// standing (the words on screen, `tipDelayMs` waited out) and the word come back to full (`fetch-hover`).
    property alias fetchPointedAt: fetchButton.pointedAt
    readonly property bool fetchTipStanding: fetchButton.tipShown
    readonly property bool fetchWordFull: Qt.colorEqual(fetchButton.fg, Theme.textPrimary)
    /// The same stand-in on the push button, and its tip standing (`push-hover`).
    property alias pushPointedAt: pushButton.pointedAt
    readonly property bool pushTipStanding: pushButton.tipShown
    /// How the fetch button stands: how many fetches have failed, and whether its frame has turned. **The turn, not
    /// the frame** — every button here wears a frame at rest; the third failure changes its colour, read off the
    /// colour the button hands it (`BandFetchButton.frameColor`).
    readonly property int fetchFails: fetchButton.fails
    readonly property bool fetchFrameTurned: Qt.colorEqual(fetchButton.frameColor, Theme.warning)
    /// Automation: the Stash button, pressed at the button so the real wiring answers (`PGG_AUTO_ACT=stash`).
    /// **Answers whether the press went in**: it is refused while the tab is busy (the fetch on opening is one), so the
    /// caller keeps offering it.
    function stashNow() {
        if (!stashButton.enabled)
            return false
        stashButton.clicked()
        return true
    }
    /// Automation: the Fetch button, pressed at the button (`PGG_AUTO_ACT=fetch-resume`): the stopped button is the one
    /// place that asks for the timer back, so calling the slot behind it would pass a build where the press no longer
    /// reaches it. Answers whether the press went in.
    function fetchNow() {
        if (!fetchButton.enabled)
            return false
        fetchButton.clicked()
        return true
    }
    /// Automation: the panel's Find mark, pressed at the button (`PGG_AUTO_ACT=band-find`): the mark and the key meet
    /// inside the page, so calling that would pass a build where the mark reaches nothing. Answers whether the press
    /// went in — it is down while a plan holds the graph's seat, loading included (`RepoPage.planShown`).
    function findNow() {
        if (!findButton.enabled)
            return false
        findButton.clicked()
        return true
    }
    /// Automation: the push held to its end (`PGG_AUTO_ACT=force-push-hold`). Answers whether the press went in — none
    /// does while git has the tab (`ActionButton.completeHold`).
    function completePushHold() {
        return pushButton.completeHold()
    }
    /// Automation: the push button's gesture is still under way (`ActionButton.gesturing`) — the automated press is run
    /// out from rest, not over one.
    readonly property bool pushHolding: pushButton.gesturing
    /// A press landed somewhere in the window (`Main`, off `FocusRelease.pressedAnywhere`), for the strip
    /// (`TabStrip.pressLanded`).
    function pressLanded() { tabStrip.pressLanded() }

    /// Automation: the strip's own hooks, handed on — `Main` and `WindowAutoActDriver` hold the band (`TabStrip`).
    function clickAppMenu() { tabStrip.clickAppMenu() }
    function clickCloneRow() { tabStrip.clickCloneRow() }
    function middleClickTab(index) { return tabStrip.middleClickTab(index) }
    function dragTabTo(from, to) { return tabStrip.dragTabTo(from, to) }
    function holdTabAt(index) { return tabStrip.holdTabAt(index) }
    function frontAskAccount() { return tabStrip.frontAskAccount() }
    function heldTabShift() { return tabStrip.heldTabShift() }
    function carryTabPastEnd(index) { return tabStrip.carryTabPastEnd(index) }
    function heldTabIndex() { return tabStrip.heldIndex() }
    function dropCarriedTab() { tabStrip.dropTab() }
    function runAtEnd() { return tabStrip.runAtEnd() }
    function runOffset() { return tabStrip.runOffset() }
    function sendTabRunAway() { return tabStrip.sendRunAway() }
    function pressTabPin() { return tabStrip.pressTabPin() }
    /// The strip's own list, passed on for the harness's probe (`TabStrip.tabsView`).
    readonly property alias tabsView: tabStrip.tabsView

    /// The word the panel's buttons are all measured for: one box for the set keeps any of them from shifting the
    /// others, and which wording is widest depends on the installed fonts. Fetch and push change their words on their
    /// own (`fetch` ⇄ `Resume`, `push` ⇄ `push -f`), so apart, one's change would push the other sideways; `stash`
    /// never changes, yet shares the box too — out of it, `fetch` and `stash`, the same five monospace cells, would
    /// be targets of two sizes. Measured on the longest wording of every state of the three (`BandWidest`); how a
    /// shorter word sits in it is `ActionButton.headAir`'s. Settled once: as a binding, reading four
    /// text metrics to feed the buttons' widths is a loop to the engine.
    property string widestAction: ""
    property bool widestActionCode: false

    // ---- how the three give way as the panel runs short ----------------
    /// The narrowest cell that still holds a word, for the **set**: the widest of the three wordings said now, whole —
    /// the moment one of them no longer fits, all three give their words up at once (`ActionButton.folded`;
    /// `ActionButton.foldWidth` — a longer wording said now, `push -f` / `Resume`, folds the set sooner).
    readonly property real actionFold: Math.max(fetchButton.foldWidth,
                                                pushButton.foldWidth,
                                                stashButton.foldWidth)
    /// The cells that share one width: the three and the find, **folded or not** — the four stay equal at every step,
    /// and giving the words up changes what is in a cell, not its width at once. Headless: `band-actions*`' `even=` /
    /// `find=`.
    readonly property int actionCells: actionSeat.children.length + 1
    /// What the panel holds around the four cells before any width goes to a wording: the names **at their whole
    /// width** (the buttons' words go before a letter of a name does — the header's order), the step between
    /// the two halves, the air at the right end (`opsRightAir`), and the seat's own spacing.
    readonly property real actionsBox:
        topBar.picksWant + Theme.spaceLg + topBar.opsRightAir
        + (actionSeat.children.length - 1) * actionSeat.spacing
    /// The widest a button may be drawn at the panel's width now; a cell short of its wording gives the word up
    /// (`ActionButton.folded`). **The panel's width alone** — never the band's leftover or the window's floor: the
    /// leftover answers how many tabs there are and how long their names, and how far the buttons can shrink is
    /// another question.
    readonly property real actionCap:
        Math.max(Theme.railWidth,
                 Math.min(fetchButton.naturalWidth,
                          (topBar.width - topBar.actionsBox) / topBar.actionCells))
    /// The two widths the shapes change at, for a run to put the window on: the narrowest panel that still says every
    /// wording whole, and the widest that has given all three up. Read off the cap's own arithmetic, so a driver never
    /// writes a second copy of it (`WindowBandActs`). **The panel's widths** — the window is wider by its edge
    /// (`Main.edgeInset`).
    readonly property int actionsWholeAt:
        Math.ceil(topBar.actionsBox + topBar.actionCells * fetchButton.naturalWidth)
    readonly property int actionsFoldAt:
        Math.ceil(topBar.actionsBox + topBar.actionCells * topBar.actionFold) - 1
    /// Where the words go. Said once for the set: three cells the row rounded differently must not come out in two
    /// different shapes.
    readonly property bool actionsFolded: topBar.actionCap < topBar.actionFold

    /// Automation: the two ends of the cap's travel and what the panel made of it at this width (`band-actions`) — a
    /// photograph says which shape landed, not which arithmetic put it there.
    readonly property int actionNaturalW: Math.round(fetchButton.naturalWidth)
    readonly property int actionFoldW: Math.round(topBar.actionFold)
    readonly property int actionCapW: Math.round(topBar.actionCap)
    /// The widest wording actually **on** the panel now: the box holds every state's wording, and where the words go
    /// is set by the one being said (`band-actions`).
    readonly property int actionWantW: Math.round(Math.max(fetchButton.wordWant,
                                                           pushButton.wordWant,
                                                           stashButton.wordWant))
    /// …and what one button came out as. Read off push — the one that says the longest wording and wears the frame,
    /// the `!` and the hold when its branch has diverged (`BandPushButton`).
    readonly property int actionCellW: Math.round(pushButton.width)
    readonly property int actionCellH: Math.round(pushButton.height)
    readonly property int actionWordW: Math.round(pushButton.wordRoom)
    readonly property int actionInkW: Math.round(pushButton.wordInk)
    /// Whether either of the two that can wear a `!` is wearing one. Asked of the pair: `band-actions-alert` stages the
    /// refused push, and the run that stages one without a network is the fetch that cannot reach its remote
    /// (`band-actions-stopped`).
    readonly property bool actionAlertShown: fetchButton.alert || pushButton.alert

    Component.onCompleted: {
        let best = widest.fetchCodeWidest
        for (const m of [widest.fetchWordWidest, widest.pushCodeWidest, widest.stashCodeWidest])
            if (m.implicitWidth > best.implicitWidth)
                best = m
        topBar.widestAction = best.text
        topBar.widestActionCode = best.code
    }
    // The four candidate wordings, drawn invisibly the way a button would draw them (`BandWidest`).
    BandWidest {
        id: widest
    }

    /// Automation: what the strip made of the run it was handed, and the two ends it was settled between
    /// (`PGG_AUTO_ACT=tab-widths` / `badges`).
    readonly property real tabTitleCap: tabStrip.tabTitleCap
    readonly property int tabTitleMinW: tabStrip.tabTitleMinW
    readonly property real tabTitleEaseW: tabStrip.tabTitleEaseW
    readonly property int tabTitleMaxW: tabStrip.tabTitleMaxW
    /// The room every tab is keeping for its mark, which the strip gives up first (`TabMetrics.markRoomFull` /
    /// `markRoomMin`) — a picture of a strip that has given it all up reads the same as one that has not had to.
    readonly property real tabMarkRoom: tabStrip.tabMarkRoom
    /// And which of the two states that leaves the strip in — the reading a run is judged on, since the count that
    /// reaches either is the platform's answer and not the run's (`TabStrip.tabNamesCut`).
    readonly property bool tabNamesCut: tabStrip.tabNamesCut
    readonly property bool tabMarksFolded: tabStrip.tabMarksFolded

    /// What the panel says this window is standing in: **the tab in front's own names** (`TabsModel.currentRepoName`),
    /// which the tab has the moment the window moves — a name taken from the page waits on git's reading, and until
    /// the listing lands it reads the repository's own worktree as another one and the tree mark stands. The folder
    /// name of the repository's own worktree — not the band's title, which adds the parent folder where two open
    /// repositories share a name: the panel names one, with no namesake beside it. Headless: `ops-repo-pick`'s
    /// `named=`.
    readonly property string repoName: topBar.tabsModel.currentRepoName
    /// The folder of the worktree this tab is standing in (規約 §別の worktree を読む). **Empty in the repository's
    /// own worktree**, as the tab above is (規約 §タブの所作): the left panel names that worktree by its branch, which
    /// is already the next name in this row.
    readonly property string worktreeName: topBar.tabsModel.currentWorktreeName
    /// Whether HEAD is on no branch at all — a state rather than a name, and coloured as one.
    readonly property bool detachedHead: topBar.curPage !== null && topBar.curPage.pageWorktree.detached
    /// The branch HEAD is on, or the marker for a HEAD that is on no branch at all.
    readonly property string branchName:
        topBar.curPage === null || !topBar.curPage.pageWorktree.headKnown ? ""
        : topBar.curPage.pageWorktree.detached ? qsTr("detached")
        : topBar.curPage.pageWorktree.branch
    /// What this branch is measured against — **the whole ref as the remote branches are listed** (`origin/main`, as
    /// the status tick names it: `platitude_core::remote::PushMarks::upstream_name`), not the remote alone: a branch
    /// can follow a remote one under another name, so `origin` would leave the reader guessing which. Empty with no
    /// upstream.
    readonly property string branchUpstream:
        topBar.curPage === null ? "" : topBar.curPage.pageWorktree.upstream
    /// Whether that upstream has gone from the remote — git's own `[gone]`, read where the left panel reads it
    /// (`NavSectionModel.headUpstreamGone`). The name stays; the colour says the far side is gone.
    readonly property bool upstreamGone:
        topBar.curPage !== null && topBar.curPage.pageBranches.headUpstreamGone !== ""
    /// Automation: the three names the panel came out with, and whether any of them had to be cut. A photograph holds
    /// what is drawn, not which of the three gave way.
    readonly property string opsNames: topBar.repoName + "/" + topBar.worktreeName + "/" + topBar.branchName
    readonly property bool opsNameCut: repoPick.nameCut || branchPick.nameCut
    /// How deep the four frames at the panel's right end are, their words given up or not: the branch's two lines
    /// whatever it writes (`OpsPicker.pairHeight`), and never under what a button stacks (`ActionButton.stackDepth`,
    /// word line + mark seat — that floor is a user decision). Not the height of what the branch writes now: with no
    /// upstream, or while loading, that is one line, and a stacked button's word stands over its frame and its mark
    /// under it. Not `pairHeight` alone either — a face with short lines (Hiragino Sans, 30px for the pair against
    /// 40px of content) sets the two lines shallower than the word over the mark (Windows 44, Ubuntu 43 stay above
    /// the floor). Headless: `boxed=` (`actionsBoxed`) on `ops-panel` (`--preset worktrees`, a branch with no
    /// upstream) and `band-actions*` (the three in the `fonts` tier — the mac shots every push runs).
    readonly property real actionDepth: Math.max(branchPick.pairHeight, fetchButton.stackDepth)
    /// Automation: whether every button at the panel's right end is as deep as what it draws (`actionDepth` says
    /// why) — a short one reads in a picture as a style of its own.
    readonly property bool actionsBoxed: [fetchButton, pushButton, stashButton, findButton].every(
        button => !button.visible || button.height >= button.implicitHeight - 0.5)
    /// …and whether every frame there is that deep, folded or not: a folded frame one line deep sits in a picture
    /// like any other box, and only its neighbours' say it shrank.
    readonly property bool actionsDeep: [fetchButton, pushButton, stashButton, findButton].every(
        button => !button.visible || Math.abs(button.height - 2 * button.frameInset - topBar.actionDepth) < 0.5)
    /// …and whether the four are one width, the find's word given up or not (`actionCells`).
    readonly property bool actionsEven: [pushButton, stashButton, findButton].every(
        button => Math.abs(button.width - fetchButton.width) < 0.5)
    /// …and whether the line between the three and the find stands clear of both frames and ends where the frame on
    /// its other side does — a pixel off reads in a picture as that frame's own edge or a line that missed. Held
    /// against the stash's frame because the line is measured off the find's.
    readonly property bool findRuled:
        findRule.visible && findRule.x > actionSeat.x + actionSeat.width
        && findRule.x + findRule.width < findButton.x
        && Math.abs(findRule.y - (actionSeat.y + stashButton.y + stashButton.frameInset)) < 0.5
        && Math.abs(findRule.height - (stashButton.height - 2 * stashButton.frameInset)) < 0.5

    /// What the names ask for when nothing has given way — what the actions are measured against (`actionsBox`), and
    /// the width the row is held to when there is room. **Read off the whole figure**, not what is drawn now: what the
    /// row can afford is decided from this, so a sum that already held the answer would close the ring.
    readonly property real picksWant:
        repoPick.wholeWidth + branchPick.wholeWidth + opsRow.spacing
    /// **The order the names give way in** (the header's order): the repository's picker, then the branch's
    /// (`OpsPicker.given`) — the tab above says the repository again, and every write in this row acts on the branch.
    readonly property real opsRoom: opsRow.width
    /// How much more the names want than the row has — cut from each name in turn towards its floor, never by
    /// dropping one: a run dropped takes what it was saying with it.
    readonly property real opsOver: Math.max(0, topBar.picksWant - topBar.opsRoom)
    /// The narrowest the names are ever laid out at — every word at its own floor.
    readonly property real picksFloor:
        repoPick.foldWidth + branchPick.foldWidth + opsRow.spacing
    /// …taken from the two in the order above. Each gives what it can and passes the rest on.
    readonly property real opsRepoCut: Math.min(topBar.opsOver, repoPick.slack)
    readonly property real opsBranchCut: Math.min(topBar.opsOver - topBar.opsRepoCut, branchPick.slack)
    /// The air at the panel's right end: the find's step in from the window edge, and a step before the find, since the
    /// three are a set and would otherwise read as four.
    readonly property real opsRightAir: topBar.findRightMargin + Theme.spaceMd
    /// The find's step in from the window's edge. Its own, not the band's: the band runs the ✕ up to the edge, and a
    /// frame there would stand against the window's drawn edge.
    readonly property real findRightMargin: topBar.captionMerged ? Theme.spaceXs : Theme.spaceMd
    /// …and with the find in it, at the width the four share.
    readonly property real opsRightMargin: topBar.actionCap + topBar.opsRightAir
    /// The panel gives up its words in the header's order — **the buttons' before any name's, because a button
    /// that has given its word up is still read by its mark**, and a name has nothing to fall back to. The find's goes
    /// first, the moment the four are short of their whole width; its cell keeps the width the three have.
    readonly property bool findFolded: topBar.actionCap < fetchButton.naturalWidth

    implicitHeight: Theme.toolbarHeight + Theme.opsBarHeight
    color: Theme.bgElevated

    // ---- the tab band ---------------------------------------------------
    // Anchored rather than laid out in a column: the two rows of this chrome are of fixed, different heights, and the
    // run the platform is told about is measured off this row (`seamRun`).
    RowLayout {
        id: bandRow
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: Theme.toolbarHeight
        // Flush where the band is the title bar, as the ☰ is on the left: the frameless window's client area is what
        // is drawn, so the ✕'s wash runs into the window's corner (user decision, header).
        anchors.rightMargin: topBar.captionMerged ? 0 : Theme.spaceMd
        spacing: Theme.spaceXs
        TabStrip {
            id: tabStrip
            tabsModel: topBar.tabsModel
            // A tab that is not in front paints no ground of its own, so this is what its name is read against and
            // fades into under its mark.
            bandColor: topBar.color
            captionMerged: topBar.captionMerged
            curPage: topBar.curPage
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumWidth: tabStrip.tabStripFloorW
            // **The strip and the state group give way at once.** Asks for its names whole, as the group asks for its
            // words: one asking in its narrowed shape would leave the other to give everything. Short of both, the row
            // takes the shortfall from the two at once, each by its room above its floor (the one with more room gives
            // a little more, even in proportion) — Qt reads no stretch below what the items ask for, so no order
            // between them can be set here (user decision — ask before changing). The group's fold is its own
            // (`BandStateGroup.folded`); room past both is the strip's, since the group asks for its ceiling.
            onOpenRepositoryRequested: topBar.openRepositoryRequested()
            onCloneRepositoryRequested: topBar.cloneRepositoryRequested()
            onSettingsRequested: topBar.settingsRequested()
            // The Exit row is the ✕ by another name: one signal out of the band, so the window has one close gate.
            onExitRequested: topBar.closeRequested()
            onCaptionStripMoved: topBar.captionStripMoved()
        }

        /// The hand over the band — a passive handler on the row, which takes no part's hover. The state group's marks
        /// can come under a still one, and its card waits for the hand to move (`BandStateGroup.movedUnderHand`).
        HandWatch {
            id: bandHand
        }

        BandStateGroup {
            id: stateGroup
            curPage: topBar.curPage
            windowAtFloor: topBar.windowAtFloor
            pointedAt: topBar.statePointedAt
            hand: bandHand
            // **Narrowed and folded against the band as it would be with its words** (`bandAsked` says why).
            room: groupAsked.width
            tabContentWidth: tabStrip.contentWidth
            tabRunAvail: Math.max(0, tabStrip.runAvail + stripAsked.width - tabStrip.width)
            tabCount: tabStrip.tabCount
            // Measured off the fetch button: a target's width is the padding a Fusion `ToolButton` keeps around its
            // content, which the theme has no number for. Its `padding`, not the two sides — those carry the shared
            // box's slack (`ActionButton.slack`) and would move this group whenever fetch changed its wording.
            controlPadding: fetchButton.padding
            cellFolded: topBar.actionsFolded
            // Both run the band's depth from its top, so the strip's y is the group's.
            lineBaseline: tabStrip.titleBaseline
            Layout.fillWidth: true
            // The band's depth for the target and the card's seat under it; the boxes keep their own
            // (`BandStateGroup.foldedDepth`).
            Layout.fillHeight: true
            // **Folded, the mark's cell and not a pixel more**: folded with the words' width kept, the band before the
            // mark stands empty and the grab run behind the tabs looks swollen.
            Layout.maximumWidth: stateGroup.folded ? stateGroup.foldedWidth : stateGroup.naturalWidth
            Layout.minimumWidth: stateGroup.foldedWidth
            onIdentityEditRequested: topBar.identityEditRequested()
        }
        // The window's own three, drawn here (`WindowButton` says why).
        WindowButton {
            id: minimizeButton
            visible: topBar.captionMerged
            kind: "window-minimize"
            Accessible.name: qsTr("Minimize")
            onTriggered: topBar.minimizeRequested()
        }
        WindowButton {
            id: maximizeButton
            visible: topBar.captionMerged
            kind: topBar.windowMaximized ? "window-restore" : "window-maximize"
            Accessible.name: topBar.windowMaximized ? qsTr("Restore") : qsTr("Maximize")
            onTriggered: topBar.maximizeToggleRequested()
        }
        WindowButton {
            id: closeButton
            visible: topBar.captionMerged
            kind: "close"
            danger: true
            Accessible.name: qsTr("Close")
            onTriggered: topBar.closeRequested()
        }
    }

    // **The band as it would be laid out with the state group asking for its words** — the row above's constraints on
    // each cell, except that the group here never gives its words up. The group is narrowed and folded against this
    // (`stateGroup.room` / `tabRunAvail`): the row above lays a folded group out at its mark alone, so a group folded
    // for want of room would never find the room to unfold into there. Unfolded, the two rows agree cell for cell.
    // Plain items, measured and nothing else.
    //
    // **A `Layout.*` added to a cell of the row above is added here too** — left out, the fold's edge moves. Each
    // stand-in mirrors what its cell asks for, its `implicitWidth`: an item that holds a plain `width` comes out 0 wide
    // here and the group overflows its cell, so the band's cells name their size by `implicitWidth` (`WindowButton`).
    // Headless: `badges`' `shadow=` (`bandShadowAgrees`), and `badges floor:16` / `old-git-fold`'s `fitted=`
    // (`BandStateGroup.markFitted`); the wiring is `tst_bandgroup.qml`'s case that parts `room` from the width.
    RowLayout {
        id: bandAsked
        anchors.left: bandRow.left
        anchors.right: bandRow.right
        anchors.top: bandRow.top
        height: bandRow.height
        spacing: bandRow.spacing
        Item {
            id: stripAsked
            implicitWidth: tabStrip.implicitWidth
            Layout.fillWidth: true
            Layout.minimumWidth: tabStrip.tabStripFloorW
        }
        Item {
            id: groupAsked
            visible: stateGroup.visible
            implicitWidth: stateGroup.naturalWidth
            Layout.fillWidth: true
            Layout.maximumWidth: stateGroup.naturalWidth
            Layout.minimumWidth: stateGroup.foldedWidth
        }
        Item {
            visible: minimizeButton.visible
            implicitWidth: minimizeButton.implicitWidth
        }
        Item {
            visible: maximizeButton.visible
            implicitWidth: maximizeButton.implicitWidth
        }
        Item {
            visible: closeButton.visible
            implicitWidth: closeButton.implicitWidth
        }
    }

    // The band's own step before the window's buttons, with nothing drawn in it — the band draws no upright line
    // (header): this run is what parts them, the grab run's second (user decision). The hit test answers HTCAPTION for
    // it as for the run past the last tab (`TabStrip.grabArea`), so a press here is the platform's own gesture — an
    // empty run that takes no press is the window's grip. Nothing is drawn in it, so only numbers show it: `band`'s
    // `seamRun=`. On a real Windows window: a double-click on the seam maximises, a drag moves the window.
    //
    // Outside the row: a child of a `RowLayout` is laid out, and this only measures where one of the row's items came
    // to rest. `bandRow` fills the band, so its children's coordinates are this item's.
    Item {
        id: seamRun
        x: bandRow.x + minimizeButton.x - bandRow.spacing
        // Nothing at all while the band is not the title bar: there are no window buttons to stand before, and a
        // `RowLayout` leaves an item it is not laying out at whatever geometry it last had.
        width: minimizeButton.visible ? bandRow.spacing : 0
        height: bandRow.height
        onXChanged: topBar.captionStripMoved()
        onWidthChanged: topBar.captionStripMoved()
        Component.onCompleted: topBar.captionStripMoved()
    }

    // ---- 操作パネル -------------------------------------------------------
    Rectangle {
        id: opsBand
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: bandRow.bottom
        anchors.bottom: parent.bottom
        color: Theme.bgRaised

        // The panel's two edges, in the ink the window divides itself with elsewhere (`SplitHandleBar`). Both, because
        // this row is a surface between two others — a line on one side alone reads as belonging to the nearer one.
        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            height: Theme.borderWidth
            color: Theme.borderSubtle
        }
        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: Theme.borderWidth
            color: Theme.borderSubtle
        }

        // ---- the three names -------------------------------------------
        // Flush with the window's own left edge, under the ☰ (the repository's mark shares its column — `markSeat`).
        RowLayout {
            id: opsRow
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            // As wide as the names ask for, and never into the actions: a name is cut only once the actions are
            // down to their end cells (`actionCap`).
            width: Math.min(topBar.picksWant,
                            Math.max(topBar.picksFloor, actionSeat.x - Theme.spaceLg))
            // None of its own: each picker already keeps a step inside its own box on both sides, and a second one
            // between them reads as a gap somebody had opened.
            spacing: 0

            OpsPicker {
                id: repoPick
                name: topBar.repoName
                blank: qsTr("No repository")
                pixelSize: Theme.fontLg
                // **Centred in the ☰'s own cell**, so the two marks that begin their rows stand on one line down the
                // window's edge — not the ☰'s step in, which is measured for a mark a size larger.
                //
                // No kind beside it, where the branch has one: the window's own subject is not a kind of anything.
                markSeat: Theme.railWidth
                // **The worktree is this name's own second run**, not a seat of its own: a seat would go with the
                // worktree's name whenever the tab stands in the repository's own worktree, and the door to the other
                // worktrees with it.
                trail: topBar.worktreeName
                given: topBar.opsRepoCut
                opened: standMenu.opened
                Layout.fillHeight: true
                Layout.preferredWidth: repoPick.drawnWidth
                Accessible.name: qsTr("Repository")
                onClicked: topBar.pressRepoName()
            }
            OpsPicker {
                id: branchPick
                name: topBar.branchName
                note: topBar.branchUpstream
                noteTone: topBar.upstreamGone ? Theme.warning : Theme.textMuted
                ahead: topBar.curPage === null ? 0 : topBar.curPage.pageWorktree.ahead
                behind: topBar.curPage === null ? 0 : topBar.curPage.pageWorktree.behind
                given: topBar.opsBranchCut
                opened: branchMenu.opened
                // `Loading…` while the page in front is still reading its HEAD (a tab come to the front reads git
                // again, `Hub::release_tab`): `No branch` meanwhile would say something untrue about the repository
                // (user decision). Both in the nameless `textMuted`. `No branch` with no page to read (no tab, a
                // repository that would not open) or a HEAD status names no branch for. The in-between is pictured
                // only by `ops-repo-pick`'s `pressed.png` (the frame after the press): the landing picture is past it.
                blank: topBar.curPage !== null && !topBar.curPage.openFailed
                       && !topBar.curPage.pageWorktree.headKnown
                       ? qsTr("Loading…") : qsTr("No branch")
                // A local branch is the accent wherever it is drawn (規約 §ref の種別); a detached HEAD is a state, and
                // takes the state's colour on mark and word alike.
                kind: "branch"
                kindTint: topBar.detachedHead ? Theme.warning : Theme.accent
                // **`iconMd`, the panel's own step** (デザイン規約 §アイコンの席 の ⑥; `OpsPicker` says how it stands):
                // a step and a half down closes the branch's two rings into a blob.
                kindSize: Theme.iconMd
                // **A mark standing in the row, not a letter of the name** (user decision — ask before changing): the
                // row's step, the mark's own air counting towards it (5px between the inks of `main`'s mark and name).
                // Not ⑦'s letter gap (the face's side bearing and one stem): that gap cannot be told from the letter
                // spacing, and the mark reads as the name's first character. The upstream's cloud keeps the same step.
                kindStep: Theme.spaceXs
                tone: topBar.detachedHead ? Theme.warning : Theme.textPrimary
                // No `markSeat`: only the repository's mark stands in a column (the ☰'s), so a seat wider than this
                // mark is air nobody asked for.
                //
                // **The same step as the repository's**: both chevrons open a name in this row, and one a step under
                // the other reads as the lesser door.
                pixelSize: Theme.fontLg
                // …and the same air in front of it: the first seat is a column and this one the mark's own size, so
                // without this the second chevron stands closer to what precedes it.
                markLeadIn: Math.max(0, repoPick.markAir - branchPick.markAir - repoPick.rightPadding)
                // …and the same air after it, to the ink of what it opens — less would read as the lesser door.
                markStep: repoPick.markAir + repoPick.markGap
                // **The last of the names keeps the air it opens with past its words** (`OpsPicker.endAir`).
                endAir: branchPick.leadAir
                Layout.fillHeight: true
                Layout.preferredWidth: branchPick.drawnWidth
                Accessible.name: qsTr("Branch")
                onClicked: topBar.pressBranchName()
            }
        }

        // Where this window is standing, at two grains: which repository, and which of its worktrees — one row apiece,
        // each folding its choices under it (規約 §メニュー).
        AppMenu {
            id: standMenu
            // **Dropped from the name it opens**, like the ☰'s card (`AppMenuButton`; header).
            //
            // **Hung off the name itself**, so "outside" is measured from it: under the default policy a press on the
            // name shuts the card and the click that follows opens it again — a name that never closes its card
            // (pinned by `tests/qml/tst_opspicker.qml`).
            parent: repoPick
            x: 0
            y: repoPick.height
            closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
            // No row here is held, so none needs the floor a hold does: each card fits its rows (`AppMenu.widthFloor`).
            widthFloor: 0
            // Only one heading wears a mark (the WORKTREES section's): the other holds its step, so the two words begin
            // on one x (`AppMenu.alignsHeadings`; user decision).
            alignsHeadings: true
            // **The word git uses**, singular like `RefBranchMenu`'s `BRANCH` and `RefTagMenu`'s `TAG`: it names the
            // one about to be chosen.
            AppMenu {
                id: repoSub
                title: qsTr("REPOSITORY")
                widthFloor: 0
                // **The repositories this window already has open**, less the one it is standing in — no history of
                // past ones. The row moves the window to that tab where it stands, so a tab in a linked worktree says
                // so here too. Their names stay words, not code (unlike the WORKTREE card's).
                Instantiator {
                    model: topBar.tabsModel
                    delegate: AppMenuItem {
                        id: repoRow
                        required property int index
                        required property string title
                        required property string worktree_name
                        text: repoRow.title
                        textNames: true
                        trail: repoRow.worktree_name
                        offered: repoRow.index !== topBar.tabsModel.currentIndex
                        onTriggered: topBar.tabsModel.setCurrentIndex(repoRow.index)
                    }
                    onObjectAdded: (at, object) => repoSub.insertItem(at, object)
                    onObjectRemoved: (at, object) => repoSub.removeItem(object)
                }
            }
            // **The mark is on the title row and nowhere else**, as on the left menu's section, in the WORKTREES
            // section's `success` (`NavSections`).
            AppMenu {
                id: worktreeSub
                /// The worktrees the card was built from as the stand card last opened (`pressRepoName`).
                property var rows: []
                title: qsTr("WORKTREE")
                titleKind: "tree"
                titleTint: Theme.success
                // Every row keeps the section's seat, so a worktree wearing the padlock and one wearing nothing begin
                // their names on one x (`NameCell`).
                keepsSeat: true
                widthFloor: 0
                // **Every worktree the left menu's WORKTREES section lists, whatever it is filtering to**
                // (`NavSectionModel.worktreeCard`), less the one this tab stands in, down that section's road
                // (`openRepositoryPathRequested`). Copied into `rows` at the press (`pressRepoName`), **never the
                // section's model handed to the Instantiator**: that model's rows are what the left menu shows
                // (`qmodel.rs` `row_count`, after the filter), so a filter would empty the card too — and the feed has
                // one receiver per section (`attach.rs`), so a second model cannot be attached to escape it.
                //
                // **Never left empty** (user decision — ask before changing): a heading that comes and goes with the
                // number of worktrees reads as a door this repository does not have. The repository's own worktree,
                // where it is the only one, stays on as a row a reader cannot choose, dressed as ever (house, branch
                // name; hover `You are in this worktree`) — less the tab's own, the card has no row, and
                // `AppMenu.delegate` drops a card with `offeredRows === 0` title row and all. `blockedReason` greys
                // it and the row's `rowPress` swallows the press. Headless: `ops-stand-worktrees`.
                Instantiator {
                    model: worktreeSub.rows
                    delegate: AppMenuItem {
                        id: worktreeRow
                        required property var modelData
                        /// Where the worktree is, as git lists it — what the row is pressed for.
                        readonly property string full: worktreeRow.modelData.full
                        readonly property string branch: worktreeRow.modelData.branch
                        readonly property string change: worktreeRow.modelData.change
                        /// Where this tab is standing. Compared the way the listing's own `current` is
                        /// (`nav::drain`): git prints one separator and Windows the other, and a path is not a name.
                        readonly property bool here:
                            topBar.curPage !== null
                            && GitFacts.samePath(worktreeRow.full, topBar.curPage.pageTab.repoPath)
                        readonly property bool homeWorktree: worktreeRow.change === "MAIN"
                        // **Named the way the left menu names it** (`NavRowBody.name`): the repository's own worktree
                        // by the branch it has out — its folder is already the tab's name — every other by its folder.
                        // As code with no mark of its own (user decision): the card's title wears the tree. Ground and
                        // face as a name in a sentence (デザイン規約 §ref の種別), cut in the middle past the ceiling.
                        codeName: worktreeRow.homeWorktree && worktreeRow.branch !== "" ? worktreeRow.branch
                                                                                        : worktreeRow.modelData.name
                        textNames: true
                        // …wearing that row's own mark and colours (`NavRowBody.seatMark` / `seatTint`). A mark about
                        // the row is not a heading.
                        headed: false
                        markKind: worktreeRow.change === "LOCKED" ? "lock"
                                : worktreeRow.change === "PRUNABLE" ? "bang"
                                : worktreeRow.homeWorktree ? "home" : ""
                        markTint: worktreeRow.change === "PRUNABLE" ? Theme.warning : Theme.textSecondary
                        // The branch that worktree has out, **in the column the left menu keeps it in**
                        // (`NavRowBody.branchSeat`). The main worktree's row is named by it already.
                        sideName: worktreeRow.homeWorktree ? "" : worktreeRow.branch
                        offered: !worktreeRow.here || worktreeSub.rows.length === 1
                        blockedReason: worktreeRow.here ? qsTr("You are in this worktree") : ""
                        onTriggered: topBar.curPage.openRepositoryPathRequested(worktreeRow.full)
                    }
                    onObjectAdded: (at, object) => worktreeSub.insertItem(at, object)
                    onObjectRemoved: (at, object) => worktreeSub.removeItem(object)
                }
            }
        }

        // The branches, less the one the window stands on — **the left menu's BRANCHES section, filed its way**
        // (`OpsBranchMenu`), down its road (`RepoPage.switchToRef`; header). Hung off the name as the stand card is.
        OpsBranchMenu {
            id: branchMenu
            page: topBar.curPage
            parent: branchPick
            x: 0
            y: branchPick.height
            closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
            onBranchPicked: name => topBar.curPage.switchToRef("branch", name)
        }

        // ---- the three actions -----------------------------------------
        // Fetch, push, stash, in that order (stash right of push: user decision — ask before changing). **These three
        // only**: the fixed targets speak for the repository as a whole; `pull` moves one branch, so it is a row on
        // that branch's menus, never a button here (`offers::pull`). **The word over its mark** (`stacked`) — the
        // panel is two lines deep, and one line here
        // floats in it. **Framed in every state** (`borderStrong`, `bgSurface` inside): this is the one row of the
        // window that is pressed, and a word with no frame reads as a label; a warning takes the frame's colour.
        Row {
            id: actionSeat
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            spacing: Theme.spaceXs
            // **Hard against the find button, wherever the names end** — centred, the set would move under the
            // reader's hand from tab to tab with the names' length.
            x: Math.max(0, opsBand.width - topBar.opsRightMargin - actionSeat.width)

            BandFetchButton {
                id: fetchButton
                anchors.verticalCenter: parent.verticalCenter
                stacked: true
                curPage: topBar.curPage
                busyLatched: topBar.holdFetchBusy
                widestText: topBar.widestAction
                widestCode: topBar.widestActionCode
                laidByBand: true
                foldRequested: topBar.actionsFolded
                // **The three ask for the same two numbers** — the box the set shares and the cap the panel's width
                // settles — and neither moves with a button's shape, so giving the word up cannot change the width
                // that decided to.
                width: topBar.actionCap
                // **Two lines deep whatever the branch writes** (`actionDepth`): held to one line, a button's word and
                // mark stand past its frame. Folded, the cell takes the panel's height and the frame keeps that depth:
                // the fold gives up the word, not the depth (`ActionButton.frameInset`).
                height: fetchButton.folded ? opsBand.height : topBar.actionDepth
                foldedDepth: topBar.actionDepth
            }
            BandPushButton {
                id: pushButton
                anchors.verticalCenter: parent.verticalCenter
                stacked: true
                curPage: topBar.curPage
                busyLatched: topBar.holdPushBusy
                widestText: topBar.widestAction
                widestCode: topBar.widestActionCode
                laidByBand: true
                foldRequested: topBar.actionsFolded
                width: topBar.actionCap
                height: pushButton.folded ? opsBand.height : topBar.actionDepth
                foldedDepth: topBar.actionDepth
            }
            BandStashButton {
                id: stashButton
                anchors.verticalCenter: parent.verticalCenter
                stacked: true
                curPage: topBar.curPage
                widestText: topBar.widestAction
                widestCode: topBar.widestActionCode
                laidByBand: true
                foldRequested: topBar.actionsFolded
                width: topBar.actionCap
                height: stashButton.folded ? opsBand.height : topBar.actionDepth
                foldedDepth: topBar.actionDepth
            }
        }

        // ---- the line between the three and the find ----------------------
        // Where the set ends — **the panel's one upright line** (user decision — ask before changing): no git command
        // stands behind the find, and the three are a set it is not part of. The band's no-line rule (header) covers
        // only the band, not this row. Centred in the `spaceMd` step between the two frames, touching neither (a pixel
        // touching reads as that frame's edge). **As tall as the frames, not the cells** (`frameInset`): top and bottom
        // on the frames' lines — shorter reads as a symbol floating between them; a folded cell is the panel's whole
        // height, and the frames keep their depth folded, so the line does not change either. Its step is paid for in
        // `opsRightMargin` at every window width, folded or not. Headless: `ops-panel`'s `rule=` (`findRuled`).
        Rectangle {
            id: findRule
            x: Math.round((actionSeat.x + actionSeat.width + findButton.x - width) / 2)
            anchors.verticalCenter: parent.verticalCenter
            width: Theme.borderWidth
            height: findButton.height - 2 * findButton.frameInset
            color: Theme.borderDefault
        }

        // ---- the find ----------------------------------------------------
        // At the panel's right end, under the window's buttons as the repository's name is under the ☰. Dressed as the
        // three: one button in a row of four wearing something else reads as another kind of control.
        ActionButton {
            id: findButton
            stacked: true
            anchors.right: parent.right
            anchors.rightMargin: topBar.findRightMargin
            anchors.verticalCenter: parent.verticalCenter
            kind: "search"
            // **Plain, and a capital to start**: the chip the three wear is for a spelling git would take, and a search
            // is none (規約 §git 用語のコード表記).
            text: qsTr("Search")
            // A step heavier than the chips beside it, so the four read as one weight: a UI face at this size puts
            // less ink down than the mono one a command is set in.
            wordWeight: Theme.fontWeightStrong
            // Measured for the three's longest wording and laid out at its width, so the ink lands at the same two
            // ends theirs does (`ActionButton.slack`).
            widestText: topBar.widestAction
            widestCode: topBar.widestActionCode
            // **The first word the panel gives up** (the header's order); the cell keeps **the three's width**
            // either way, so the four stand as one set of cells.
            foldRequested: topBar.findFolded
            laidByBand: true
            width: topBar.actionCap
            height: findButton.folded ? opsBand.height : topBar.actionDepth
            foldedDepth: topBar.actionDepth
            frameColor: Theme.borderStrong
            faceColor: Theme.bgSurface
            // A plan standing over the graph refuses this press wherever it comes from (`RepoPage.startFind`), so the
            // button says so rather than answering a hand with nothing.
            enabled: topBar.curPage !== null && !topBar.curPage.planShown
            // The freeze names itself, the way the stash button's does; with no tab open there is nothing on the panel
            // to ask about (規約 §無効).
            tip: {
                if (topBar.curPage === null)
                    return ""
                if (topBar.curPage.planShown)
                    return qsTr("A rebase plan is being composed — the graph it stands over cannot be searched")
                return qsTr("Find commits")
            }
            Accessible.name: qsTr("Find commits")
            onActivated: topBar.curPage.startFind()
        }
    }
}
