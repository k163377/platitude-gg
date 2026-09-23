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
    /// platform where its empty runs sit — the gestures on those (drag, snap, the double-click, the window menu) are
    /// the platform's own, because the hit test calls them caption (`AppBackend.setCaptionStrips`).
    property bool captionMerged: false
    /// Which shape the middle button is in.
    property bool windowMaximized: false

    /// Automation: what the band came out to. A layout change can lose the window's own buttons, or the run of band
    /// left to take hold of, or push either off the end — and none of that shows in a screenshot taken where the
    /// platform draws no buttons at all. The strip's readings come back through the band (`WindowAutoActDriver`).
    readonly property real bandGrabRun: tabStrip.grabRun
    /// …and the band's other run, which a picture holds no better: the seam before the window's own buttons is empty
    /// band whether or not the hit test was ever told about it.
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
    /// Automation: the five things that can be the matter here. The group owns the conditions and the hidden
    /// measurements behind them; what the hooks ask the band for comes back through here (`BandStateGroup`).
    readonly property bool opBadgeShown: stateGroup.opBadgeShown
    readonly property bool conflictBadgeShown: stateGroup.conflictBadgeShown
    readonly property bool identityBadgeShown: stateGroup.identityBadgeShown
    readonly property bool oldGitBadgeShown: stateGroup.oldGitBadgeShown
    readonly property bool staleBadgeShown: stateGroup.staleBadgeShown
    readonly property real stateBadgeMinW: stateGroup.stateBadgeMinW

    /// Automation: which of the group's three shapes is on screen, what the badges were narrowed to, and what the card
    /// came back with (`PGG_AUTO_ACT=badges` / `badges-hover`). The group makes them; the band is where they are read.
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
    /// Whether the row has ever placed the group — the third of the three things the stand-in pointer needs before a
    /// card can come up (`BandStateGroup.standInAsking`), and the only one of them a verb cannot otherwise read. A run
    /// that stood the pointer in and never saw a card is two different faults depending on this.
    readonly property bool statePlaced: stateGroup.placed
    readonly property string stateCardRows: stateGroup.stateCardRows
    readonly property string stateCardSize: stateGroup.stateCardSize
    readonly property bool stateCardLaidOut: stateGroup.stateCardLaidOut
    /// Whether the window is standing on its floor. Handed in, because the floor is the larger of this band's and the
    /// page's and only the body that seats both has it (`WindowBody`). The group gives up its words there whatever
    /// else is true.
    property bool windowAtFloor: false
    /// Stands in for the pointer where headless cannot put one, so the card can be photographed (`badges-hover` /
    /// `identity-tip`). The real hover writes this same one property — hover is the input that cannot be injected, so
    /// the card has to be answering a single question or the headless run proves nothing about it.
    property bool statePointedAt: false
    /// Whether the fetch button can be pressed at all — the edge `fetch-tip` waits on, since what it reads is only
    /// worth anything once the band has settled on an answer.
    readonly property bool fetchLive: fetchButton.enabled

    /// A press on either name. **Each is a toggle**: the name is the way into its card and the way out of it, the way
    /// the ☰ is (`AppMenuButton`) — `open()` on a card already up does nothing, which reads as a name that can never
    /// be pressed a second time. The branch's card is assembled as it opens (`OpsBranchMenu.offerFrom`). Each says
    /// whether it opened a card.
    function pressRepoName() {
        if (standMenu.opened) {
            standMenu.close()
            return false
        }
        return standMenu.offerHere()
    }
    function pressBranchName() {
        if (branchMenu.opened) {
            branchMenu.close()
            return false
        }
        return branchMenu.offerFrom()
    }
    /// Automation: the panel's two doors, opened the way a press opens them — a press is the one input a headless run
    /// cannot make (app-ui.md §UI 自動化), so these go in at the press's own handler. Each says whether the card came
    /// up, which a picture of a window with no card in it cannot tell from a card that opened somewhere off screen.
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
    /// green on a card it was never about (observed: the copies' tier photographed for the repositories' run).
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
    /// Automation: what the two names are washed in and which way their chevrons point — **the output side**, the
    /// colour the wash was handed and the turn the mark is drawn at: a picture of a lit name and one of a name under
    /// a pointer are the same picture (`HoverToolButton.washColor`).
    readonly property bool repoNameLit: Qt.colorEqual(repoPick.washColor, Theme.bgHover)
    readonly property bool branchNameLit: Qt.colorEqual(branchPick.washColor, Theme.bgHover)
    readonly property bool repoNameTurned: repoPick.foldTurn === 90
    /// …and where the branch's counts came out against the two lines (`OpsPicker.trackPlace`).
    readonly property string branchTrackPlace: branchPick.trackPlace
    readonly property bool branchNameTurned: branchPick.foldTurn === 90
    /// Automation: a row of each card pressed — the row's own `triggered`, which is what a click on it emits, so the
    /// handler that runs is the row's and everything it reaches from there is the wiring a hand goes through
    /// (`AppMenuButton.clickCloneRow`). Each answers whether it found an offered row to press: a card is assembled
    /// from a listing that may not have landed, and a press at nothing is not one to wait on.
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
            if (!row || !row.offered)
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
    /// (`Main.floorWidth`). **The larger of its two rows**, each read off that row's *minimum* rather than summed
    /// here, so a control added to either one is counted without anybody remembering to.
    readonly property real floorWidth:
        Math.max(bandRow.Layout.minimumWidth + bandRow.anchors.rightMargin,
                 topBar.picksFloor + Theme.spaceLg + topBar.actionsFoldedWidth
                 + topBar.opsRightMargin)
    /// What the three actions come to once every one of them is a mark in an end cell — the panel's half of the floor
    /// above, counted in the folded shape the way the band's always has been (規約 §窓の床「帯の床は畳んだ姿で数える」).
    /// As many cells as there are actions, so one added later is counted without anybody remembering to.
    readonly property real actionsFoldedWidth:
        actionSeat.children.length * Theme.railWidth
        + (actionSeat.children.length - 1) * actionSeat.spacing

    /// Keeps the two actions' waiting visual up past the operation that raised it. Written from outside and false
    /// wherever nobody wrote it: a fast remote is done before a picture of the wait can be grabbed, and the wait is
    /// what the pair of buttons was drawn for — the framed one and the bare one
    /// (デザイン規約 §進行中・長押しの定数).
    property bool holdPushBusy: false
    property bool holdFetchBusy: false
    readonly property string pushMode: pushButton.mode
    /// Automation: what the Stash button read the working tree as, and the edge `stash-state` waits on. The reading
    /// is a binding over a HEAD and four counts, and a photograph of a dim button carries none of them — every
    /// refusal frames the same way.
    readonly property string stashMode: stashButton.mode
    readonly property bool stashLive: stashButton.enabled
    /// Whether either action has a string to open under a pointer. **The string itself** — a
    /// disabled control takes hover and opens its attached ToolTip like any other one (rules-refs/app-ui.md §hover),
    /// so what tells the two sides apart is whether there is anything to open (`fetch-tip`).
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
    /// the frame** — every button in this panel wears one at rest, so "wearing a frame" says nothing about the state
    /// it is in; what the third failure does is change that line's colour (`BandFetchButton.frameColor`). Read off
    /// the colour the button hands its frame, because a claim about a line's colour cannot be judged on a picture of
    /// a line that is there either way.
    readonly property int fetchFails: fetchButton.fails
    readonly property bool fetchFrameTurned: Qt.colorEqual(fetchButton.frameColor, Theme.warning)
    /// Automation: the Stash button, pressed (`PGG_AUTO_ACT=stash`). Put in at the button,
    /// so what answers is the band's real wiring. **Answers whether the
    /// press went in**: the band refuses it while the tab is busy — the fetch a repository does on the way open is one
    /// — and a shot fired at nothing is not one to latch, so the caller keeps offering it.
    function stashNow() {
        if (!stashButton.enabled)
            return false
        stashButton.clicked()
        return true
    }
    /// Automation: the Fetch button, pressed (`PGG_AUTO_ACT=fetch-resume`). Put in at the button for the reason
    /// `stashNow` is: the stopped button is the one place that asks for the timer back, so a run that called the slot
    /// behind it would pass a build where the press no longer reaches it. Answers whether the press went in.
    function fetchNow() {
        if (!fetchButton.enabled)
            return false
        fetchButton.clicked()
        return true
    }
    /// Automation: the panel's Find mark, pressed (`PGG_AUTO_ACT=band-find`). Put in at the button for the reason
    /// `stashNow` is: the mark and the key meet inside the page, so a run that called that would pass a build where
    /// the mark reaches nothing. Answers whether the press went in — it is down while a plan stands over the graph.
    function findNow() {
        if (!findButton.enabled)
            return false
        findButton.clicked()
        return true
    }
    function completePushHold() {
        pushButton.completeHold()
    }

    /// Automation: the strip's own hooks, handed on. What `Main` and `WindowAutoActDriver` hold is the band, so the way
    /// in stays here after the tabs themselves have gone (`TabStrip`).
    function clickAppMenu() { tabStrip.clickAppMenu() }
    function clickCloneRow() { tabStrip.clickCloneRow() }
    function middleClickTab(index) { return tabStrip.middleClickTab(index) }
    function dragTabTo(from, to) { return tabStrip.dragTabTo(from, to) }
    function holdTabAt(index) { return tabStrip.holdTabAt(index) }
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

    /// The word all three toolbar buttons are measured for: one box for the set keeps any of them from shifting the
    /// others, and which wording is wider is a question about the installed fonts. Settled once: a
    /// binding that reads four text metrics and feeds three button widths is a loop as far as the engine is concerned.
    property string widestAction: ""
    property bool widestActionCode: false

    // ---- how the three give way as the panel runs short ----------------
    // The names give first — they can be cut and still be read — and the actions keep their words for as long as
    // there is room for them beside the names at their own floor. **Not a stretch counted back from the window's
    // floor**: the actions stand in the middle of the panel now, and a schedule read off the window's width shortens
    // the wordings while the panel still has room to the right of them — a thing nobody can see a reason for.
    /// The floor a wording is cut down to — two characters of the family this band says its wordings in
    /// (`BandWidest.wordFloor`).
    readonly property real actionWordFloor: widest.wordFloor
    /// The narrowest cell that still holds a word, for the **set**: the widest of the three floors, since a cell that
    /// only fits the shortest wording's floor cuts the longest one past its own (`ActionButton.foldWidth`). It moves
    /// with what the buttons are saying — a longer wording gives up sooner, which is the rule the shared box is under.
    readonly property real actionFold: Math.max(fetchButton.foldWidth,
                                                pushButton.foldWidth,
                                                stashButton.foldWidth)
    /// What the panel holds around the three: the names cut down as far as they go, the step between the two halves,
    /// the close button's cell at the far end, and the spacing inside the seat itself. Everything a width has to carry
    /// before any of it can go to a wording.
    /// **The names at their whole width**, not cut down as far as they go: the words on the buttons are what the
    /// panel gives up before a letter of a name does, so what the three are measured against is the row the names
    /// actually want (§譲る順).
    readonly property real actionsBox:
        topBar.picksWant + Theme.spaceLg + topBar.opsRightMargin
        + (actionSeat.children.length - 1) * actionSeat.spacing
    /// The widest a button may be drawn at the width the panel is standing at now: an even share of whatever is left
    /// once that box is paid for, capped at the width a whole wording wants and floored at the band's end cell. Every
    /// wording is whole while the share covers one; under that each elides into what is left
    /// (`ActionButtonLabel.cap`); at the floor every one of them is a mark in an end cell.
    readonly property real actionCap:
        Math.max(Theme.railWidth,
                 Math.min(fetchButton.naturalWidth,
                          (topBar.width - topBar.actionsBox) / actionSeat.children.length))
    /// The two widths the shapes change at, which is what a run has to put the window on to photograph one: the
    /// narrowest panel that still says every wording whole, and the widest one that has given all three up. Read off
    /// the same arithmetic the cap is, so a driver never writes a second copy of it (`WindowBandActs`).
    readonly property int actionsWholeAt:
        Math.ceil(topBar.actionsBox + actionSeat.children.length * fetchButton.naturalWidth)
    readonly property int actionsFoldAt:
        Math.ceil(topBar.actionsBox + actionSeat.children.length * topBar.actionFold) - 1
    /// Where the words go. Said once for the set: three cells the row rounded differently must not come out in two
    /// different shapes.
    readonly property bool actionsFolded: topBar.actionCap < topBar.actionFold

    /// Automation: the two ends of the cap's travel and what the band made of it at this width (`band-actions`). A
    /// photograph says which shape landed but not which arithmetic put it there, and the widths the three shapes sit
    /// at are a question about the installed fonts.
    readonly property int actionNaturalW: Math.round(fetchButton.naturalWidth)
    readonly property int actionFoldW: Math.round(topBar.actionFold)
    readonly property int actionCapW: Math.round(topBar.actionCap)
    readonly property int actionWordFloorW: Math.round(topBar.actionWordFloor)
    /// The widest wording actually **on** the band right now: the
    /// box holds every state's wording, and the run that has to land between "cut" and "given up" has to aim at the
    /// one being said (`band-actions`).
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
    /// Whether any of the three had to cut its wording. Asked of the set: the cell is shared, so
    /// the widest wording is the first to be cut, and which of the three is saying it is a question about the fonts.
    readonly property bool actionWordCut:
        fetchButton.wordCut || pushButton.wordCut || stashButton.wordCut
    /// Whether either of the two that can wear a `!` is wearing one. Asked of the pair: the run
    /// that can stage a refusal without a network is the fetch that cannot reach its remote (`band-actions-alert`).
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
    /// (`PGG_AUTO_ACT=tab-widths` / `badges`). Re-exposed for the reason the band's other readings are.
    readonly property real tabTitleCap: tabStrip.tabTitleCap
    readonly property int tabTitleMinW: tabStrip.tabTitleMinW
    readonly property real tabTitleEaseW: tabStrip.tabTitleEaseW
    readonly property int tabTitleMaxW: tabStrip.tabTitleMaxW
    /// The room every tab is keeping for its mark, which is what the strip gives up first — the two ends it is settled
    /// between are the strip's own (`TabMetrics.markRoomFull` / `markRoomMin`). A picture of a strip that has given it
    /// all up reads the same as one that has not had to.
    readonly property real tabMarkRoom: tabStrip.tabMarkRoom
    /// And which of the two states that leaves the strip in — the reading a run is judged on, since the count that
    /// reaches either is the platform's answer and not the run's (`TabStrip.tabNamesCut`).
    readonly property bool tabNamesCut: tabStrip.tabNamesCut
    readonly property bool tabMarksFolded: tabStrip.tabMarksFolded

    /// What the panel under the band says this window is standing in: **the tab in front's own names**
    /// (`TabsModel.currentRepoName`). The strip has them the moment the window moves — git named both places when the
    /// tab was made — where the page moved to has read nothing yet, and a name taken from the page arrives after its
    /// repository does (規約 §操作パネル).
    readonly property string repoName: topBar.tabsModel.currentRepoName
    /// The working copy — the folder of the copy this tab is standing in, which is what tells one worktree of a
    /// repository from another (規約 §別の作業コピーを読む).
    ///
    /// **Empty in the repository's own copy**, the way the tab above says nothing there either (規約 §タブの所作).
    /// The left panel names that copy by the branch it holds rather than by its folder, and the branch is already the
    /// next name in this row — a word here would be the row saying the same thing twice, in a language the panel
    /// beside it does not use. **A run that appears is the row saying "this is not the usual place".**
    readonly property string copyName: topBar.tabsModel.currentCopyName
    /// Whether HEAD is on no branch at all — a state rather than a name, and coloured as one.
    readonly property bool detachedHead: topBar.curPage !== null && topBar.curPage.pageWt.detached
    /// The branch HEAD is on, or the marker for a HEAD that is on no branch at all.
    readonly property string branchName:
        topBar.curPage === null || !topBar.curPage.pageWt.headKnown ? ""
        : topBar.curPage.pageWt.detached ? qsTr("detached")
        : topBar.curPage.pageWt.branch
    /// What this branch is measured against, where it has one — **the whole ref as git names it** (`origin/main`)
    /// rather than the remote out of it. The remote alone does not say what the measuring is against: a branch can
    /// follow a remote one under another name, and two local branches can follow the same remote one, so `origin`
    /// beside `feature` leaves the reader to guess which branch on it the counts are from. Empty for a branch with no
    /// upstream, which is the branch the panel says nothing extra about because there is nothing to measure it by.
    readonly property string branchUpstream:
        topBar.curPage === null ? "" : topBar.curPage.pageWt.upstream
    /// Whether that upstream has gone from the remote — git's own `[gone]`, read where the left panel reads it
    /// (`NavSectionModel.headUpstreamGone`). The name stays on screen: what a branch is measured against is still
    /// what it is measured against, and the colour is what says the far side is not there any more.
    readonly property bool upstreamGone:
        topBar.curPage !== null && topBar.curPage.pageBranches.headUpstreamGone !== ""
    /// Automation: the three names the panel came out with, and whether any of them had to be cut. A photograph holds
    /// what is drawn, not which of the three gave way.
    readonly property string opsNames: topBar.repoName + "/" + topBar.copyName + "/" + topBar.branchName
    readonly property bool opsNameCut: repoPick.nameCut || branchPick.nameCut
    /// Automation: whether every button at the panel's right end is as deep as what it draws — one laid out shorter
    /// than its own two lines stands its word over its frame and its mark under it, and a picture of it reads as a
    /// style of its own until it is set beside a panel that has an upstream to write (`OpsPicker.pairHeight`).
    readonly property bool actionsBoxed: [fetchButton, pushButton, stashButton, findButton].every(
        button => !button.visible || button.height >= button.implicitHeight - 0.5)

    /// What the names ask for between them when nothing has given way — the run the actions may not be centred into
    /// (`actionSeat.x`), and the width the row is held to when there is room for all of it. **Read off the whole
    /// figure**, not what is drawn now: what the row can afford is decided from this, so a sum that already held the
    /// answer would close the ring.
    readonly property real picksWant:
        repoPick.wholeWidth + branchPick.wholeWidth + opsRow.spacing
    /// **The order the names give way in** (§譲る順 past the buttons): the repository's picker first — its copy's run,
    /// then its name — and the branch's last, the upstream before the name wherever the upstream is the longer of its
    /// two lines (`OpsPicker.given`). The repository is the one the tab above says again, and the branch is what every
    /// write in this row acts on.
    readonly property real opsRoom: opsRow.width
    /// How much more the names want than the row has. **Nothing is dropped to close it** — each name is cut towards
    /// its own floor, in turn, and every one of them is still on screen at the floor.
    readonly property real opsOver: Math.max(0, topBar.picksWant - topBar.opsRoom)
    /// The narrowest the names are ever laid out at — every word at its own floor, and **all of them still drawn**.
    readonly property real picksFloor:
        repoPick.foldWidth + branchPick.foldWidth + opsRow.spacing
    /// …taken from the two in the order above. Each gives what it can and passes the rest on.
    readonly property real opsRepoCut: Math.min(topBar.opsOver, repoPick.slack)
    readonly property real opsBranchCut: Math.min(topBar.opsOver - topBar.opsRepoCut, branchPick.slack)
    /// The seat the two smaller chevrons stand in. **The same proportion to their own mark that the repository's keeps
    /// to its** — its seat is the ☰'s cell and its mark is a step larger, so the two written as one ratio put the same
    /// amount of air, to the eye, around every chevron in the row.
    readonly property int opsMarkSeat: Math.round(Theme.iconSm * Theme.railWidth / Theme.iconMd)
    /// What the panel keeps clear at its right end for everything laid out in it: the find button, the band's own step
    /// in from the window edge, and a step between it and the actions, which are a set and would otherwise read as
    /// four. Its whole width, because that is what it is drawn at — the share the three divide between them is what is
    /// left once this is paid for. Nothing in the panel stands under a window button (規約 §ウィンドウの縁 —
    /// 帯の両端のセル).
    readonly property real opsRightMargin:
        topBar.findCap + bandRow.anchors.rightMargin + Theme.spaceMd
    /// **What the panel gives up, in order**: the find button's word, then the three
    /// commands' words, then the working copy, the repository's name, the upstream, and last the branch. **The words
    /// on the buttons go first because a button that has given its word up is still read by its mark** — a name has
    /// no mark to fall back to, and the branch is what every write in this row acts on, so it is the last thing left.
    ///
    /// What every wording standing whole costs, which is the width the first of them starts giving way under.
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
        // The band's own right edge is not the window's: the client area reaches past what is drawn, so a row flush
        // with it puts the close button's last few pixels off screen and its wash reads as clipped. `spaceXs` lands
        // it flush.
        anchors.rightMargin: topBar.captionMerged ? Theme.spaceXs : Theme.spaceMd
        spacing: Theme.spaceXs
        TabStrip {
            id: tabStrip
            tabsModel: topBar.tabsModel
            // The band's own ground, said by the band: a tab that is not in front paints none of its own, so this is
            // what its name is read against — and what the name goes quiet into under its mark.
            bandColor: topBar.color
            captionMerged: topBar.captionMerged
            curPage: topBar.curPage
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumWidth: tabStrip.tabStripFloorW
            // The band's order when it runs short: the tab names narrow together, then the state group's words do,
            // then the strip scrolls, then the group becomes a mark — a stretch this much larger than the group's puts
            // the strip at the front of both queues.
            Layout.horizontalStretchFactor: 100
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
            // **What the group's words are narrowed and given up against is the band as it would be with them**
            // (`bandAsked`): the room the row would hand the words, and the run the strip would have beside them. A
            // group that has given its words up is laid out at its mark alone, so reading its own width or the strip's
            // run beside it would read the room its folding handed out — and a group folded for want of room would
            // never find the room to unfold into.
            room: groupAsked.width
            tabContentWidth: tabStrip.contentWidth
            tabRunAvail: Math.max(0, tabStrip.runAvail + stripAsked.width - tabStrip.width)
            tabCount: tabStrip.tabCount
            // Measured off the pair beside it: what sets how big a target is
            // here is the padding a Fusion `ToolButton` keeps around its content — a number the theme does not have.
            // The button's `padding`: the two sides it settles to carry the shared box's slack as
            // well (`ActionButton.slack`), so reading them would move this group whenever fetch changed its wording.
            controlPadding: fetchButton.padding
            controlHeight: fetchButton.implicitHeight
            cellFolded: topBar.actionsFolded
            Layout.fillWidth: true
            Layout.fillHeight: stateGroup.cellFolded
            // **Folded, the mark's cell and not a pixel more**: the width the words were asking for goes back to the
            // strip, where it held nothing but air between the tabs' grab run and the mark.
            Layout.maximumWidth: stateGroup.folded ? stateGroup.foldedWidth : stateGroup.naturalWidth
            Layout.minimumWidth: stateGroup.foldedWidth
            // Second in both queues (the strip's comment carries the order).
            Layout.horizontalStretchFactor: 1
            onIdentityEditRequested: topBar.identityEditRequested()
        }
        // The window's own three, drawn here: the platform's cannot be styled and its
        // maximize mark never becomes a restore mark (P3-確認事項 §ウィンドウ chrome). **Nothing is drawn between them
        // and what the app owns** (規約 §ウィンドウの縁): the boxes keep the band's own step apart.
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

    // **The band as it would be laid out with the state group asking for its words** — the same constraints the row
    // above puts on each cell, less the one thing that differs: the group here never gives its words up. What the
    // group is narrowed against and folded by is read off this (`stateGroup.room` / `tabRunAvail`), because the row
    // above lays a folded group out at its mark and hands the rest to the strip. Unfolded, the two rows agree cell for
    // cell. Nothing in it is drawn or takes a press: plain items, measured and nothing else.
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
            Layout.horizontalStretchFactor: 100
        }
        Item {
            id: groupAsked
            visible: stateGroup.visible
            implicitWidth: stateGroup.naturalWidth
            Layout.fillWidth: true
            Layout.maximumWidth: stateGroup.naturalWidth
            Layout.minimumWidth: stateGroup.foldedWidth
            Layout.horizontalStretchFactor: 1
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

    // The seam between the last thing the app can be asked and the first thing the window can — the band's own step
    // before the window's buttons, with nothing drawn in it. The hit test answers HTCAPTION for it the way it does for
    // the run past the last tab (`TabStrip.grabArea`), so a press here is the platform's own gesture (規約
    // §ウィンドウの縁「何も受けない空きは窓の掴み所」).
    //
    // Outside the row, so that measuring the row's own layout does not become part of it: a child of a `RowLayout` is
    // laid out, and this one only wants to know where one of the row's items came to rest. `bandRow` fills the band,
    // so its children's coordinates are this item's.
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
    // What the window is standing in, and what is done to it. A step up from the band above rather than the same
    // surface twice — but only half a step (`Theme.bgRaised`): a whole one made this the brightest wide surface in a
    // window that is otherwise graph ground, and then everything drawn on it had to shout to be seen.
    Rectangle {
        id: opsBand
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: bandRow.bottom
        anchors.bottom: parent.bottom
        color: Theme.bgRaised

        // The panel's two edges, in the ink the window already divides itself with (`SplitHandleBar` — the line
        // between the graph and the commit beside it). One window, one way of saying "these are two things".
        //
        // Both edges, because this row is a surface of its own between two others — a line on one side alone would
        // read as belonging to whichever side it was nearer. They are the only lines in the window that run its
        // whole width.
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
        // Flush with the window's own left edge: the repository is the first thing this window says, and the ☰ above
        // it is the only other thing that starts there. Its mark stands in a seat of the ☰'s own width
        // (`OpsPicker.markSeat` = `railWidth`), so the name begins where the tabs above it begin.
        RowLayout {
            id: opsRow
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            // As wide as the names ask for, and never into the actions: what the panel runs short of is taken from
            // here, because the actions are three fixed boxes and a name can be cut.
            width: Math.min(topBar.picksWant,
                            Math.max(topBar.picksFloor, actionSeat.x - Theme.spaceLg))
            // None of its own: each picker already keeps a step inside its own box on both sides, and a second one
            // laid between them read as a gap somebody had opened.
            spacing: 0

            OpsPicker {
                id: repoPick
                name: topBar.repoName
                blank: qsTr("No repository")
                // The one that holds the other two, and said so with the word a step up the ramp
                // (規約 §タイポグラフィ — 見出しは重み・色が作る、段は役割で選ぶ).
                pixelSize: Theme.fontLg
                // **Centred in the ☰'s own cell**, so the two marks that begin their rows stand on one line down the
                // window's edge. Not the same step in as the ☰: that mark is a size larger, and a step copied off it
                // would put a smaller mark's middle two pixels to the left of it (observed). The cell is what the two
                // share, so the cell is what this is measured from.
                //
                // It stands alone where the other two carry a kind beside them: the window's own subject is not a kind
                // of anything.
                //
                // **This mark stands in the ☰'s cell**, the way the ☰ does: one column at the window's edge, one mark
                // centred in it, and what follows beginning at its far edge. The name needs no step of its own after
                // that — the cell is the air — so it starts against the cell the way the tabs do in the band above,
                // and nothing reaches back into the column the window's own mark owns.
                markSeat: Theme.railWidth
                // **The copy is this name's own second run**, not a seat of its own beside it (規約 §タブの所作 と同じ
                // 形): where the reader is standing is one question at two grains, and a seat that came and went with
                // the answer would take the door to the other copies with it every time the window stood in the
                // repository's own.
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
                // **Said as what it is while the page in front is reading**: a tab moved to reads its repository from
                // the start (`Hub::release_tab`), and until its status answers, "no branch" would be a claim about the
                // repository that is not so (規約 §操作パネル). A page whose repository would not open is reading
                // nothing, and one whose status answered has said what HEAD is.
                blank: topBar.curPage !== null && !topBar.curPage.openFailed && !topBar.curPage.pageWt.headKnown
                       ? qsTr("Loading…") : qsTr("No branch")
                // A local branch is the accent wherever it is drawn — the left panel's BRANCHES section, a graph row's
                // chip, and this mark (規約 §ref の種別). A detached HEAD is not a branch but a state, and takes the
                // state's colour on both the mark and the word: it is the one thing in this row a reader has to be
                // told rather than reminded of.
                kind: "branch"
                kindTint: topBar.detachedHead ? Theme.warning : Theme.accent
                // **The size the left list draws the same mark at** (`NavHeader`, the BRANCHES section): this is the
                // one mark in the panel a reader meets twice in the window, and the branch's two rings closed into
                // a blob a step and a half under it.
                kindSize: Theme.iconMd
                tone: topBar.detachedHead ? Theme.warning : Theme.textPrimary
                // **The mark's own size is the seat**: the
                // column at the window's edge is the repository's, because the ☰ above owns that column and the name
                // under it begins where the tabs do. Nothing here stands in a column, so a seat wider than the mark
                // is air nobody asked for — at `opsMarkSeat` it came to 11px between the chevron and the branch.
                //
                // **The same step as the one beside it**: both chevrons open a name in this row, and one drawn a
                // step under the other read as the lesser door rather than as the same door twice.
                pixelSize: Theme.fontLg
                // …and the same air in front of it. The first seat is a column and this one is the mark's own size,
                // so without this the second chevron stands closer to what precedes it than the first one does.
                markLeadIn: Math.max(0, repoPick.markAir - branchPick.markAir - repoPick.rightPadding)
                // …and the same air after it, to the ink of what it opens — the first chevron's is the column's own
                // half. Left to the family's step, the second chevron stands against its mark at half the distance
                // the first one keeps from its name, and reads as the lesser door.
                markStep: repoPick.markAir + repoPick.markGap
                // **The last of the names keeps the air it opens with past its words** (`OpsPicker.endAir`): its wash
                // is the one a hand sees end in the open panel, and nothing after it takes a share of its step.
                endAir: branchPick.leadAir
                Layout.fillHeight: true
                Layout.preferredWidth: branchPick.drawnWidth
                Accessible.name: qsTr("Branch")
                onClicked: topBar.pressBranchName()
            }
        }

        // Where this window is standing, and the two grains it can be moved at: which repository, and which of that
        // repository's copies. One row apiece, each folding its own choices under it — the shape every other menu in
        // the app has (規約 §メニュー).
        AppMenu {
            id: standMenu
            // **Dropped from the name it opens**, the way the ☰'s card is dropped from the ☰ (`AppMenuButton`): this
            // door is a word in a row rather than a right-click, and a card left where the pointer was would open in
            // a different place every time the same word was pressed.
            //
            // **Hung off the name itself**, so the name is what "outside" is measured from: the default policy calls
            // a press on the name outside and shuts the card on it, and the click that follows the same press opens it
            // again — a name that opens its card and never closes it (`AppMenuButton` answers the ☰ the same way).
            parent: repoPick
            x: 0
            y: repoPick.height
            closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
            // None of the panel's cards holds a held row, so none is held to the floor a hold needs: each fits its
            // rows (`AppMenu.widthFloor`).
            widthFloor: 0
            // Both rows are headings and one of them wears the WORKTREES section's mark: the other holds its step, so
            // the two words begin on one x (`AppMenu.alignsHeadings`).
            alignsHeadings: true
            // **The word git uses**, and singular like the two cards that already open this way
            // (`RefBranchMenu` = `BRANCH`, `RefTagMenu` = `TAG`): the row names the one thing about to be chosen,
            // not the list behind it.
            AppMenu {
                id: repoSub
                title: qsTr("REPOSITORY")
                widthFloor: 0
                // **The repositories this window already has open**, less the one it is standing in: a tab is a
                // repository the reader has already chosen, and the row moves the window to it rather than opening
                // anything. What was open in some earlier run is a listing this window does not keep
                // (P3-確認事項 §app).
                //
                // **A tab standing in a linked copy says so here too** — what the row opens is that tab where it
                // stands, not the repository's own copy, and the two are different places.
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
            // **The mark is on the title row and nowhere else** — the same place `RefBranchMenu` puts its branch and
            // the left panel puts the WORKTREES section's own, in the same `success` (`NavSections`). The rows under
            // it carry none, the way that section's rows carry none.
            AppMenu {
                id: copySub
                title: qsTr("WORKTREE")
                titleKind: "tree"
                titleTint: Theme.success
                // Every row keeps the section's seat, so a copy wearing the padlock and one wearing nothing begin their
                // names on one x, the way the section's rows do (`NameCell`).
                keepsSeat: true
                widthFloor: 0
                // **The same listing the left menu's WORKTREES section draws**, less the copy this tab is already
                // standing in, and down the same road a row of that section takes (`openRepositoryPathRequested`).
                // The section has no folders in it, so every row here is a copy.
                Instantiator {
                    model: topBar.curPage === null ? null : topBar.curPage.pageWorktrees
                    delegate: AppMenuItem {
                        id: copyRow
                        required property string name
                        required property string full
                        required property string bucket
                        required property string change
                        required property bool folder
                        /// Where this tab is standing. Compared the way the listing's own `current` is
                        /// (`nav::drain`): git prints one separator and Windows the other, and a path is not a name.
                        readonly property bool here:
                            topBar.curPage !== null
                            && GitFacts.samePath(copyRow.full, topBar.curPage.pageTab.repoPath)
                        readonly property bool homeCopy: copyRow.change === "MAIN"
                        // **Named the way the left menu names it**: the repository's own copy by the branch it has
                        // out, every other by its folder — the folder of the main copy is the tab's name and the
                        // window's title already (`NavRowBody.name`). Its folder is what is left when it holds no
                        // branch.
                        text: copyRow.homeCopy && copyRow.bucket !== "" ? copyRow.bucket : copyRow.name
                        // …wearing that row's own mark, in that row's own colours: the house on the repository's
                        // own copy, the padlock on one somebody locked, the warning on one git can no longer find
                        // (`NavRowBody.seatMark` / `seatTint`). A mark about the row is not a heading.
                        headed: false
                        markKind: copyRow.change === "LOCKED" ? "lock"
                                : copyRow.change === "PRUNABLE" ? "bang"
                                : copyRow.homeCopy ? "home" : ""
                        markTint: copyRow.change === "PRUNABLE" ? Theme.warning : Theme.textSecondary
                        // The branch that copy has out, **in the column the left menu keeps it in** — at the row's far
                        // end, a step down and in the quieter ink (`NavRowBody.branchSeat`): a fact about the copy, where
                        // the copy's own name is what the row is read for. The main copy's row is named by it already,
                        // and saying it twice would be this row's one fact said twice.
                        sideName: copyRow.homeCopy ? "" : copyRow.bucket
                        offered: !copyRow.folder && !copyRow.here
                        onTriggered: topBar.curPage.openRepositoryPathRequested(copyRow.full)
                    }
                    onObjectAdded: (at, object) => copySub.insertItem(at, object)
                    onObjectRemoved: (at, object) => copySub.removeItem(object)
                }
            }
        }

        // The branches this repository has, less the one the window is already standing on — **the left menu's
        // BRANCHES section, filed the way it files them** (`OpsBranchMenu`). Down the same road that section's rows
        // take (`RepoPage.switchToRef`), which is also what answers for a branch another copy is standing on: it goes
        // to that copy rather than refusing. Hung off the name for the reason the stand card is.
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
        // In the middle of the panel, and pushed right by the names when they need the room — never over them, and
        // never past the close button's cell. Placed rather than laid out, because "centred in the panel" is a
        // question about the panel's width and a row can only centre what is left after its other items.
        Row {
            id: actionSeat
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            spacing: Theme.spaceXs
            // **Hard against the find button, wherever the names end**. Centred, the set
            // stood in a different place in every repository — the middle of the panel is a question about how long
            // the names are, and the answer moved the buttons under the reader's hand from tab to tab.
            x: Math.max(0, opsBand.width - topBar.opsRightMargin - actionSeat.width)

            // Fetch, and everything the network has to say about fetching (`BandFetchButton`).
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
                // **The three ask for the same two numbers** — the box the set shares and the cap the window's width
                // settles — and neither moves with the shape a button is in, so giving the word up cannot change the
                // width that decided to. Folded, the box is the band's end cell over the panel's whole depth: the mark
                // stands in the same square the ☰ and the window's own three do (規約 §ウィンドウの縁).
                width: topBar.actionCap
                // **Two lines deep whatever the branch writes** (`OpsPicker.pairHeight`): a branch following nothing,
                // and a page still reading its repository, write one line there — and a button held to that stands its
                // own word over its frame and its mark under it.
                height: fetchButton.folded ? opsBand.height : branchPick.pairHeight
            }
            // Push, in whichever shape this branch's standing with its remote allows (`BandPushButton`).
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
            }
            // Everything uncommitted, set aside in one entry, on the press (`BandStashButton`).
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
            }
        }

        // ---- the find ----------------------------------------------------
        // At the panel's right end, ending it under the window's own ✕ the way the repository's name begins it under
        // the ☰ (規約 §ウィンドウの縁). Dressed like the three beside it — same frame, same ground inside it, same
        // mark-and-word — because it is a button in the same panel, and one button in a row of four wearing something
        // else reads as a different kind of control.
        ActionButton {
            id: findButton
            stacked: true
            anchors.right: parent.right
            anchors.rightMargin: topBar.bandRightMargin
            anchors.verticalCenter: parent.verticalCenter
            kind: "search"
            // **Plain, and a capital to start**, where the three beside it wear the chip. That dress belongs to a
            // spelling git would take, and no one git command is behind a search of the graph (規約
            // §git 用語のコード表記) — what this says is a word of the window's own vocabulary, so it wears that.
            text: qsTr("Search")
            // A step heavier than the chips beside it, so the four read as one weight: a UI face at this size puts
            // less ink down than the mono one a command is set in, and next to three of them the plain word thinned.
            wordWeight: Font.DemiBold
            // Measured for the same longest wording the three are, and laid out at the width that wording wants, so
            // the ink comes out at the same two ends theirs does (`ActionButton.slack`). **The natural width, not the
            // share** — the share is arithmetic on the room left over once this button's own run is paid for, and
            // reading it here would close that ring.
            widestText: topBar.widestAction
            widestCode: topBar.widestActionCode
            // **The first word the panel gives up** (§譲る順): folded, the button is the band's end cell with its
            // mark in it, the way the three beside it end up.
            foldRequested: topBar.findFolded
            wordFloor: topBar.actionWordFloor
            width: topBar.findCap
            height: findButton.folded ? opsBand.height : branchPick.pairHeight
            frameColor: Theme.borderStrong
            // The canvas inside the frame, like the three beside it (`BandFetchButton`).
            faceColor: Theme.bgSurface
            // A plan standing over the graph refuses this press wherever it comes from (`RepoPage.startFind`), so the
            // button says so rather than answering a hand with nothing.
            enabled: topBar.curPage !== null && !topBar.curPage.planActive
            // The freeze names itself, the way the stash button's does; with no tab open there is nothing on the band
            // to ask about (規約 §無効).
            tip: {
                if (topBar.curPage === null)
                    return ""
                if (topBar.curPage.planActive)
                    return qsTr("A rebase plan is being composed — the graph it stands over cannot be searched")
                return qsTr("Find commits")
            }
            Accessible.name: qsTr("Find commits")
            onActivated: topBar.curPage.startFind()
        }
    }
}
