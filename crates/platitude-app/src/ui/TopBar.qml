pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

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
    /// …and whether the band laid out with the words (`bandAsked`) hands the strip and the group together what the
    /// real one does — the two rows share every other cell, so a cell the shadow does not mirror is the difference.
    readonly property bool bandShadowAgrees:
        Math.abs(stripAsked.width + (groupAsked.visible ? groupAsked.width : 0)
                 - tabStrip.width - (stateGroup.visible ? stateGroup.width : 0)) < 1
    readonly property bool stateCardOpen: stateGroup.stateCardOpen
    /// Whether the row has ever placed the group — one of the three things the stand-in pointer needs before a card
    /// comes up (`BandStateGroup.standInAsking`), and the only one a verb cannot otherwise read: a run that saw no
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

    /// A press on either name. **Each is a toggle**, like the ☰ (`AppMenuButton`): `open()` on a card already up does
    /// nothing, which reads as a name that cannot be pressed twice. Each card is assembled as it opens
    /// (`NavSectionModel.copyCard` / `OpsBranchMenu.offerFrom`); each says whether it opened a card.
    function pressRepoName() {
        if (standMenu.opened) {
            standMenu.close()
            return false
        }
        copySub.rows = topBar.curPage === null ? [] : topBar.curPage.pageWorktrees.copyCard()
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
    function openStandCopies() { return standMenu.openSub(copySub) }
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
        : copySub.opened ? "copies"
        : standMenu.opened ? "stand" : "none"
    readonly property int standRepoRows: repoSub.offeredRows
    readonly property int standCopyRows: copySub.offeredRows
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
    function pickCopyRow(leaf) {
        return topBar.pickIn(copySub, row => GitFacts.pathLeaf(row.full) === leaf)
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
    /// Whether the ☰'s card is standing (`TabStrip`). The runs are the parts of the band a press never reaches, so it
    /// hands them back to the scene while the card is up (`WindowChrome.captionYielded`).
    readonly property bool appMenuOpen: tabStrip.appMenuOpen

    /// The narrowest this chrome can be laid out at — one of the two numbers the window's floor is the larger of
    /// (`Main.floorWidth`). **The larger of its two rows**: the band's read off its layout minimum, the panel's off its
    /// parts at their floors, so a control added to either is counted without anybody remembering to.
    readonly property real floorWidth:
        Math.max(bandRow.Layout.minimumWidth + bandRow.anchors.rightMargin,
                 topBar.picksFloor + Theme.spaceLg + topBar.actionsFoldedWidth
                 + topBar.opsRightMargin)
    /// What the three actions come to once every one is a mark in an end cell — the panel's half of the floor above
    /// (規約 §窓の床「fetch / push / stash も畳んだ姿で数える」).
    readonly property real actionsFoldedWidth:
        actionSeat.children.length * Theme.railWidth
        + (actionSeat.children.length - 1) * actionSeat.spacing

    /// Keeps the two actions' waiting visual up past the operation that raised it, for a picture of the wait: a fast
    /// remote is done before one can be grabbed. False unless written from outside (デザイン規約 §進行中・長押しの定数).
    property bool holdPushBusy: false
    property bool holdFetchBusy: false
    readonly property string pushMode: pushButton.mode
    /// Automation: what the Stash button read the working tree as, and the edge `stash-state` waits on — every
    /// refusal photographs as the same dim button.
    readonly property string stashMode: stashButton.mode
    readonly property bool stashLive: stashButton.enabled
    /// Whether either action has a string to open under a pointer. **The string itself** — a disabled control takes
    /// hover and opens its attached ToolTip like any other (rules-refs の `hoverEnabled` の行), so what tells the two
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
    /// Automation: the push button's gesture is still under way (`ActionButton.gesturing`) — false again once the
    /// button blanked it, which is a press that sent nothing.
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
    /// others, and which wording is widest depends on the installed fonts. Settled once: as a binding, reading four
    /// text metrics to feed the buttons' widths is a loop to the engine.
    property string widestAction: ""
    property bool widestActionCode: false

    // ---- how the three give way as the panel runs short ----------------
    /// The floor a wording is cut down to — two characters of the family this band says its wordings in
    /// (`BandWidest.wordFloor`).
    readonly property real actionWordFloor: widest.wordFloor
    /// The narrowest cell that still holds a word, for the **set**: the widest of the three floors, since a cell that
    /// only fits the shortest wording's floor cuts the longest one past its own (`ActionButton.foldWidth`).
    readonly property real actionFold: Math.max(fetchButton.foldWidth,
                                                pushButton.foldWidth,
                                                stashButton.foldWidth)
    /// What the panel holds around the three before any width goes to a wording: the names **at their whole width**
    /// (the buttons' words go before a letter of a name does — 規約 §操作パネル の譲る順), the step between the two
    /// halves, the right end (`opsRightMargin`), and the seat's own spacing.
    readonly property real actionsBox:
        topBar.picksWant + Theme.spaceLg + topBar.opsRightMargin
        + (actionSeat.children.length - 1) * actionSeat.spacing
    /// The widest a button may be drawn at the panel's width now; under a whole wording each elides into it
    /// (`ActionButtonLabel.cap`).
    readonly property real actionCap:
        Math.max(Theme.railWidth,
                 Math.min(fetchButton.naturalWidth,
                          (topBar.width - topBar.actionsBox) / actionSeat.children.length))
    /// The two widths the shapes change at, for a run to put the window on: the narrowest panel that still says every
    /// wording whole, and the widest that has given all three up. Read off the cap's own arithmetic, so a driver never
    /// writes a second copy of it (`WindowBandActs`).
    readonly property int actionsWholeAt:
        Math.ceil(topBar.actionsBox + actionSeat.children.length * fetchButton.naturalWidth)
    readonly property int actionsFoldAt:
        Math.ceil(topBar.actionsBox + actionSeat.children.length * topBar.actionFold) - 1
    /// Where the words go. Said once for the set: three cells the row rounded differently must not come out in two
    /// different shapes.
    readonly property bool actionsFolded: topBar.actionCap < topBar.actionFold

    /// Automation: the two ends of the cap's travel and what the panel made of it at this width (`band-actions`) — a
    /// photograph says which shape landed, not which arithmetic put it there.
    readonly property int actionNaturalW: Math.round(fetchButton.naturalWidth)
    readonly property int actionFoldW: Math.round(topBar.actionFold)
    readonly property int actionCapW: Math.round(topBar.actionCap)
    readonly property int actionWordFloorW: Math.round(topBar.actionWordFloor)
    /// The widest wording actually **on** the panel now: the box holds every state's wording, and the run that has to
    /// land between "cut" and "given up" has to aim at the one being said (`band-actions`).
    readonly property int actionWantW: Math.round(Math.max(fetchButton.wordWant,
                                                           pushButton.wordWant,
                                                           stashButton.wordWant))
    /// The cell width at which that wording starts being cut: itself, plus everything the cell holds around a word.
    readonly property int actionCutW:
        Math.round(fetchButton.naturalWidth - fetchButton.wordBox + topBar.actionWantW)
    /// …and what one button came out as. Read off push — the one that says the longest wording and wears the frame,
    /// the `!` and the hold when its branch has diverged (`BandPushButton`).
    readonly property int actionCellW: Math.round(pushButton.width)
    readonly property int actionCellH: Math.round(pushButton.height)
    readonly property int actionWordW: Math.round(pushButton.wordRoom)
    readonly property int actionInkW: Math.round(pushButton.wordInk)
    /// Whether any of the three had to cut its wording — asked of the set, since which one says the widest wording
    /// depends on the fonts.
    readonly property bool actionWordCut:
        fetchButton.wordCut || pushButton.wordCut || stashButton.wordCut
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
    /// which the tab has the moment the window moves — a name taken from the page arrives after its repository does
    /// (規約 §操作パネル).
    readonly property string repoName: topBar.tabsModel.currentRepoName
    /// The folder of the copy this tab is standing in (規約 §別の作業コピーを読む). **Empty in the repository's own
    /// copy**, as the tab above is (規約 §タブの所作): the left panel names that copy by its branch, which is already
    /// the next name in this row.
    readonly property string copyName: topBar.tabsModel.currentCopyName
    /// Whether HEAD is on no branch at all — a state rather than a name, and coloured as one.
    readonly property bool detachedHead: topBar.curPage !== null && topBar.curPage.pageWt.detached
    /// The branch HEAD is on, or the marker for a HEAD that is on no branch at all.
    readonly property string branchName:
        topBar.curPage === null || !topBar.curPage.pageWt.headKnown ? ""
        : topBar.curPage.pageWt.detached ? qsTr("detached")
        : topBar.curPage.pageWt.branch
    /// What this branch is measured against — **the whole ref as git names it** (`origin/main`), not the remote alone:
    /// a branch can follow a remote one under another name, so `origin` would leave the reader guessing which. Empty
    /// with no upstream.
    readonly property string branchUpstream:
        topBar.curPage === null ? "" : topBar.curPage.pageWt.upstream
    /// Whether that upstream has gone from the remote — git's own `[gone]`, read where the left panel reads it
    /// (`NavSectionModel.headUpstreamGone`). The name stays; the colour says the far side is gone.
    readonly property bool upstreamGone:
        topBar.curPage !== null && topBar.curPage.pageBranches.headUpstreamGone !== ""
    /// Automation: the three names the panel came out with, and whether any of them had to be cut. A photograph holds
    /// what is drawn, not which of the three gave way.
    readonly property string opsNames: topBar.repoName + "/" + topBar.copyName + "/" + topBar.branchName
    readonly property bool opsNameCut: repoPick.nameCut || branchPick.nameCut
    /// Automation: whether every button at the panel's right end is as deep as what it draws (`fetchButton.height`
    /// says why) — a short one reads in a picture as a style of its own.
    readonly property bool actionsBoxed: [fetchButton, pushButton, stashButton, findButton].every(
        button => !button.visible || button.height >= button.implicitHeight - 0.5)
    /// …and whether every frame there is the panel's two lines deep, folded or not: a folded frame one line deep sits
    /// in a picture like any other box, and only its neighbours' say it shrank.
    readonly property bool actionsDeep: [fetchButton, pushButton, stashButton, findButton].every(
        button => !button.visible || Math.abs(button.height - 2 * button.frameInset - branchPick.pairHeight) < 0.5)
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
    /// **The order the names give way in** (規約 §操作パネル の譲る順): the repository's picker, then the branch's
    /// (`OpsPicker.given`) — the tab above says the repository again, and every write in this row acts on the branch.
    readonly property real opsRoom: opsRow.width
    /// How much more the names want than the row has — cut from each name in turn towards its floor, never by
    /// dropping one.
    readonly property real opsOver: Math.max(0, topBar.picksWant - topBar.opsRoom)
    /// The narrowest the names are ever laid out at — every word at its own floor.
    readonly property real picksFloor:
        repoPick.foldWidth + branchPick.foldWidth + opsRow.spacing
    /// …taken from the two in the order above. Each gives what it can and passes the rest on.
    readonly property real opsRepoCut: Math.min(topBar.opsOver, repoPick.slack)
    readonly property real opsBranchCut: Math.min(topBar.opsOver - topBar.opsRepoCut, branchPick.slack)
    /// What the panel keeps clear at its right end: the find button at its drawn width, the band's step in from the
    /// window edge, and a step before the actions, which are a set and would otherwise read as four.
    readonly property real opsRightMargin:
        topBar.findCap + bandRow.anchors.rightMargin + Theme.spaceMd
    /// The panel gives up its words in 規約 §操作パネル の譲る順 — **the buttons' before any name's, because a button
    /// that has given its word up is still read by its mark**, and a name has nothing to fall back to.
    ///
    /// What every wording standing whole costs: the width the first of them (the find's) starts giving way under.
    readonly property real chromeWholeWidth:
        topBar.picksWant + Theme.spaceLg
        + actionSeat.children.length * fetchButton.naturalWidth
        + (actionSeat.children.length - 1) * actionSeat.spacing
        + Theme.spaceMd + fetchButton.naturalWidth + bandRow.anchors.rightMargin
    readonly property bool findFolded: topBar.width < topBar.chromeWholeWidth
    readonly property real findCap:
        topBar.findFolded ? Theme.railWidth : fetchButton.naturalWidth

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
        // `spaceXs` in from the client area's edge, not flush: flush puts the close button's last pixels off screen
        // (規約 §ウィンドウの縁).
        anchors.rightMargin: topBar.captionMerged ? Theme.spaceXs : Theme.spaceMd
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
            // Asks for its names whole, as the group asks for its words: short of both, the row takes the shortfall
            // from the two at once, each by its room above its floor — Qt reads no stretch below what the items ask
            // for, so no order between them can be set here (規約 §ウィンドウの縁「タブと群は同時に譲る」). The group's
            // fold is its own (`BandStateGroup.folded`); room past both is the strip's, since the group asks for its
            // ceiling.
            onOpenRepositoryRequested: topBar.openRepositoryRequested()
            onCloneRepositoryRequested: topBar.cloneRepositoryRequested()
            onSettingsRequested: topBar.settingsRequested()
            // The Exit row is the ✕ by another name: one signal out of the band, so the window has one close gate.
            onExitRequested: topBar.closeRequested()
            onCaptionStripMoved: topBar.captionStripMoved()
        }

        BandStateGroup {
            id: stateGroup
            curPage: topBar.curPage
            windowAtFloor: topBar.windowAtFloor
            pointedAt: topBar.statePointedAt
            // **Narrowed and folded against the band as it would be with its words** (`bandAsked` says why).
            room: groupAsked.width
            tabContentWidth: tabStrip.contentWidth
            tabRunAvail: Math.max(0, tabStrip.runAvail + stripAsked.width - tabStrip.width)
            tabCount: tabStrip.tabCount
            // Measured off the fetch button: a target's size is the padding a Fusion `ToolButton` keeps around its
            // content, which the theme has no number for. Its `padding`, not the two sides — those carry the shared
            // box's slack (`ActionButton.slack`) and would move this group whenever fetch changed its wording.
            controlPadding: fetchButton.padding
            controlHeight: fetchButton.implicitHeight
            cellFolded: topBar.actionsFolded
            Layout.fillWidth: true
            Layout.fillHeight: stateGroup.cellFolded
            // **Folded, the mark's cell and not a pixel more** (規約 §ウィンドウの縁「畳んだ群が帯から取るのは印の箱だけ」).
            Layout.maximumWidth: stateGroup.folded ? stateGroup.foldedWidth : stateGroup.naturalWidth
            Layout.minimumWidth: stateGroup.foldedWidth
            onIdentityEditRequested: topBar.identityEditRequested()
        }
        // The window's own three, drawn here (規約 §ウィンドウの縁「窓ボタンはアプリが描く」).
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

    // The band's own step before the window's buttons, with nothing drawn in it. The hit test answers HTCAPTION for it
    // as for the run past the last tab (`TabStrip.grabArea`), so a press here is the platform's own gesture (規約
    // §ウィンドウの縁「何も受けない空きは窓の掴み所」).
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
                // **The copy is this name's own second run**, not a seat of its own (規約 §操作パネル).
                trail: topBar.copyName
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
                ahead: topBar.curPage === null ? 0 : topBar.curPage.pageWt.ahead
                behind: topBar.curPage === null ? 0 : topBar.curPage.pageWt.behind
                given: topBar.opsBranchCut
                opened: branchMenu.opened
                // `Loading…` while the page in front is still reading (規約 §操作パネル, `Hub::release_tab`).
                blank: topBar.curPage !== null && !topBar.curPage.openFailed && !topBar.curPage.pageWt.headKnown
                       ? qsTr("Loading…") : qsTr("No branch")
                // A local branch is the accent wherever it is drawn (規約 §ref の種別); a detached HEAD is a state, and
                // takes the state's colour on mark and word alike.
                kind: "branch"
                kindTint: topBar.detachedHead ? Theme.warning : Theme.accent
                // **The size the left list draws the same mark at** (`NavHeader`, 規約 §操作パネル).
                kindSize: Theme.iconMd
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
                // …and the same air after it, to the ink of what it opens (規約 §操作パネル).
                markStep: repoPick.markAir + repoPick.markGap
                // **The last of the names keeps the air it opens with past its words** (`OpsPicker.endAir`,
                // 規約 §操作パネル).
                endAir: branchPick.leadAir
                Layout.fillHeight: true
                Layout.preferredWidth: branchPick.drawnWidth
                Accessible.name: qsTr("Branch")
                onClicked: topBar.pressBranchName()
            }
        }

        // Where this window is standing, at two grains: which repository, and which of its copies — one row apiece,
        // each folding its choices under it (規約 §メニュー).
        AppMenu {
            id: standMenu
            // **Dropped from the name it opens**, like the ☰'s card (`AppMenuButton`; 規約 §操作パネル).
            //
            // **Hung off the name itself**, so "outside" is measured from it: under the default policy a press on the
            // name shuts the card and the click that follows opens it again — a name that never closes its card.
            parent: repoPick
            x: 0
            y: repoPick.height
            closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
            // No row here is held, so none needs the floor a hold does: each card fits its rows (`AppMenu.widthFloor`).
            widthFloor: 0
            // Only one heading wears a mark (the WORKTREES section's): the other holds its step, so the two words begin
            // on one x (`AppMenu.alignsHeadings`).
            alignsHeadings: true
            // **The word git uses**, singular like `RefBranchMenu`'s `BRANCH` and `RefTagMenu`'s `TAG` (規約 §操作パネル).
            AppMenu {
                id: repoSub
                title: qsTr("REPOSITORY")
                widthFloor: 0
                // **The repositories this window already has open**, less the one it is standing in (規約 §操作パネル).
                // The row moves the window to that tab where it stands, so a tab in a linked copy says so here too.
                Instantiator {
                    model: topBar.tabsModel
                    delegate: AppMenuItem {
                        id: repoRow
                        required property int index
                        required property string title
                        required property string copy_name
                        text: repoRow.title
                        trail: repoRow.copy_name
                        offered: repoRow.index !== topBar.tabsModel.currentIndex
                        onTriggered: topBar.tabsModel.setCurrentIndex(repoRow.index)
                    }
                    onObjectAdded: (at, object) => repoSub.insertItem(at, object)
                    onObjectRemoved: (at, object) => repoSub.removeItem(object)
                }
            }
            // **The mark is on the title row and nowhere else** (規約 §操作パネル), in the WORKTREES section's `success`
            // (`NavSections`).
            AppMenu {
                id: copySub
                /// The copies the card was built from as the stand card last opened (`pressRepoName`).
                property var rows: []
                title: qsTr("WORKTREE")
                titleKind: "tree"
                titleTint: Theme.success
                // Every row keeps the section's seat, so a copy wearing the padlock and one wearing nothing begin their
                // names on one x (`NameCell`).
                keepsSeat: true
                widthFloor: 0
                // **Every copy the left menu's WORKTREES section lists, whatever it is filtering to**
                // (`NavSectionModel.copyCard`), less the one this tab stands in, down that section's road
                // (`openRepositoryPathRequested`).
                //
                // **Never left empty** (規約 §操作パネル): the repository's own copy, where it is the only one, stays on
                // as a row a reader cannot choose.
                Instantiator {
                    model: copySub.rows
                    delegate: AppMenuItem {
                        id: copyRow
                        required property var modelData
                        /// Where the copy is, as git lists it — what the row is pressed for.
                        readonly property string full: copyRow.modelData.full
                        readonly property string branch: copyRow.modelData.branch
                        readonly property string change: copyRow.modelData.change
                        /// Where this tab is standing. Compared the way the listing's own `current` is
                        /// (`nav::drain`): git prints one separator and Windows the other, and a path is not a name.
                        readonly property bool here:
                            topBar.curPage !== null
                            && GitFacts.samePath(copyRow.full, topBar.curPage.pageTab.repoPath)
                        readonly property bool homeCopy: copyRow.change === "MAIN"
                        // **Named the way the left menu names it** (`NavRowBody.name`): the repository's own copy by
                        // the branch it has out — its folder is already the tab's name — every other by its folder.
                        text: copyRow.homeCopy && copyRow.branch !== "" ? copyRow.branch : copyRow.modelData.name
                        // …wearing that row's own mark and colours (`NavRowBody.seatMark` / `seatTint`). A mark about
                        // the row is not a heading.
                        headed: false
                        markKind: copyRow.change === "LOCKED" ? "lock"
                                : copyRow.change === "PRUNABLE" ? "bang"
                                : copyRow.homeCopy ? "home" : ""
                        markTint: copyRow.change === "PRUNABLE" ? Theme.warning : Theme.textSecondary
                        // The branch that copy has out, **in the column the left menu keeps it in**
                        // (`NavRowBody.branchSeat`). The main copy's row is named by it already.
                        sideName: copyRow.homeCopy ? "" : copyRow.branch
                        offered: !copyRow.here || copySub.rows.length === 1
                        blockedReason: copyRow.here ? qsTr("You are in this working copy") : ""
                        onTriggered: topBar.curPage.openRepositoryPathRequested(copyRow.full)
                    }
                    onObjectAdded: (at, object) => copySub.insertItem(at, object)
                    onObjectRemoved: (at, object) => copySub.removeItem(object)
                }
            }
        }

        // The branches, less the one the window stands on — **the left menu's BRANCHES section, filed its way**
        // (`OpsBranchMenu`), down its road (`RepoPage.switchToRef`; 規約 §操作パネル). Hung off the name as the stand
        // card is.
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
                wordFloor: topBar.actionWordFloor
                foldRequested: topBar.actionsFolded
                // **The three ask for the same two numbers** — the box the set shares and the cap the panel's width
                // settles — and neither moves with a button's shape, so giving the word up cannot change the width
                // that decided to.
                width: topBar.actionCap
                // **Two lines deep whatever the branch writes** (`OpsPicker.pairHeight`): held to one line, a button's
                // word and mark stand past its frame (規約 §操作パネル). Folded, the cell takes the panel's height and
                // the frame keeps those two lines: the fold gives up the word, not the depth.
                height: fetchButton.folded ? opsBand.height : branchPick.pairHeight
                foldedDepth: branchPick.pairHeight
            }
            BandPushButton {
                id: pushButton
                anchors.verticalCenter: parent.verticalCenter
                stacked: true
                curPage: topBar.curPage
                busyLatched: topBar.holdPushBusy
                widestText: topBar.widestAction
                widestCode: topBar.widestActionCode
                wordFloor: topBar.actionWordFloor
                foldRequested: topBar.actionsFolded
                width: topBar.actionCap
                height: pushButton.folded ? opsBand.height : branchPick.pairHeight
                foldedDepth: branchPick.pairHeight
            }
            BandStashButton {
                id: stashButton
                anchors.verticalCenter: parent.verticalCenter
                stacked: true
                curPage: topBar.curPage
                widestText: topBar.widestAction
                widestCode: topBar.widestActionCode
                wordFloor: topBar.actionWordFloor
                foldRequested: topBar.actionsFolded
                width: topBar.actionCap
                height: stashButton.folded ? opsBand.height : branchPick.pairHeight
                foldedDepth: branchPick.pairHeight
            }
        }

        // ---- the line between the three and the find ----------------------
        // Where the set ends (規約 §操作パネル). Its step is paid for in `opsRightMargin`, folded or not; **as tall as
        // the frames, not the cells** (`frameInset`).
        Rectangle {
            id: findRule
            x: Math.round((actionSeat.x + actionSeat.width + findButton.x - width) / 2)
            anchors.verticalCenter: parent.verticalCenter
            width: Theme.borderWidth
            height: findButton.height - 2 * findButton.frameInset
            color: Theme.borderDefault
        }

        // ---- the find ----------------------------------------------------
        // At the panel's right end, under the window's ✕ as the repository's name is under the ☰. Dressed like the
        // three: one button in a row of four wearing something else reads as another kind of control.
        ActionButton {
            id: findButton
            stacked: true
            anchors.right: parent.right
            anchors.rightMargin: topBar.bandRightMargin
            anchors.verticalCenter: parent.verticalCenter
            kind: "search"
            // **Plain, and a capital to start**: the chip the three wear is for a spelling git would take, and a search
            // is none (規約 §git 用語のコード表記).
            text: qsTr("Search")
            // A step heavier than the chips beside it, so the four read as one weight: a UI face at this size puts
            // less ink down than the mono one a command is set in.
            wordWeight: Font.DemiBold
            // Measured for the three's longest wording and laid out at its width, so the ink lands at the same two
            // ends theirs does (`ActionButton.slack`). **The natural width, not the share** — the share is what is left
            // once this button is paid for, so reading it here would close that ring.
            widestText: topBar.widestAction
            widestCode: topBar.widestActionCode
            // **The first word the panel gives up** (規約 §操作パネル の譲る順): folded, the button is an end cell
            // with its mark in it, like the three.
            foldRequested: topBar.findFolded
            wordFloor: topBar.actionWordFloor
            width: topBar.findCap
            height: findButton.folded ? opsBand.height : branchPick.pairHeight
            foldedDepth: branchPick.pairHeight
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
