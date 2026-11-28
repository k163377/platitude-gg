pragma ComponentBehavior: Bound

import QtQuick
import platitude.ui
import platitude.ui.words

// The right-click on a graph row: one menu for a commit, another for a stash, a third for another worktree's
// uncommitted row (デザイン規約 §グラフ行の右クリック), a fourth inside a choice of two commits — this worktree's own
// uncommitted row has none (user decision — ask before changing). The first level
// holds the rows used often — on the row's commit, and on the name the chip draws (or the name pressed in the chip's
// stacked list): its pull, push and delete table —; the cards under them hold what is used rarely.
//
// The discard log's entry on the graph is a display mode of its own (user decision — ask before changing), and only a
// commit it draws has a menu: this one, aimed at no name and with only what adds (`menu::Seen::Graph`'s
// `provisional`) — `Create branch here…`, `cherry-pick`, the WORKTREE and TAG cards' making rows, GITHUB. What the
// mode can never do has no row; what stays greys as anywhere. A row it leaves dim and a tip it draws as a stash or as
// uncommitted work have none (`GraphModel.provisionalMenuShutAt`, `RepoPage.openRowMenu`).
//
// Which rows a menu has and why each greys is the menu type's (`menu::lines`), handed in whole (`lines`); what a press
// sends stays the row's own. Nothing here is looked up and nothing is run: answers arrive as plain values, set as the
// menu opens and again as it stands again on the tree as it is, its rows kept (`CommitMenuState`, デザイン規約
// §メニュー), and presses leave as signals for `RepoPage` — which is what lets `tst_commitrowmenu.qml` drive the real
// menu.
//
// An `Item` because a menu measures the window through the item it was declared under (`AppMenu.ownerItem`).
Item {
    id: rowMenu

    /// Why every row is out while a history-replaying write holds the doors (`RepoPage.doorsHeldWhy`). Every card
    /// takes it; the rows stay and grey (`AppMenu.heldReason`).
    property string heldReason: ""
    /// What the rows say while they wait for a git command this window started (`RepoPage.runningWhy`). Every card
    /// takes it; a card opened under one keeps its rows and greys them until it ends (`AppMenu.runningWhy`).
    property string runningWhy: ""

    /// The name this menu is aimed at: the one the chip draws, or the one pressed in the chip's stacked list. `kind`
    /// is `branch` / `remote` / `tag`, or `worktree` for the chip of a worktree with no branch (named by its folder) —
    /// its name then the worktree's path — and empty on a row that draws neither (デザイン規約 §グラフ行の右クリック).
    property string targetKind: ""
    property string targetName: ""
    /// What merge / rebase are handed: the name where the row draws one — a merge by name says so in the commit it
    /// writes, and one by id writes `Merge commit '<hash>'` after `main` was pressed. A worktree's path is no ref, so
    /// the commit. Whether they can be pressed is the commit's place, not the name (`menu::Why::SameCommit`): on HEAD's
    /// own commit they grey, a chip naming another branch there too.
    readonly property string integrateRef:
        rowMenu.targetName !== "" && rowMenu.targetKind !== "worktree" ? rowMenu.targetName : rowMenu.oid
    /// The branch every one of these rows makes its sentence about (規約 §履歴を合流させる): the one the worktree
    /// holds, off the tree as it is (`WorktreeModel.movedBranch`).
    required property string branch

    /// The commit the menu was opened on, and the stash it is ("" on an ordinary commit).
    required property string oid
    required property string stashRef
    /// Whether a remote already has that commit: the rows that rewrite it from here up carry a tag and do not ask
    /// (デザイン規約 §回数押し「ローカルの履歴書き換えはクリック」).
    required property bool published
    /// Whether a rebase onto that commit would rewrite one a remote has — `<row>..HEAD`, what the rebase replays, as
    /// the left menu's `rebase` row reads it (`RefRowMenu.rebasePublished`). Not `published`: the commit pressed is the
    /// one the rebase lands on, not one it rewrites.
    required property bool rebasePublished
    /// The rows the menu and its cards have and why each is greyed, as core answered the stand
    /// (`CommitMenuState.lines`): a commit row's — its commit's, its name's and the cards' —, or a stash's.
    required property MenuLines lines
    /// Whether anything besides this branch still reaches the tip — what decides whether the drop row is held
    /// (`CommitMenuState.menuTipHeldElsewhere`, the same answer `RebasePlanRunBar` is handed).
    required property bool tipHeldElsewhere
    /// Whether the move to the row's name asks first (`CommitMenuState`).
    required property bool switchAsks
    /// …or leads to the worktree holding that name instead: the name that worktree goes by (`RefRowMenu.heldLeaf`).
    required property string heldLeaf
    /// That worktree is the repository's own: its name wears the house (`RefRowMenu.heldHome`).
    property bool heldHome: false
    /// What the `pull` row runs on that name — `here` / `aside`, "" for no row (`CommitMenuState.menuPullRuns`).
    required property string pullRuns
    /// The remote branch that name tracks, which the `own` reason names (`RefRowMenu.pullUpstream`).
    required property string pullUpstream
    /// How many uncommitted files `--hard` takes besides the commits (`status::Counts::hard_reset_takes`); 0 on a
    /// clean tree.
    required property int hardResetTakes
    /// Whether what `--hard` takes holds a file the discard record cannot copy (`WorktreeModel.hardResetNotCopied`),
    /// which holds the row in `danger` (デザイン規約 §回数押し の色の表).
    required property bool hardResetNotCopied
    /// git's answers about the branch's plain delete (refusal, landing, early check), live while the menu stands —
    /// passed straight through to `RefBranchMenu`, whose answers the delete rows read.
    property string refusedDelete: ""
    property string landedDelete: ""
    property string checkedBranch: ""
    property string checkedMerged: ""
    /// Another worktree's uncommitted row the third menu stands on: where that worktree is (as its row names it) and
    /// the name the row gives it; its rows are `lines`' (`CommitMenuState.standCarried`).
    property string carriedPath: ""
    property string carriedName: ""
    /// That worktree is the repository's own, whose name wears the house (`GraphModel.carriedAt`'s `home`).
    property bool carriedHome: false

    /// Move the worktree to the row's name, by the road a double-click takes (`RepoPage.switchToRef`).
    signal switchRequested(string kind, string name)
    signal branchHereRequested(string oidHex)
    signal tagHereRequested(string oidHex)
    signal squashRequested(string oidHex)
    signal dropRequested(string oidHex)
    /// The full interactive rebase, opened as a plan over `from^..HEAD`. Nothing runs until the plan's own button, so
    /// the row is a plain click.
    signal planRequested(string oidHex)
    signal resetRequested(string mode)
    signal applyStashRequested(string selector)
    signal popStashRequested(string selector)
    signal dropStashRequested(string selector)
    /// The BRANCH card's five, passed straight up — raised by its own rows and by the push row and delete table on the
    /// first level: the delete git may still refuse, the upstream set (`UpstreamFlow`) and taken off, the push, and the
    /// first push's question (`PublishFlow`).
    signal deleteRequested(string kind, string id, string name, string oidHex)
    signal upstreamRequested(string branch, string counterpart, bool unsettable)
    signal unsetUpstreamRequested(string branch)
    signal pushBranchRequested(string branch, string force, string expect)
    signal publishRequested(string branch, string upstream)
    signal cherryPickRequested(string oidHex)
    signal revertRequested(string oidHex)
    signal mergeRequested(string ref)
    signal rebaseRequested(string ref)
    /// `branch` is the name a fast-forward moves, "" for the worktree's own `git pull` (`RepoTab.pull`).
    signal pullRequested(string branch)
    signal checkDeleteRequested(string branch)
    signal forceDeleteRequested(string branch)
    signal deleteRemoteRequested(string remoteRef, string expect)
    signal deleteEverywhereRequested(string branch, string remoteRef, bool forced, string expect)
    signal pushTagRequested(string remote, string tag, string expect)
    signal deleteTagRequested(string tag)
    signal deleteRemoteTagRequested(string remote, string tag, bool rowGoes, string expect)
    signal deleteTagEverywhereRequested(string remote, string tag, string expect)
    /// The WORKTREE card's three, passed straight up (`RefWorktreeMenu`): the box for a new branch in a worktree of
    /// its own, the row's own branch out in one, and `worktree remove`.
    signal worktreeHereRequested(string oidHex)
    signal worktreeAddRequested(string mode, string branch, string start, string path, string name)
    signal removeWorktreeRequested(string path, string name, string holds)
    /// The third menu's two: that worktree opened, by the road its row's double-click takes, and its work applied here.
    signal carriedOpenRequested(string path)
    signal carriedApplyRequested(string path)
    /// A link on its way to the clipboard (`GitHubMenu`); the page owns the clipboard.
    signal copyRequested(string text)
    /// The menu went away — and the stacked list it may have been standing on is the pointer's to answer for again.
    signal dismissed()

    /// Either menu is on screen — what the page reads to hide hover behind it (`RepoPage.menuStanding`). Read this, not
    /// `visible`, which comes back stale from another file (`RefusalBadge`).
    readonly property bool showing: commitMenu.visible || stashMenu.visible || carriedMenu.visible
                                    || choiceMenu.visible
    /// Whether the one standing is the third, on another worktree's uncommitted row (`CommitMenuState.standAgain`).
    readonly property bool carriedShowing: carriedMenu.visible
    /// …or the fourth, on a choice of two commits, which stands on nothing a command can take away.
    readonly property bool choiceShowing: choiceMenu.visible

    /// The automation's handles into these rows, an automation-only exposure the same as `GraphPane.view` is
    /// (rules-refs/app-ui.md「製品の部品はハーネスへ答える相手を丸ごと渡す」).
    readonly property alias menu: commitMenu
    /// What a stash's row raises in its place.
    readonly property alias stashLevel: stashMenu
    readonly property alias branchHereItem: branchHereCommitItem
    readonly property alias switchItem: switchCommitItem
    readonly property alias pullItem: pullCommitItem
    readonly property alias pushBranchItem: pushCommitRow
    readonly property alias deleteItem: deleteCommitRow
    readonly property alias deleteRemoteItem: remoteDeleteCommitRow
    readonly property alias deleteBothItem: bothDeleteCommitRow
    readonly property alias tagHereItem: tagCommitMenu.tagHereItem
    readonly property alias dropItem: dropCommitItem
    readonly property alias rebaseInteractiveItem: rebaseInteractiveCommitItem
    readonly property alias stashDropItem: stashDeleteItem
    readonly property alias resetSubmenu: resetMenu
    readonly property alias hardResetRow: hardResetItem
    /// The three cards the rows used rarely hang behind (`AppMenu.openSub`).
    readonly property alias branchCard: branchCommitMenu
    readonly property alias worktreeCard: worktreeCommitMenu
    readonly property alias tagCard: tagCommitMenu
    readonly property alias carriedRowMenu: carriedMenu
    readonly property alias carriedOpenItem: carriedOpenRow
    readonly property alias carriedApplyItem: carriedApplyRow
    readonly property alias githubCard: githubCommitMenu
    readonly property alias choiceLevel: choiceMenu
    readonly property alias choiceGithubCard: githubChoiceMenu

    anchors.fill: parent

    /// Before a fresh stand, ahead of the answer: the first level a `stash`'s row, a `carried` one (another worktree's)
    /// or a `commit`'s raises becomes the one the answer is drawn into (`MenuLines.menu`), and the shape a standing
    /// menu held goes.
    function standFresh(kind) {
        rowMenu.lines.menu = kind === "stash" ? stashMenu : kind === "carried" ? carriedMenu : commitMenu
        rowMenu.lines.release()
    }

    // Each menu closes itself (a row's `dismiss()`, or the delete row when its answer lands), so nothing here
    // closes them — but for a row taken away while it stood, by the command it waited for or outside
    // (`CommitMenuState.standAgain`). Each raises the level `standFresh` drew the answer into.
    function offerStash() {
        stashMenu.offer()
    }
    function offerCarried() {
        carriedMenu.offer()
    }
    /// `facts` is what the cards stand on, read where the models are (`RefMenuFacts.stand`, aimed at the target): each
    /// card reads its part by name, and a card whose kind the target is not stands on nothing.
    function offerCommit(facts) {
        // Every card stands on its facts before `offer()`: its `applies` decides whether its row counts
        // (`AppMenu.offeredRows`).
        rowMenu.standCards(facts)
        commitMenu.offer()
    }
    /// The GITHUB card's half, before `offerCommit`: the links on the row's commit and its name
    /// (`GitHubLinks.ofRef`). Only as the menu opens — the links do not change with a command it waits for — and so
    /// kept out of `standCards`: rows handed again after the wait would be built anew by the card's `Instantiator`,
    /// which `holdShape` cannot hold.
    function standGithub(links) {
        githubCommitMenu.standOn(links)
    }
    /// The menu on a choice of two commits: the links that take both (`GitHubLinks.ofPair`). Says whether it
    /// opened — with nothing to offer, the press falls to the one-commit menu (`RepoPage.openRowMenu`).
    function offerChoice(links) {
        githubChoiceMenu.standOn(links)
        return choiceMenu.offer()
    }
    /// The cards' half of `offerCommit`, for the menu standing again on a newer reading as well — where the BRANCH
    /// card's branch may point elsewhere by then, `branchAt` (`CommitMenuState.aimedAt`); left out, the row's commit.
    /// `again`: why the menu stands again — `follow` or `wait` (`CommitMenuState.standAgain`, `RefBranchMenu.standOn`).
    function standCards(facts, branchAt, again) {
        worktreeCommitMenu.standOn(facts)
        const branchy = rowMenu.targetKind === "branch" || rowMenu.targetKind === "remote"
        const branchKind = branchy ? rowMenu.targetKind : ""
        const branchName = branchy ? rowMenu.targetName : ""
        branchCommitMenu.standOn(branchKind, branchName, branchName, branchAt === undefined ? rowMenu.oid : branchAt,
                                 facts, again)
        const tagged = rowMenu.targetKind === "tag"
        tagCommitMenu.standOn(tagged ? "tag" : "", tagged ? rowMenu.targetName : "", rowMenu.oid, facts, again)
    }
    /// The commit, the stash or the other worktree's work the menu stands on is gone — with the command it waited for,
    /// or outside while it stood: nothing left to act on. The pair's menu reads nothing of the tree, so it is never
    /// withdrawn.
    function withdraw() {
        (stashMenu.visible ? stashMenu : carriedMenu.visible ? carriedMenu : commitMenu).dismiss()
    }

    // A stash row's right-click opens this one instead of the commit menu: what brings the stash back and its delete,
    // as the left menu's stash row — a stash is on no branch's history, so no commit row applies. The details pane's
    // band says `apply` / `pop` too (`StashActionsBand`), but for the row chosen, which is not always the one
    // right-clicked (デザイン規約 §メニュー「右のペインは入口に数えない」). Code chips without `stash`: the menu hangs off a
    // stash row, where the word says the same on every row and picks none (デザイン規約 §git 用語のコード表記).
    AppMenu {
        id: stashMenu
        heldReason: rowMenu.heldReason
        runningWhy: rowMenu.runningWhy
        // `apply` keeps the entry, `pop` drops it once its changes are in: a plain click each, nothing is lost.
        AppMenuItem {
            code: "apply"
            word: "apply-stash"
            onTriggered: rowMenu.applyStashRequested(rowMenu.stashRef)
        }
        AppMenuItem {
            code: "pop"
            word: "pop-stash"
            onTriggered: rowMenu.popStashRequested(rowMenu.stashRef)
        }
        // Held (デザイン規約 §回数押し): what is dropped leaves the list, and only the discard record brings it back. The
        // left menu's stash row offers it in the same words (`RefRowMenu`).
        AppMenuItem {
            id: stashDeleteItem
            code: "drop"
            word: "drop-stash"
            holdPresses: Metrics.holdPressesWarned
            onHeld: {
                stashDeleteItem.closeMenu()
                rowMenu.dropStashRequested(rowMenu.stashRef)
            }
        }
    }

    // Another worktree's uncommitted row: its door, then its work brought here (デザイン規約 §別の worktree を読む).
    // These two rows only (user decision — ask before changing), and no WORKTREE card (`RefWorktreeMenu`):
    // `worktree remove` / `prune` destroy, and a worktree holding uncommitted work gets no row that takes it away.
    // It waits as the other two do (`runningWhy`, its end `MenuLines.commandEnded`). Headless `carried-menu` /
    // `carried-apply` / `carried-apply-refused`.
    AppMenu {
        id: carriedMenu
        heldReason: rowMenu.heldReason
        runningWhy: rowMenu.runningWhy
        onClosed: rowMenu.dismissed()
        // The row's double-click, in the words and seat a branch another worktree holds has (`switchCommitItem`).
        // Other products put the door on that row too (GitKraken: `Open Worktree` on each worktree's WIP node).
        AppMenuItem {
            id: carriedOpenRow
            //: The row that leads to the worktree whose uncommitted changes these are; its name follows it — the folder's,
            //: or the branch the repository's own worktree has out.
            text: qsTr("Open")
            nameMark: rowMenu.carriedHome ? "home" : "tree"
            codeName: rowMenu.carriedName
            word: "reach"
            onTriggered: rowMenu.carriedOpenRequested(rowMenu.carriedPath)
        }
        // Words, not a command, and no chip (user decision — ask before changing): it lands as `stash apply` lands,
        // but no one command a reader would type does this (デザイン規約 §git 用語のコード表記). The words say what is
        // applied (`changes`), as other products name it (VS Code `Migrate Worktree Changes`), about as long as the
        // menus' long rows (`Discard everything after it`). Not "here": in this app that is the row right-clicked
        // (`Create branch here…`), so it would read as applying onto that row. The whole worktree only, no row per
        // file (user decision — ask before changing).
        AppMenuItem {
            id: carriedApplyRow
            text: qsTr("Apply changes to this worktree")
            // Greyed, not taken away, while a conflict stands on either side (`menu::Row::ApplyChanges`).
            word: "apply-changes"
            whyNames: ({ "worktree": rowMenu.carriedName })
            // The worktree in that reason wears its mark, as the door above does.
            reasonMarkWord: carriedApplyRow.why === "conflicts-there" ? rowMenu.carriedName : ""
            reasonMarkKind: rowMenu.carriedHome ? "home" : "tree"
            onTriggered: rowMenu.carriedApplyRequested(rowMenu.carriedPath)
        }
    }

    AppMenu {
        id: commitMenu
        heldReason: rowMenu.heldReason
        runningWhy: rowMenu.runningWhy
        onClosed: rowMenu.dismissed()
        // Which rows stand, in what order and seat, is the type's answer (`MenuLines`); GITHUB, outside the type, at
        // the foot (デザイン規約 §メニュー).
        //
        // Words, not a command: it opens a box, and what runs depends on what is typed. On the menu though the
        // double-click opens the same box (user decision — ask before changing; the exception to デザイン規約 §メニュー
        // 「入口がそちら」): on most rows the double-click never reaches it. What it runs, `switch -c <name> <oid>`,
        // makes a branch and moves onto it, rightly read as a kind of moving to what is there (`menu::Group::Start`).
        AppMenuItem {
            id: branchHereCommitItem
            text: Words.createBranchHere
            word: "branch-here"
            onTriggered: rowMenu.branchHereRequested(rowMenu.oid)
        }
        // The name is the chip's, so the row does not repeat it; landing is by name, since the commit itself would
        // leave HEAD on no branch. Here as well as on the double-click (user decision — ask before changing, the same
        // exception): the menu is the one place to read what the press will do before it.
        AppMenuItem {
            id: switchCommitItem
            // A branch another worktree holds leads to that worktree, in the same words as the sidebar's row
            // (デザイン規約 §メニュー).
            code: rowMenu.heldLeaf === "" ? "switch" : ""
            //: The row that leads to the worktree holding this branch; its name follows it — the folder's, or the branch
            //: the repository's own worktree has out.
            text: rowMenu.heldLeaf === "" ? "" : qsTr("Open")
            nameMark: rowMenu.heldLeaf === "" ? "" : rowMenu.heldHome ? "home" : "tree"
            codeName: rowMenu.heldLeaf
            // Greyed only while the tab opens: where the move would ask first, `asks` marks the row before the press
            // (as `RefRowMenu`).
            word: "reach"
            asks: rowMenu.switchAsks
            onTriggered: rowMenu.switchRequested(rowMenu.targetKind, rowMenu.targetName)
        }
        AppMenuItem {
            code: "cherry-pick"
            word: "cherry-pick"
            onTriggered: rowMenu.cherryPickRequested(rowMenu.oid)
        }
        // Both cherry-pick and revert only add a commit, so both are a plain click. From here on, a row whose reason
        // names HEAD's branch (`head-unborn`) seats that name's mark in it, as its own words wear it.
        AppMenuItem {
            id: revertCommitItem
            code: "revert"
            word: "revert"
            whyNames: ({ "head": rowMenu.branch })
            reasonMarkWord: revertCommitItem.why === "head-unborn" ? rowMenu.branch : ""
            reasonMarkKind: "branch"
            onTriggered: rowMenu.revertRequested(rowMenu.oid)
        }
        // The message is edited in the details pane's boxes, where a right-click on the row has put it — not one in a
        // chip's card, which chooses no row (デザイン規約 §メニュー「右のペインは入口に数えない」). A click, pushed or not
        // (`already pushed` only says so). The words name only where it goes: the clicked row is the one folded, into
        // `parent` — the details pane's parent link already says `Go to parent commit`.
        AppMenuItem {
            id: squashCommitItem
            code: "squash"
            //: Follows the `squash` chip: "squash into parent".
            text: qsTr("into parent")
            warning: rowMenu.published ? qsTr("already pushed") : ""
            word: "squash"
            whyNames: ({ "head": rowMenu.branch })
            reasonMarkWord: squashCommitItem.why === "head-unborn" ? rowMenu.branch : ""
            reasonMarkKind: "branch"
            onTriggered: rowMenu.squashRequested(rowMenu.oid)
        }
        // Held while this branch alone holds its tip; a plain click once something else does, since the commits then
        // stay drawn and a cherry-pick brings them back (デザイン規約 §回数押し; what counts as holding:
        // `reachable.rs`). The row's warning colour goes with the mark: only where something is lost. The only row
        // here that takes a commit away. Local only: taking it off a remote needs a force push, which has its own
        // hold.
        AppMenuItem {
            id: dropCommitItem
            code: "drop"
            warning: rowMenu.published ? qsTr("already pushed") : ""
            word: "drop"
            whyNames: ({ "head": rowMenu.branch })
            reasonMarkWord: dropCommitItem.why === "head-unborn" ? rowMenu.branch : ""
            reasonMarkKind: "branch"
            holdPresses: rowMenu.tipHeldElsewhere ? 0 : Metrics.holdPressesWarned
            // The commit is the row's; the branch whose history it leaves is HEAD's, and the warning is the reading's —
            // a press made on another branch, or without the warning, says nothing of it.
            premise: rowMenu.branch + "\n" + dropCommitItem.warning
            onTriggered: rowMenu.dropRequested(rowMenu.oid)
            onHeld: {
                dropCommitItem.closeMenu()
                rowMenu.dropRequested(rowMenu.oid)
            }
        }
        // The clicked commit is the oldest one included; the plan shows the base it lands on as its own last row
        // (`RebasePlanPane`). A plain click — it opens a plan, and nothing is lost yet.
        AppMenuItem {
            id: rebaseInteractiveCommitItem
            code: "rebase --interactive"
            //: Follows the `rebase --interactive` chip: "from here up".
            text: qsTr("from here up")
            warning: rowMenu.published ? Words.rewritesPushed : ""
            word: "rebase-interactive"
            whyNames: ({ "head": rowMenu.branch })
            reasonMarkWord: rebaseInteractiveCommitItem.why === "head-unborn" ? rowMenu.branch : ""
            reasonMarkKind: "branch"
            onTriggered: rowMenu.planRequested(rowMenu.oid)
        }
        AppMenuItem {
            code: "merge"
            refSentence: Words.intoBranch
            refName: rowMenu.branch
            refMark: "branch"
            word: "merge"
            onTriggered: rowMenu.mergeRequested(rowMenu.integrateRef)
        }
        AppMenuItem {
            id: rebaseCommitItem
            code: "rebase"
            //: Follows the `rebase` chip: "rebase main onto it".
            refSentence: qsTr("%1 onto it")
            refName: rowMenu.branch
            refMark: "branch"
            warning: rowMenu.rebasePublished ? Words.rewritesPushed : ""
            word: "rebase"
            whyNames: ({ "head": rowMenu.branch })
            reasonMarkWord: rebaseCommitItem.why === "head-unborn" ? rowMenu.branch : ""
            reasonMarkKind: "branch"
            onTriggered: rowMenu.rebaseRequested(rowMenu.integrateRef)
        }
        // The branch taken back to this commit. `reset` is git's word for two jobs — moving the branch, and taking a
        // path out of the index, which this application calls **Unstage** — so the move puts the command itself on the
        // title as its verb: the `reset` chip, then `<branch> here` (規約 §git 用語のコード表記). The command is named
        // once, there; each row keeps only its flag chip and a sentence saying what is kept or discarded, so a hand
        // that knows `reset --soft` maps rows to commands at a glance and an eye that does not reads the sentence
        // alone. `here`: the commit meant is the row the menu opened on, which the row already shows. Only `--hard`
        // asks — a count of presses on its own row; the question bar is too much machinery for a one-line question —
        // since `--soft` / `--mixed` discard no byte of content. The row does not say that `--hard` leaves untracked
        // files: a menu row says what fits (規約 §長さ). When the card stands and why its rows grey:
        // `menu::Part::MovedTo` and `menu::Why`.
        AppMenu {
            id: resetMenu
            // Every row inside greyed — a stopped operation, HEAD's own commit — greys the title with them, and it
            // still opens to show why (`AppMenuItem.cardOut`).
            heldReason: rowMenu.heldReason
            runningWhy: rowMenu.runningWhy
            titleCode: "reset"
            //: Follows the `reset` chip: "reset main here".
            titleSentence: qsTr("%1 here")
            // Named off the tree as it is: a replay's detached HEAD still holds the branch it rewrites
            // (`WorktreeModel.movedBranch`). No stand-in word fills an empty one.
            titleRef: rowMenu.branch
            titleRefMark: "branch"
            word: "reset"
            AppMenuItem {
                code: "--soft"
                text: qsTr("Keep everything, staged")
                word: "reset-soft"
                onTriggered: rowMenu.resetRequested("soft")
            }
            AppMenuItem {
                code: "--mixed"
                text: qsTr("Keep everything, unstaged")
                word: "reset-mixed"
                onTriggered: rowMenu.resetRequested("mixed")
            }
            // Held, in the hold's colour before it is touched (規約 §回数押しの色). The sentence names the commits
            // (everything after it), but `--hard` overwrites the index and every tracked path of the working tree,
            // which are not "after" anything: nothing else on screen says they go, and the reflog does not keep them.
            // So the warning says it first, in the seat `already pushed` takes (規約 §メニュー) — the hold already asks
            // for the consent.
            AppMenuItem {
                id: hardResetItem
                code: "--hard"
                text: qsTr("Discard everything after it")
                // Two words, as the tags (`already pushed` / `not merged` here, `OpExitCard`'s `conflicts stay`): what
                // tells it apart is the *uncommitted* part, and `too` means "besides the commits". Not `worktree`:
                // the left menu's `WORKTREES` lists git worktrees, so the word would name another thing. No count:
                // the uncommitted row behind the menu counts from the same status, and `%n … file(s)` would show
                // `(s)` verbatim, since no translation loads (規約 §タイポグラフィ「複数形を `(s)` で畳まない」). Tracked
                // files only (`hardResetTakes`): over a tree dirty with untracked files alone there is no warning.
                warning: rowMenu.hardResetTakes > 0 ? qsTr("uncommitted too") : ""
                word: "reset-hard"
                holdPresses: Metrics.holdPressesWarned
                // The commits come back from the branch's reflog and the uncommitted changes from the discard
                // record's copy — unless a file in them is one the copy cannot take (デザイン規約 §回数押し の色の表).
                holdTone: rowMenu.hardResetNotCopied ? Theme.danger : Theme.warning
                // The branch it moves, and what the warning and the tone say goes: presses begun on a tree with nothing
                // uncommitted, or on one the copy takes whole, are not consent to more.
                premise: [rowMenu.branch, hardResetItem.warning, rowMenu.hardResetNotCopied].join("\n")
                onHeld: {
                    // This card and the menu it hangs off go together (`AppMenuItem.closeMenu`).
                    hardResetItem.closeMenu()
                    rowMenu.resetRequested("hard")
                }
            }
        }
        // The sidebar's `pull` row (wording and colour: `RefRowMenu`'s pull row).
        AppMenuItem {
            id: pullCommitItem
            code: "pull"
            // The branch that moves, in its own colour: the worktree's, or the chip's where the pull runs aside.
            refSentence: Words.intoBranch
            refName: rowMenu.pullRuns === "aside" ? rowMenu.targetName : rowMenu.branch
            refTint: rowMenu.pullRuns === "aside" ? Theme.textPrimary : Theme.textLink
            refMark: "branch"
            word: "pull"
            whyNames: ({ "holder": rowMenu.heldLeaf, "upstream": rowMenu.pullUpstream })
            reasonMarkWord: pullCommitItem.why === "held" ? rowMenu.heldLeaf : ""
            reasonMarkKind: rowMenu.heldHome ? "home" : "tree"
            reasonMarks: pullCommitItem.why === "own"
                         ? Words.listMarks(pullCommitItem.blockedReason, rowMenu.pullUpstream, "remote") : []
            onTriggered: rowMenu.pullRequested(rowMenu.pullRuns === "aside" ? rowMenu.targetName : "")
        }
        // The sidebar's `push` row: the chip's branch sent, on the BRANCH card's answers.
        BranchPushRow {
            id: pushCommitRow
            card: branchCommitMenu
        }
        // The chip's delete table, on the BRANCH card's answers. A row that draws no branch has none of the three.
        BranchDeleteRow {
            id: deleteCommitRow
            card: branchCommitMenu
        }
        BranchRemoteDeleteRow {
            id: remoteDeleteCommitRow
            card: branchCommitMenu
        }
        BranchBothDeleteRow {
            id: bothDeleteCommitRow
            card: branchCommitMenu
        }
        // The same card the sidebar's row opens (デザイン規約 §メニュー); a row that draws no branch hands it "", and its
        // class has no BRANCH rows.
        RefBranchMenu {
            id: branchCommitMenu
            heldReason: rowMenu.heldReason
            runningWhy: rowMenu.runningWhy
            refusedDelete: rowMenu.refusedDelete
            landedDelete: rowMenu.landedDelete
            checkedBranch: rowMenu.checkedBranch
            checkedMerged: rowMenu.checkedMerged
            onDeleteRequested: (kind, id, name, oidHex) => rowMenu.deleteRequested(kind, id, name, oidHex)
            onUpstreamRequested: (branch, counterpart, unsettable) =>
                rowMenu.upstreamRequested(branch, counterpart, unsettable)
            onUnsetUpstreamRequested: branch => rowMenu.unsetUpstreamRequested(branch)
            onPushRequested: (branch, force, expect) => rowMenu.pushBranchRequested(branch, force, expect)
            onPublishRequested: (branch, upstream) => rowMenu.publishRequested(branch, upstream)
            onCheckDeleteRequested: branch => rowMenu.checkDeleteRequested(branch)
            onForceDeleteRequested: branch => rowMenu.forceDeleteRequested(branch)
            onDeleteRemoteRequested: (remoteRef, expect) => rowMenu.deleteRemoteRequested(remoteRef, expect)
            onDeleteEverywhereRequested: (branch, remoteRef, forced, expect) =>
                rowMenu.deleteEverywhereRequested(branch, remoteRef, forced, expect)
        }
        // The same WORKTREE card as the sidebar's (デザイン規約 §メニュー の入れ子).
        RefWorktreeMenu {
            id: worktreeCommitMenu
            heldReason: rowMenu.heldReason
            runningWhy: rowMenu.runningWhy
            onWorktreeHereRequested: oidHex => rowMenu.worktreeHereRequested(oidHex)
            onWorktreeAddRequested: (mode, branch, start, path, name) =>
                rowMenu.worktreeAddRequested(mode, branch, start, path, name)
            onRemoveRequested: (path, name, holds) => rowMenu.removeWorktreeRequested(path, name, holds)
        }
        // The same TAG card as the sidebar's (デザイン規約 §メニュー).
        RefTagMenu {
            id: tagCommitMenu
            heldReason: rowMenu.heldReason
            runningWhy: rowMenu.runningWhy
            onTagHereRequested: oidHex => rowMenu.tagHereRequested(oidHex)
            onPushTagRequested: (remote, tag, expect) => rowMenu.pushTagRequested(remote, tag, expect)
            onDeleteTagRequested: tag => rowMenu.deleteTagRequested(tag)
            onDeleteRemoteTagRequested: (remote, tag, rowGoes, expect) =>
                rowMenu.deleteRemoteTagRequested(remote, tag, rowGoes, expect)
            onDeleteTagEverywhereRequested: (remote, tag, expect) =>
                rowMenu.deleteTagEverywhereRequested(remote, tag, expect)
        }
        AppMenuSeparator {}
        // GitHub's pages for this commit and the name the chip draws, last: they leave the window, every card above
        // acts in it (デザイン規約 §GitHub のページへのリンク).
        GitHubMenu {
            id: githubCommitMenu
            onCopyRequested: text => rowMenu.copyRequested(text)
        }
    }

    // A choice of two commits, right-clicked inside it: what takes the pair — every row above takes one commit, and a
    // right-click outside the choice takes it down to that row (デザイン規約 §複数のコミットを選ぶ). Nothing here starts
    // git, so it neither waits nor stands again.
    AppMenu {
        id: choiceMenu
        onClosed: rowMenu.dismissed()
        GitHubMenu {
            id: githubChoiceMenu
            onCopyRequested: text => rowMenu.copyRequested(text)
        }
    }
}
