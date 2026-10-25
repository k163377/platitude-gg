pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import platitude
import platitude.ui

// One sidebar row: ref / file / folder, shared by every section and the WIP file list. What a click means is the
// owner's business — the row only reports it.
Item {
    id: navRow
    required property int index
    required property string name
    required property string full
    required property string oid_hex
    required property string change
    required property string bucket
    required property string orig_path
    /// `orig_path` written the way this row writes names; `orig_path` stays whole because it addresses a diff
    /// (`models::nav`).
    required property string orig_name
    required property bool is_head
    required property bool has_remote
    required property bool only_remote
    required property bool has_pr
    required property int depth
    required property bool folder
    required property bool eol_mark
    /// The path the pane's line-ending card is out for; the card's own close clears it (`WipPane`).
    property string pointedEolPath: ""
    /// How far a local branch stands from its upstream as of the last fetch; 0 on every other row (`HeadTrack`).
    required property int ahead
    required property int behind
    /// The hand arrived at a row carrying the mark (`on`), or left it — the hand walking off, or the row's path
    /// changing under it (a recycled delegate). The path is the one that asked (`eolAskFor`); which list the row
    /// stands in is the list's to add (`WipBucketPane.section`) — the same path in the other list is another row,
    /// and the row's own `bucket` is the change's (an untracked file's is `untracked`), not the list's.
    signal eolPointed(string path, bool on)
    /// Stands in for the pointer where headless cannot put one, so a cut-down row's tooltip can be photographed
    /// (PGG_AUTO_ACT=path-tip). -1 points at no row.
    property int pointedTipRow: -1
    /// Whether the row is lit: the hand on it (`handOn`), or its own tip or menu standing — the hand walked into them
    /// (デザイン規約 §左メニューの所作).
    readonly property bool pointed: navRow.handOn || navRow.factsTipOut || navRow.factsMenuOut
    /// That menu was raised on the panel's open row (`SidebarRowGestures.menuOnOpenRow`) — held by the gestures, since
    /// the delegate showing that row may be another by the time it goes.
    property bool menuOnOpenRow: false
    property string kindHint: "branch"
    /// The remote pushes go to, empty where none is marked (`RepoTab.pushDefault`).
    property string markedRemote: ""
    /// The remote both origin keys name, empty where they part (`RepoTab.markedOrigin`).
    property string originRemote: ""
    /// The configured remote names, so a folder row can tell a remote from the shape of the names below it: a remote
    /// called `my/fork` puts a plain `my` folder above its own row.
    property var remoteNames: []
    /// Whether this row is a remote itself. Its fold key is the remote's whole name, which is what makes the test
    /// work on a name with a slash in it.
    readonly property bool isRemoteRow:
        navRow.kindHint === "remote" && navRow.folder && navRow.remoteNames.indexOf(navRow.fullName) >= 0
    /// Whether this row is the one pushes go to — what its badge says.
    readonly property bool pushesHere: navRow.isRemoteRow && navRow.fullName === navRow.markedRemote
    /// Whether this row is wholly origin — what its hover says. A remote only the push's key names wears the badge
    /// but not this: its menu offers to finish the mark, and a hover calling it origin would contradict it.
    readonly property bool holdsOrigin: navRow.isRemoteRow && navRow.fullName === navRow.originRemote
    property real listWidth: 200
    /// Where this row's seat begins and how far each fold steps it in — one answer for every list
    /// (`NavList.rowInset` / `nestStep`, デザイン規約 §余白), so a file's name stands where a branch's does. The
    /// sidebar hands its own down, which its stand-in row reads too (`HeadPinRow`).
    property int rowInset: Theme.spaceXs + Theme.iconSm - Theme.iconXs
    property int nestStep: Theme.iconSm / 2
    /// The layer an open name box is drawn in, and where this row's list sits in it (`NavList`), so the box follows
    /// its row through a scroll. No layer: the box stays in its seat.
    property Item boxLayer: null
    property real boxRowsX: 0
    property real boxRowsY: 0
    property real boxRowsTop: 0
    property real boxRowsHeight: 0
    // Shows the hover stage/unstage affordance (WIP view).
    property bool showStage: false
    /// Whether this row is one of those chosen (worktree list). Held by the list: delegates are recycled.
    property bool chosen: false
    /// The pointer is on another chosen row's stage affordance, and this row goes with it: what one press moves is
    /// seen before it is pressed (デザイン規約 §その他の操作).
    property bool stagePeer: false
    /// What each side of a conflict is called. **The two swap over during a rebase**, so they come from the model
    /// (`WorktreeModel.sideOurs` / `sideTheirs`); empty where git left nothing to name a side by.
    property string sideOurs: ""
    property string sideTheirs: ""

    /// What the two sides each did to this file (デザイン規約 §conflict の種別).
    function conflictWords() {
        return Words.conflict(navRow.change, navRow.sideOurs, navRow.sideTheirs)
    }

    // ---- the two-click gestures ------------------------------------
    // Which row was clicked last, and which is being typed into, are held by the sidebar: delegates are recycled
    // (デザイン規約 §左メニューの所作).
    property string rowKey: ""
    property string activeKey: ""
    property string editKey: ""
    /// "rename" (the name is in the box), "branch", "tag" or "worktree" (a name for a new one on this row's commit —
    /// the last a branch out in a worktree of its own).
    property string editMode: ""
    /// What a rename in that box renames — `branch` / `remote` / `tag` / `stash` / `worktreeFolder`, held by the sidebar
    /// (`SidebarRowGestures.editKind`); the row's own kind until something is typed into.
    property string editKind: navRow.kindHint
    /// What has been typed so far, held by the sidebar so a row that scrolls off and back keeps it.
    property string editText: ""
    property bool editRefused: false
    property string editRefusedWhy: ""
    /// A worktree's folder in that line, which the box's tip marks (`SidebarRowGestures.editRefusedMark`).
    property string editRefusedMark: ""
    /// …and the branch, tag or remote in it (`SidebarRowGestures.editRefusedMarks`).
    property var editRefusedMarks: []
    readonly property bool editing: navRow.editKey !== "" && navRow.editKey === navRow.rowKey
    /// The name git knows this row by.
    readonly property string fullName: navRow.full !== "" ? navRow.full : navRow.name
    /// What the walk over this list calls this row, empty on a folder — the name both file lists' rows answer to
    /// (`FileRowWalk`).
    readonly property string walkKey: navRow.folder ? "" : navRow.fullName
    /// That name while the row is painted as chosen, empty otherwise — what a headless run reads. The rectangle's own
    /// `visible`, since reading `chosen` back would go green with the rectangle unwired.
    readonly property string litKey: chosenBox.visible ? navRow.walkKey : ""
    /// The same rectangle as a bare answer, for counting lit rows (`FileRowWalk.litRows`; `FileRowDelegate` answers
    /// the same pair).
    readonly property bool litNow: chosenBox.visible

    signal refClicked(string oidHex)
    /// A worktree file row was clicked. `modifiers` carries Ctrl and Shift, which choose several rows.
    signal fileClicked(string bucket, string path, string origPath, int modifiers)
    signal folderClicked(string key)
    signal stageClicked(string bucket, string path)
    /// The pointer arrived at (or left) this row's stage affordance.
    signal stageHovered(string bucket, string path, bool on)
    /// A left click landed on this row, whatever it then meant.
    signal rowClicked()
    /// Double-click: go where this row leads.
    signal activateRequested()
    /// The box: typed into, accepted, walked away from.
    signal editTyped(string text)
    signal editAccepted(string text)
    signal editCancelled()
    /// Right-click on a ref row; the page owns the menu because delegates are recycled out from under an open popup.
    /// `name` is what the row shows, `full` what git knows it by (a stash shows a message and answers to a selector).
    /// `aim` is a remote the press named on its own — a TAGS row's carrier line (`NavRowFacts.lineMenu`) — else empty.
    signal refMenuRequested(string name, string full, string oidHex, string aim)
    /// Right-click on a remote's own row, the only folder row with anything behind it (デザイン規約 §左メニューの所作).
    /// Its own signal because a remote is configuration and opens a different menu.
    signal remoteMenuRequested(string name)
    /// Right-click on a worktree file row. The menu reads a rename's two names off the chosen rows (`orig_path`).
    signal fileMenuRequested(string bucket, string path)

    /// A row's height of ground at this row's foot for the stand-in of a current branch a fold closed over
    /// (`NavList.pinSeatRow` / `HeadPinRow.seatedUnder`); 0 on every other row. **Not part of the row**: nothing here
    /// lights it, and a hand on it is not on this row.
    property real pinSeat: 0

    width: listWidth
    // **The row grows by what it has open under it** and the rows below move down, rather than something landing on
    // top of them (デザイン規約 §左メニューの所作).
    height: navRow.rowHeight + navRow.pinSeat
    /// The row itself, seat aside — where its grounds stop and where the hand is still on it.
    readonly property real rowHeight: Theme.rowHeight + navRow.nameOverflow + factsSeat.height
    /// How far the name shown whole hangs below the line the row draws it on — 0 while the row is closed, and while
    /// the whole of it fits the line it was already on (`NameCell.wholeOver`).
    readonly property real nameOverflow: rowLayout.nameWholeOver

    // **The grounds in this list answer the hand alone** (click and pointer). The current branch is said by the name
    // (`NavRowBody`, デザイン規約 §ref の種別): a ground for it would wear the selection's colour, leaving a click on it
    // nothing to show.
    // The row a click last landed on. Without it the rename's second click has no target, and a click on a row whose
    // commit is already being read would look like it missed.
    Rectangle {
        id: chosenBox
        anchors.fill: parent
        anchors.bottomMargin: navRow.pinSeat
        color: Theme.bgSelected
        visible: !navRow.folder && (navRow.chosen || (navRow.rowKey !== "" && navRow.activeKey === navRow.rowKey))
    }
    Rectangle {
        id: washBox
        anchors.fill: parent
        anchors.bottomMargin: navRow.pinSeat
        color: Theme.bgHover
        // The stand-in lights it too, so a picture of a row with no tip still shows where the pointer was
        // (`tipPointedAt`). **An open row stays lit**: dark, its open lines would read as belonging to nothing.
        visible: navRow.pointed || navRow.tipPointedAt || navRow.factsOpen
    }
    // Nothing asks a question about a row in this list: what a row takes away is pressed through on the menu row that
    // names it (デザイン規約 §回数押し).
    // The row's ink (`NavRowBody`), handed the row itself: mirroring its properties here would double every binding
    // paid on reuse.
    NavRowBody {
        id: rowLayout
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        height: Theme.rowHeight
        anchors.leftMargin: navRow.rowInset + navRow.depth * navRow.nestStep
        // Stops at the gutter of the list's scroll bar, which is drawn over the row's right-aligned column
        // (デザイン規約 §QML 実装ルール の摘みの項).
        anchors.rightMargin: Theme.navBarGutter
        row: navRow
    }
    // The name box, drawn outside the list: it reparents itself (`NavNameBox.parent`), the loader only owns it.
    // **Built only while this row is typed into** (rules-refs/app-ui.md 「行のデリゲートが見せない部品は消す」).
    Loader {
        id: editSeat
        active: navRow.editing
        sourceComponent: NavNameBox {
            row: navRow
            seat: rowLayout.boxSeat
            drawnIn: navRow.boxLayer
            // Where the row's columns begin; the box adds the seat's place itself.
            rowsX: navRow.boxRowsX + rowLayout.x
            rowsY: navRow.boxRowsY
            rowsTop: navRow.boxRowsTop
            rowsHeight: navRow.boxRowsHeight
            editing: navRow.editing
            mode: navRow.editMode
            // What the box is naming, which is not always what the row is: `Create tag here…` opens on a branch row
            // too.
            // **Not the row's kind on a worktree row**: there `worktree` is the new worktree's box, and a rename names
            // the branch the row has out or its folder (`editKind`).
            namesKind: navRow.editMode === "tag" || navRow.editMode === "branch" || navRow.editMode === "worktree"
                       ? navRow.editMode : navRow.editKind
            carried: navRow.editText
            refused: navRow.editRefused
            refusedWhy: navRow.editRefusedWhy
            refusedMark: navRow.editRefusedMark
            refusedMarks: navRow.editRefusedMarks
            onTyped: text => navRow.editTyped(text)
            onSubmitted: text => navRow.editAccepted(text)
            onCancelled: navRow.editCancelled()
        }
    }
    // Rows whose name can be changed from here. A remote branch counts though git has no rename there: its box leads
    // to a replace (a push and a delete, asked first). A folder is only the shape of the names below it. A worktree
    // counts but the repository's own one on no branch, which git never moves (`worktreeRename`).
    readonly property bool nameable: !navRow.folder
        && (navRow.kindHint === "branch" || navRow.kindHint === "tag"
            || navRow.kindHint === "stash" || navRow.kindHint === "remote"
            || (navRow.kindHint === "worktree" && (navRow.bucket !== "" || navRow.worktreeRename !== "")))
    /// What the spaced second click does on a worktree row on no branch (`GitFacts.worktreeRename`): `box`, `locked`,
    /// `gone`, or empty for nothing. A worktree with a branch out renames the branch.
    readonly property string worktreeRename: navRow.kindHint === "worktree" && navRow.bucket === ""
        ? GitFacts.worktreeRename(navRow.change) : ""
    /// The spaced second click on this row opened no box, and why (`SidebarRowGestures.withholdRename`) — said where
    /// the box would have stood, as a refused name's reason is (`NavNameBox`).
    property string withheldKey: ""
    property string withheldWhy: ""
    property string withheldMark: ""
    readonly property bool renameWithheld: navRow.withheldKey !== "" && navRow.withheldKey === navRow.rowKey
    // Built only while it says so (rules-refs/app-ui.md「行のデリゲートが見せない部品は消す」), on the name — the box's
    // own seat is drawn only while a box stands in it (`NavRowBody.boxSeat`).
    Loader {
        id: withheldSeat
        active: navRow.renameWithheld
        sourceComponent: Item {
            id: withheldBox
            parent: rowLayout.nameInk
            anchors.fill: parent
            // Answers the press that asked, not a hand (`TipAsk.typed`); the folder the line names behind the tree
            // mark (デザイン規約 §ref の種別「名前の印」).
            readonly property TipAsk withheldTip: TipAsk {
                target: withheldBox
                asked: withheldBox.visible && navRow.withheldWhy !== ""
                typed: true
                words: navRow.withheldWhy
                markWord: navRow.withheldMark
            }
        }
    }
    /// The gesture the rows of this section share, or null for a list whose rows cannot be typed into (the working
    /// tree's files). Held by the sidebar, since it has to outlive this delegate (`SidebarRowGestures`).
    property ReclickGesture reclick: null
    /// The panel's hand (`HandWatch`), which this row reads its light and its rest off: Qt hands hover to whatever
    /// arrives under a still pointer, and a row the list moved under one was not walked to. Both lists hand theirs in
    /// (the left panel's, the worktree's file lists'). A read tells the lists only the rows it moved, so the row under
    /// the hand keeps its delegate, and the pane puts it back under the hand (`RowKeep`). Null — a row standing on its
    /// own — reads Qt's hover (`rowHover`).
    property HandWatch paneHand: null
    /// Every move of the hand is one read per row, and so is the reader moving the rows under it (`HandWatch.shifts`):
    /// the light goes to the row under the hand, as Qt's own lists hand their hover on with a scroll
    /// (`QAbstractItemView::verticalScrollbarValueChanged`), and what that row asks for waits on the seat until the
    /// hand moves (デザイン規約 §左メニューの所作「手が動いていない所へ来た行は開かない」「手が動いたかの見切り」).
    /// **Never the layout moving on its own**: these rows grow in place, and the row the hand walked to keeps its
    /// light and its rest, as Qt's lists keep their hover on the item when rows open above it (§「点くのも同じ規則」).
    readonly property int handMoves: navRow.paneHand !== null ? navRow.paneHand.moves : 0
    onHandMovesChanged: navRow.syncHand(false)
    readonly property int handShifts: navRow.paneHand !== null ? navRow.paneHand.shifts : 0
    onHandShiftsChanged: navRow.syncHand(true)
    /// The hand itself on this row's line or its open lines, which are inside it. **Unlike `pointed`, a standing tip
    /// does not count**: the light stays while the tip does, but what asks for the tip must not, or the tip would
    /// hold itself up for good. Weighed by geometry against the hand's place, not by Qt's hover: Qt hands hover out
    /// before the view lays its rows again, and a row it has just built at a stale place over this one takes the
    /// hover from it.
    property bool handOn: false
    /// This row slid out from under a still hand (the reader's scroll): what it had out goes at once, with no beat
    /// (§hover のツールチップ「的が手の下で動けば」). A standing tip or menu of its own holds a row otherwise — the hand
    /// walked into it — but not one that slid away: no hand went there. **Not a row a read moved**
    /// (`HandWatch.disturbed`): it leaves the hand as a hand leaves it, and the seats weigh what stays
    /// (§「外乱では閉じない」, `OpenRowSeat`).
    property bool slid: false
    function syncHand(shifted) {
        if (navRow.paneHand === null) {
            // A row the view has pooled is hidden with whatever hover it had (`QQuickItemView` hides it, Qt sends it
            // no leave): not on screen is not under the hand.
            navRow.handOn = navRow.visible && (rowHover.hovered || navRow.factsPointed)
            return
        }
        const hand = navRow.paneHand
        let on = hand.known && navRow.visible && !navRow.pooled
        if (on) {
            const p = navRow.mapFromItem(null, hand.at.x, hand.at.y)
            on = p.x >= 0 && p.y >= 0 && p.x < navRow.width && p.y < navRow.rowHeight
        }
        // Inside the list's own box: a row kept built past its edge (`cacheBuffer`) lies under the band and the next
        // section, drawn not at all. And not under the stand-in on its edge (`NavList.shieldAt`) — the worktree's file
        // lists are plain `AppListView`s, with none.
        const view = navRow.ListView.view
        if (on && view) {
            const v = view.mapFromItem(null, hand.at.x, hand.at.y)
            on = v.x >= 0 && v.y >= 0 && v.x < view.width && v.y < view.height
                 && !(view.shieldAt !== undefined && view.shieldAt(hand.at))
        }
        if (shifted) {
            navRow.slid = navRow.handOn && !on && !hand.disturbed
            navRow.handOn = on
            return
        }
        // A move of the hand, however small: the light goes to the row under it, the slid row's goes, and the row
        // under it asks — the seat's rest judges whether it opens (デザイン規約 §左メニューの所作「手が動いたかの見切り」).
        navRow.slid = false
        navRow.handOn = on
    }
    /// The hand itself on the row it walked onto, still under it — what asks for the row's lines, its tip and its
    /// card. **The light is wider** (`pointed`): a standing tip or menu of the row's own keeps the light, but is the
    /// seat's hold, never an ask (`SidebarRowGestures.openSeat`).
    readonly property bool stillPointed: !navRow.slid && navRow.handOn
    /// A delegate the view pools is off screen, and one it uses again is another row (`ListView.reuseItems`): what
    /// the hand found on it does not carry over — its light would land on a row nobody walked to, its rest open it.
    function forgetHand() {
        navRow.handOn = false
        navRow.slid = false
    }
    /// Pooled, the delegate stays `visible` where its row last stood (the view only stops drawing it): it reads no
    /// hand until it is used again. Its tips go with it — the row they named left the list or the screen, and the
    /// delegate is to stand for another — but for the box the hand is reading, which stays where it stands until the
    /// hand leaves it or a read's keeper finds the row went (`Tips.dropIn`).
    property bool pooled: false
    ListView.onPooled: {
        navRow.pooled = true
        navRow.forgetHand()
        Tips.dropIn(navRow)
    }
    ListView.onReused: {
        navRow.pooled = false
        navRow.forgetHand()
    }
    /// The hand is on this row's open lines (`NavRowFacts.pointed`). **Read as the row's own hover**: the lines are a
    /// hover-taking sibling stacked over `rowGround` and take the pointer off its handler (rules-refs/app-ui.md
    /// 「開いた行は段の hover を自分のものとして読む」).
    readonly property bool factsPointed: factsSeat.item ? factsSeat.item.pointed : false
    onFactsPointedChanged: if (navRow.paneHand === null) navRow.syncHand()
    /// Whether the tip these lines put out is standing (`NavRowFacts.tipShown`). **A row whose tip is up is still the
    /// row being read**: the tip is a popup that takes the pointer, and closing the row then would take the tip's
    /// target away (デザイン規約 §hover のツールチップ).
    readonly property bool factsTipOut: factsSeat.item ? factsSeat.item.tipShown : false
    /// Whether the menu standing was raised on this row while open (`rowPressed`). **The row is still the one being
    /// read**, as with its tip: the menu takes the pointer off the row, and the row reading that as the hand gone would
    /// shrink from under the menu about it (デザイン規約 §左メニューの所作). Once the menu goes, the hand decides again.
    readonly property bool factsMenuOut: navRow.factsOpen && navRow.menuOnOpenRow
    /// Whether this row holds the wait the name box opens after, and whether a click now would still count as a
    /// double-click's second half — read by headless runs. Both are the sidebar's gesture's answer
    /// (`SidebarRowGestures`).
    readonly property bool renameArmed: navRow.reclick ? navRow.reclick.armedFor(navRow.rowKey) : false
    readonly property bool clickGuarded: navRow.reclick ? navRow.reclick.guarded : false
    /// Whether the box on this row has the keyboard — the output side: a box drawn where nothing can be typed still
    /// reads as a box, and the folded list's section is a popup that takes the keyboard only when something asks.
    readonly property bool editFocused: editSeat.item ? editSeat.item.activeFocus : false
    /// What the box came out as, for the runs that photograph it (`NavNameBox`): as drawn, as what is in it wants,
    /// whether it is on screen at all, and where it landed.
    readonly property real editBoxWidth: editSeat.item ? editSeat.item.width : 0
    readonly property real editBoxWhole: editSeat.item ? editSeat.item.wantWidth : 0
    readonly property real editBoxSeat: rowLayout.boxSeat.width
    readonly property bool editBoxShown: editSeat.item ? editSeat.item.visible : false
    readonly property string editBoxAt: editSeat.item ? editSeat.item.cameOut : ""
    /// Whether the window's tooltip stands **on this row's box** (a refused name's reason, `NavNameBox`). Read off the
    /// box's own ask, which the seat weighs against the target; the box's own `visible` would answer for anybody's
    /// tip.
    readonly property bool editTipShown: editSeat.item ? editSeat.item.tipShown : false
    /// What the box says it takes, rubbed out (PGG_AUTO_ACT=worktree-rename-box): a picture of a box holding a name
    /// does not show it.
    readonly property string editPlaceholder: editSeat.item ? editSeat.item.placeholderText : ""
    /// A left click, as this row answers one. Named so that a run with no pointer to press with puts its click in at
    /// the row itself (PGG_AUTO_ACT=nav-reclick).
    function leftClick(modifiers) {
        // A double-click's second click belongs to the double: the first already did what a click does.
        // What a second click would name is read now: by the time the wait runs out this delegate may be showing
        // another row (`ReclickGesture`).
        if (navRow.reclick
                && !navRow.reclick.click(navRow.rowKey,
                                         navRow.nameable ? navRow.renameNames() : null))
            return
        navRow.rowClicked()
        navRow.ordinaryClick(modifiers)
    }
    /// What the box a second click opens is named after: the row's kind, what git knows it by, the commit it is on,
    /// and the name it shows (a stash is typed by its message, not by its selector). A worktree row renames the branch
    /// it has out, else its folder (known to git by its path) — or opens no box and says why (`withheld`).
    function renameNames() {
        if (navRow.kindHint === "worktree" && navRow.bucket !== "")
            return { "kind": "branch", "id": navRow.bucket, "oid": navRow.oid_hex, "name": navRow.bucket }
        if (navRow.kindHint === "worktree" && navRow.worktreeRename !== "box")
            return { "withheld": Words.worktreeRenameWithheld(navRow.worktreeRename, navRow.name), "mark": navRow.name }
        return {
            "kind": navRow.kindHint === "worktree" ? "worktreeFolder" : navRow.kindHint,
            "id": navRow.full !== "" ? navRow.full : navRow.name,
            "oid": navRow.oid_hex,
            "name": navRow.name
        }
    }
    function ordinaryClick(modifiers) {
        if (navRow.folder) {
            navRow.folderClicked(navRow.full)
        } else if (navRow.kindHint === "file") {
            navRow.fileClicked(navRow.bucket, navRow.fullName, navRow.orig_path,
                               modifiers === undefined ? Qt.NoModifier : modifiers)
        } else if (navRow.oid_hex !== "") {
            // Every row naming a commit goes there, a worktree row to the commit its checkout stands on
            // (デザイン規約 §左メニューの所作). A bare entry names none and answers a click with nothing.
            navRow.refClicked(navRow.oid_hex)
        }
    }
    // Which row the hand is on, for a row no panel's hand covers (one standing on its own) — a handler, since a
    // `MouseArea` would lose `containsMouse` to the stage `+` the moment the hand reached it (rules-refs/app-ui.md
    // 「行の hover は `HoverHandler`」). **Where a panel's hand is read, nothing is written from here**: hover follows
    // the item, not the hand, so rows growing or a list scrolling under a still pointer would light rows the reader
    // never walked to (`tests/qml/tst_hoverunderstillhand.qml`).
    // **On `rowGround`, not the whole item**: a `HoverHandler` takes its item's full face, and the seat at this row's
    // foot (`pinSeat`) is not this row (rules-refs/app-ui.md の「親アイテムの全面」の行).
    Item {
        id: rowGround
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: navRow.rowHeight
        HoverHandler {
            id: rowHover
            enabled: !navRow.editing
            onHoveredChanged: if (navRow.paneHand === null) navRow.syncHand()
        }
    }
    /// The path the line-ending card is asked for: this row's, while the hand is on it and it wears the mark; "" for
    /// none. Every way the ask ends sends the leave for the path that asked — the hand walking off, the path changing
    /// under the hand (a recycled delegate) — so the pane's seat is never left waiting on a row that let go
    /// (`WipPane.pointEol`: the rest before the card, its beat, a hand back from the card answered at once). A mark
    /// that goes comes with a status read: the pane weighs the card's target then (`WipPane.weighEol`).
    readonly property string eolAskFor: navRow.eol_mark && navRow.stillPointed ? navRow.fullName : ""
    property string eolAskedPath: ""
    onEolAskForChanged: {
        if (navRow.eolAskedPath !== "")
            navRow.eolPointed(navRow.eolAskedPath, false)
        navRow.eolAskedPath = navRow.eolAskFor
        if (navRow.eolAskedPath !== "")
            navRow.eolPointed(navRow.eolAskedPath, true)
    }
    // The row's facts are asked for as the hand arrives and let go as it leaves: the panel's seat counts the rest and
    // the beat, and holds them while the row's own tip or menu stands (`SidebarRowGestures.openSeat`). They are
    // **inside** the row, so the hand walking down into them has not left it (`syncHand`).
    onStillPointedChanged: {
        if (navRow.stillPointed)
            navRow.askFacts(navRow.paneHand !== null ? navRow.paneHand.at : rowHover.point.scenePosition)
        else if (navRow.expands)
            // A row that slid out from under the hand lets go at once, with no beat (規約 §hover のツールチップ「的が
            // 手の下で動けば hover は外れ、出したものはその瞬間に消える」).
            navRow.factsAsked(false, Qt.point(0, 0), navRow.slid)
    }
    MouseArea {
        id: itemMouse
        anchors.fill: parent
        anchors.bottomMargin: navRow.pinSeat
        // While the box is open the row belongs to it.
        visible: !navRow.editing
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        // **The row keeps the drag it is handed** (`GraphRowDelegate`, `tst_pressorder`): the list steals at the same
        // distance this row hands the press to the sweep at, and losing that race cancels the press.
        preventStealing: true
        /// Where it went down, and whether this press was handed on to the name. **Each handler is one line into the
        /// row's own function**, which is what a run enters (verify-ui implement.md「注入はハンドラ本体そのものへ入れる」).
        property point pressFrom: Qt.point(0, 0)
        property bool pressing: false
        property bool handedOn: false
        onPressed: mouse => navRow.linePressed(mouse.button, mouse.x, mouse.y)
        onPositionChanged: mouse => navRow.lineDragged(mouse.x, mouse.y)
        onReleased: navRow.lineReleased()
        onCanceled: navRow.lineReleased()
        onClicked: mouse => navRow.lineClicked(mouse.button, mouse.modifiers)
        onDoubleClicked: mouse => navRow.rowDoubled(mouse.button)
    }
    /// The name as the open row shows it, and what a drag over it came away with — null and empty while the row
    /// is closed (`NameCell.whole`).
    readonly property Item nameField: rowLayout.nameWhole
    readonly property string nameTook: navRow.nameField ? navRow.nameField.selected : ""
    readonly property bool nameCaret: !!navRow.nameField && navRow.nameField.hasCaret
    /// The name the row draws — the main worktree's row draws its branch where it has one (`NavRowBody.shownName`).
    readonly property string shownName: rowLayout.shownName

    /// A press on the row's **own line** (the open lines answer theirs in `NavRowFacts`). One that never moves is a
    /// click; one that moves is going for the name, so the row opens **now** and the drag carries on into the name
    /// (デザイン規約 §左メニューの所作「開いた字は掴める」).
    function linePressed(button, x, y) {
        if (button !== Qt.LeftButton)
            return
        navRow.takePress()
        itemMouse.pressFrom = Qt.point(x, y)
        itemMouse.pressing = true
        itemMouse.handedOn = false
        // One selection in the window: a press here drops the last one, so a click that takes no words is read as a
        // click and not as the sweep before it.
        if (navRow.factsItem !== null)
            navRow.factsItem.dropSweep()
        if (navRow.nameField !== null)
            navRow.nameField.deselect()
    }
    /// A left press on this row and its end, its open lines' included (`NavRowFacts`). The row answers at the click —
    /// the release — so the gesture is told at the press that the press is the row's (`ReclickGesture.take`), and at
    /// the release that it ended (`hear`): unless the click follows in that release, the press was no click — a sweep
    /// of the name, one let go off the row or taken away — and it came between two clicks all the same.
    function takePress() {
        if (navRow.reclick)
            navRow.reclick.take()
    }
    function endPress() {
        if (navRow.reclick)
            navRow.reclick.hear()
    }
    function lineDragged(x, y) {
        if (!navRow.expands || !itemMouse.pressing)
            return
        // A click with a tremor in it is still a click.
        if (!itemMouse.handedOn
                && Math.abs(x - itemMouse.pressFrom.x) < Qt.styleHints.startDragDistance
                && Math.abs(y - itemMouse.pressFrom.y) < Qt.styleHints.startDragDistance)
            return
        if (!itemMouse.handedOn) {
            // The rest is what this press is skipping — the reader has already chosen this row.
            if (!navRow.factsOpen)
                navRow.askFacts(navRow.mapToItem(null, x, y), true)
            // Refused (a box is open, a menu is standing): there is no field to sweep, and the press stays a press.
            if (navRow.nameField === null)
                return
            // Anchored where the button went down; a press beside a short name clamps to its end (`CardText.inBox`).
            navRow.nameField.anchorFrom(navRow, itemMouse.pressFrom)
            itemMouse.handedOn = true
        }
        navRow.nameField.extendFrom(navRow, Qt.point(x, y))
    }
    function lineReleased() {
        itemMouse.pressing = false
        navRow.endPress()
    }
    /// **A drag that took some of the name is not a click**: the reader was copying, and a second click would open a
    /// name box (`NavRowFacts.handClicked` tests the same).
    function lineClicked(button, modifiers) {
        if (button === Qt.LeftButton && navRow.nameTook !== "")
            return
        navRow.rowPressed(button, modifiers)
    }
    /// A press on this row, and a double-click on it — out of the handler because the open lines answer with these
    /// two as well (`NavRowFacts`): a press there that never moved is this row's own click. `aim` is the remote a
    /// line of those names on its own, the menu's to act on (`refMenuRequested`).
    function rowPressed(button, modifiers, aim) {
        if (button === Qt.RightButton) {
            // Only rows with operations behind them open a menu. A worktree's is the ref menu too: the branch it has
            // out, or its commit, and its own card (デザイン規約 §左メニューの所作).
            if (!navRow.folder && navRow.oid_hex !== ""
                    && (navRow.kindHint === "branch"
                        || navRow.kindHint === "remote"
                        || navRow.kindHint === "tag"
                        || navRow.kindHint === "stash"
                        || navRow.kindHint === "worktree")) {
                // Raised on the open row — its own line or its lines — the menu is about that row and keeps it open
                // (デザイン規約 §左メニューの所作): the page keys the menu by the row, as the panel keys the row
                // (`SidebarRowGestures.menuOnOpenRow`).
                navRow.refMenuRequested(navRow.name, navRow.fullName, navRow.oid_hex, aim === undefined ? "" : aim)
            } else if (!navRow.folder && navRow.kindHint === "file")
                navRow.fileMenuRequested(navRow.bucket, navRow.fullName)
            else if (navRow.isRemoteRow)
                navRow.remoteMenuRequested(navRow.fullName)
            return
        }
        navRow.leftClick(modifiers)
    }
    function rowDoubled(button) {
        if (button !== Qt.LeftButton || navRow.folder)
            return
        if (navRow.reclick)
            navRow.reclick.drop()
        navRow.activateRequested()
    }
    HoverToolButton {
        id: stageButton
        visible: navRow.showStage && !navRow.folder && (navRow.pointed || navRow.stagePeer)
        anchors.right: parent.right
        anchors.rightMargin: Theme.spaceXs
        // The row's own line, not a row grown by open facts: the mark sits alike in both lists.
        anchors.verticalCenter: rowLayout.verticalCenter
        // The mark a step down in the `iconLg` seat, as the sidebar band's `+` (デザイン規約 §アイコンの席「④ の席」).
        padding: (Theme.iconLg - Theme.iconMd) / 2
        implicitWidth: Theme.iconLg
        implicitHeight: Theme.iconLg
        // Told to the list, so the rows that go with this one put their marks out too.
        onHoveredChanged: navRow.stageHovered(
            navRow.bucket, navRow.full !== "" ? navRow.full : navRow.name,
            stageButton.hovered)
        // On a conflicted row `git add` tells git the conflict is dealt with, and the word says so
        // (デザイン規約 §diff の中のステージ).
        tip: navRow.bucket === "conflicts" ? qsTr("Mark resolved")
             : navRow.bucket === "staged" ? qsTr("Unstage file")
                                          : qsTr("Stage file")
        onClicked: navRow.stageClicked(navRow.bucket, navRow.full !== "" ? navRow.full : navRow.name)
        contentItem: NavIcon {
            kind: navRow.bucket === "staged" ? "minus" : "plus"
            tint: navRow.bucket === "staged" ? Theme.diffRemovedFg : Theme.diffAddedFg
        }
    }
    /// Whether this row's list opens a row's facts under it, and which row is open there — handed down because
    /// delegates are recycled (`SidebarRowGestures.openKey`).
    property bool opensFacts: false
    property string openKey: ""
    /// This row's facts are asked for, or let go of, at the place the pointer was standing. `now` skips the wait
    /// either way: a press going for the words opens with no rest, a row that slid out from under the hand closes
    /// with no beat. The pane holds which row is open.
    signal factsAsked(bool open, point at, bool now)
    /// A line of those facts going somewhere of its own was pressed: the graph goes to the commit it names
    /// (`NavFacts.place`), and this row — the one the press was in — is the one last clicked
    /// (`SidebarRowGestures.followLine`).
    signal factsFollowed(string key, string oidHex)
    function followFact(to) {
        navRow.factsFollowed(navRow.rowKey, to.oid)
    }
    /// Where this row's answers come off: its own section's model (`upstreamOf`) and the worktrees' section
    /// (`worktreeHolding`), the only one holding the worktrees.
    property var sectionModel: null
    property var worktreesModel: null
    /// The branches' own section, which a worktree's row asks about the branch it holds (`upstreamOf` /
    /// `upstreamGoneOf`) — what that branch's own row would say of itself. Null in every other list.
    property var branchesModel: null
    /// TAGS only: the remote this window's tag rows act on, the reading a tag's open lines are read against
    /// (`NavList.pushRemote`).
    property string pushRemote: ""
    /// Whether this row answers a rest by opening. **Leaves only**: a fold parent is the shape of the names below it,
    /// has no line to open, and says its path on hover instead (`hoverText`). **REMOTES leaves out the remote's own
    /// row** too: what it says is its role (origin), a sentence and not a name. Every row of WORKTREES opens (it
    /// shows a folder name; the whole of it is its path).
    /// **A tag's row folds the remotes carrying its name** — a tag has no namespace, so each carrier takes a line
    /// (`NavFacts.carrierLine`). A row with nothing else to say still opens: the name in full is reason enough.
    readonly property bool expands:
        navRow.opensFacts
        && !navRow.folder
        && (navRow.kindHint === "branch"
            || navRow.kindHint === "worktree"
            || navRow.kindHint === "tag"
            || (navRow.kindHint === "remote" && !navRow.isRemoteRow))
    /// Whether this row is the open one. The marks it spells out leave the row's own line while it is, so they are
    /// seen to move down rather than stand twice. **The key carries the section** (`NavList.keyOf`): a name a remote
    /// carries and a branch here carries are two refs (デザイン規約 §ref の種別), and a bare name would open both rows.
    readonly property bool factsOpen: navRow.expands && navRow.fullName !== "" && navRow.openKey === navRow.rowKey
    /// Whether the row is painted as the one under the hand — the rectangle's own answer, so a run cannot go green
    /// with the wash unwired.
    readonly property bool washLit: washBox.visible
    /// The lines this row has open under it, where a run drives the sweeping hand (`NavRowFacts`); null while closed.
    /// `lineHeight` is the row's own line — what a run reads to see the lines were seated under it, not over it.
    readonly property Item factsItem: factsSeat.item
    readonly property real lineHeight: rowLayout.height

    // Hover says the name in full — what a folded or elided row cannot show (デザイン規約 §hover のツールチップ). A
    // stash's name is its message; nobody hovers to learn `stash@{0}`.
    readonly property string hoverText: {
        const full = navRow.fullName
        // A row that opens says it all under itself, name included (one pointer opens one thing). Its fold parents
        // fall through to the folder rule: nothing opens under a folder.
        if (navRow.expands)
            return ""
        // The remote row that is origin says its role first, then the name (デザイン規約 §hover のツールチップ: 結論から
        // 1 行), in the words its mark uses (§リモートを書き留める).
        if (navRow.holdsOrigin)
            return qsTr("origin (default remote) — %1").arg(full)
        // Every folder row says its own path: in the worktree's list it rides in `orig_path`, and a ref folder's
        // fold key is its path.
        if (navRow.folder)
            return navRow.kindHint === "file" ? navRow.orig_path : full
        if (navRow.kindHint === "stash")
            return navRow.name
        if (navRow.kindHint === "branch" || navRow.kindHint === "tag" || navRow.kindHint === "remote")
            return full
        // A marked row's card names the path as its first line (規約 §hover のツールチップ「1 つのポインタが開けるものは
        // 1 つ」). **The mark is the test**, not "is my card up": both open after the same rest, and the tip would
        // come up a turn before the card.
        if (navRow.eol_mark)
            return ""
        return full
    }
    /// Whether the card standing is this row's: the pane keeps the one answer and the row weighs it against its own
    /// name (`NavSectionModel::point_eol`).
    readonly property bool eolCardOut: navRow.eol_mark && navRow.pointedEolPath === navRow.fullName
    /// Whether the headless stand-in points at this row (the report reads the window's box itself — the output side).
    /// **`>= 0` first**: a pooled delegate reports `index` -1, which is also "no row", and every pooled row would claim
    /// the window's one tooltip (rules-refs/app-ui.md 「プールへ戻された delegate は `index` -1 を名乗る」).
    readonly property bool tipPointedAt: navRow.pointedTipRow >= 0 && navRow.pointedTipRow === navRow.index
    /// The row's own tip is up (the output side, for the tests).
    readonly property bool tipShown: rowTip.shown
    readonly property TipAsk rowTip: TipAsk {
        id: rowTip
        target: navRow
        // Behind a standing menu the window's seat holds the box down (`TipSeat`'s machine, `HoverSeat.blockedBy`,
        // デザイン規約 §メニュー), not this ask: an ask that fell for the menu reads as the hand leaving the row, and the
        // box would come back under the still hand the moment the menu went.
        asked: (navRow.stillPointed || navRow.tipPointedAt) && !navRow.editing && navRow.hoverText !== ""
        words: navRow.hoverText
        // Out of the list, never over the rows either side: the left panel has no room on its left, so its box goes
        // right, and the right panel's to its left.
        rowSide: "left"
    }

    /// What the row opens under itself, read as it opens rather than bound: the answers come off slots, which a
    /// binding freezes (app-ui.md).
    property string factsName: ""
    property string factsLocal: ""
    property string factsHeldBy: ""
    property string factsUpstream: ""
    property bool factsGone: false
    property string factsBranch: ""
    property int factsAhead: 0
    property int factsBehind: 0
    property string factsPath: ""
    /// TAGS only, comma-separated for a report line (`NavList.openWords`): the remotes carrying this name, those that
    /// have it elsewhere than the reading they are read against, and which reading that is. **The last two are what
    /// no picture settles** (`NavSectionModel.tagRemotes`).
    property string factsRemotes: ""
    property string factsApart: ""
    property string factsAgainst: ""
    /// Who the right reading belongs to, comma-separated as the others (`NavFacts.answers`' `by`).
    property string factsBy: ""
    /// TAGS only: the copy here stands apart from that reading too — the row's own name wears the warning while open
    /// (`NavRowBody`), and `factsNameNote` is what a rest on it says (`NavFacts.answers`).
    property bool factsHereApart: false
    property string factsNameNote: ""
    /// Those answers as the lines to draw, in reading order (`NavFacts.lines`); the named fields above are what a run
    /// reads back (`NavList.openWords`).
    property var factsLines: []
    property string factsState: ""
    property string factsWhy: ""

    /// Read them, and answer whether there is anything to open. **The name alone is enough** (it is what a reader can
    /// drag away); a row the model has not filled yet opens nothing.
    function gatherFacts() {
        // **One table answers for every section** (`NavFacts`), so nothing here knows which section this row is in.
        const said = NavFacts.answers(navRow)
        navRow.factsLocal = said.local
        navRow.factsUpstream = said.upstream
        navRow.factsGone = said.gone
        navRow.factsHeldBy = said.heldBy
        navRow.factsState = said.state
        navRow.factsWhy = said.why
        navRow.factsBranch = said.branch
        navRow.factsAhead = said.ahead
        navRow.factsBehind = said.behind
        navRow.factsPath = said.path
        navRow.factsRemotes = said.remotes.map(carried => carried.remote).join(",")
        navRow.factsApart = said.remotes.filter(carried => carried.apart)
                                        .map(carried => carried.remote).join(",")
        navRow.factsAgainst = said.against
        navRow.factsBy = said.by.split(", ").join(",")
        navRow.factsHereApart = said.hereApart
        navRow.factsNameNote = said.nameNote
        navRow.factsName = said.name
        navRow.factsLines = NavFacts.lines(navRow.kindHint, said)
        return navRow.factsName !== ""
    }
    /// Ask for this row to be the open one, with the pointer where it was standing when it asked — `now` for a press
    /// going for the words, which skips the rest. Whether it may — a name box, a menu, a held bar, a hand that has
    /// not walked — is the seat's to weigh.
    function askFacts(at, now) {
        if (navRow.expands && navRow.gatherFacts())
            navRow.factsAsked(true, at, now === true)
    }
    // The stand-in asks for the facts from the row's own middle, so two rows are never asked for from one place; the
    // seat counts the rest for it as for a hand.
    onTipPointedAtChanged: {
        if (navRow.tipPointedAt)
            navRow.askFacts(navRow.mapToItem(null, navRow.width / 2, Theme.rowHeight / 2))
        else if (navRow.expands)
            navRow.factsAsked(false, Qt.point(0, 0), false)
    }
    // A row that comes back open — recycled onto the open name while it was scrolled off — reads its answers again.
    onFactsOpenChanged: if (navRow.factsOpen) navRow.gatherFacts()
    // The facts, under the row's own line and inside it (`NavRowFacts`). **Built only while the row is open**
    // (rules-refs/app-ui.md 「行のデリゲートが見せない部品は消す」).
    Loader {
        id: factsSeat
        active: navRow.factsOpen
        visible: factsSeat.active
        // **Measured off the row's height, not the layout's foot**: a `RowLayout` is only as tall as its content, and
        // lines seated there cover the name. A wrapped name pushes them below its last line.
        anchors.top: parent.top
        anchors.topMargin: Theme.rowHeight + navRow.nameOverflow
        // **Wider than the row's columns either side**, given back inside (`NavRowFacts.bandReach`): a line's band
        // reaches that far and has to be inside the item that is hovered and pressed.
        anchors.left: parent.left
        anchors.leftMargin: navRow.rowInset + navRow.depth * navRow.nestStep - Theme.spaceXs
        anchors.right: parent.right
        anchors.rightMargin: Theme.navBarGutter - Theme.spaceXs
        height: factsSeat.item ? factsSeat.item.implicitHeight : 0
        sourceComponent: NavRowFacts {
            row: navRow
            // Already in reading order (`NavFacts.lines`); the lines know nothing of which section they are in.
            lines: navRow.factsLines
            // The one answer that is not a line: where a worktree stands, said on a rest anywhere in the open row.
            path: navRow.factsPath
            // **The rest on the row's own line asks for it too** — walking down to the lines would pay a second rest
            // for it. Both roads write the same ask, so the box stays up as the hand crosses into the lines. Not once
            // the row slides out from under the hand (`slid`).
            rowPointed: (navRow.handOn && !navRow.slid) || navRow.tipPointedAt
            // Why the row's own name wears the warning, said on a rest on that name alone: the lines under it keep
            // notes of their own. `rowHover` hears only the row's own line — the lines take the pointer off it.
            nameNote: navRow.factsNameNote
            nameRested: rowHover.hovered
        }
    }
}
