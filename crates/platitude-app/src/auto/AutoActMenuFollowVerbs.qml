pragma ComponentBehavior: Bound

import QtQuick
import platitude
import platitude.ui

/// A menu raised with nothing running, and the repository changed under it from outside — git run in the run's
/// repository as a terminal would (`Harness.runGit`), or a file written as an editor saves it (`Harness.writeFile`).
/// The window hears it only by reading the tree again. The menu stays up and follows the tree as it is
/// (`MenuLines.moved`; デザイン規約 §メニュー「今の内容」): the names its rows say, the notes,
/// and the rows that no longer apply greyed where they stand. Over `--preset replay`, on `base`, the branch that is not
/// the one checked out; the forms that reach over there over `--preset others-behind`, on `feature/mine` (ahead of
/// `origin/feature/mine`) and that reading's own row.
// `Item` because `QtObject` has no default property to hold the sampler below.
Item {
    id: acts

    /// `var`: typing it `AutoActDriver` would be circular — that file builds this one.
    required property var driver

    readonly property var page: driver.page
    readonly property var repoTab: driver.repoTab
    readonly property var worktree: driver.worktree
    readonly property var graphModel: driver.graphModel
    readonly property var graphPane: driver.graphPane
    readonly property var branchesModel: driver.branchesModel
    readonly property var commitMenu: driver.commitMenu
    readonly property var commitMenuState: driver.commitMenuState
    readonly property var resetMenu: driver.resetMenu
    readonly property var hardResetItem: driver.hardResetItem
    readonly property var refMenu: driver.refMenu
    readonly property var refRowMenu: driver.refRowMenu
    readonly property var remotesModel: driver.remotesModel
    readonly property var refBranchCard: driver.refBranchCard
    readonly property var refWorktreeCard: driver.refWorktreeCard
    readonly property var refGithubCard: driver.refGithubCard
    readonly property var worktreesModel: driver.worktreesModel

    /// Runs `act` if it is this family's, and says whether it was (`AutoActDriver` asks each family in turn).
    function run(act, arg) {
        if (act !== "menu-follow")
            return false
        followTimer.form = arg === "" ? "rename" : arg
        // The reading's own row is in its remote's fold, which starts closed: opened once here, not in the press that
        // raises the menu, which is sent again until it takes.
        if (followTimer.form === "remote-moved")
            acts.remotesModel.toggleFolder(
                GitFacts.remoteBranchOf(followTimer.reading, acts.repoTab.remoteNames).remote)
        followTimer.order = followTimer.withdraws ? ["raised", "changed", "tipped"] : ["raised", "changed"]
        followTimer.start()
        return true
    }

    // `menu-follow` / `menu-follow rename`: the graph row's menu on `base`, and `main` renamed outside to `trunk`
    // (`git branch -m`). `named=` is the branch the `reset` card's title names: the name the rows say follows the tree.
    // `menu-follow detach`: the same menu, and HEAD taken off its branch outside (`git switch --detach`) — the rows
    // that name the branch no longer apply, and grey where they stand (`withdrawn=`, their chips) rather than stay
    // pressable with no name in them, the `reset` card's inside it and its title greying with them
    // (`AppMenuItem.cardOut`); the first of them shows its tooltip, forced (verify-ui §hover の絵の撮り方), saying to
    // open the menu again (`tip=`). `menu-follow detach-ref`: the same over the left menu's `base` row.
    // `menu-follow renamed`: the same menu, aimed at `base`'s own chip, and `base` itself renamed outside to
    // `base-two` — told apart from a branch deleted by its reflog (`NavSectionModel.renamedTo`), the menu stays up and
    // goes on standing on the branch under its new name (`aimed=`) rather than close as for a branch gone (§hover「中身
    // が残っていれば字だけ替え」). `menu-follow renamed-ref`: the same over the left menu's `base` row, whose door goes
    // with it (`door=`: the page's menu door and the row its name boxes open on answer to the new name,
    // `RepoPage.followRenamedDoor`).
    // `menu-follow dirty`: the menu with its `reset` card open, and a tracked file written outside — the `--hard` row's
    // warning (`warning=`) says the uncommitted change now there, and the card grows to hold it whole (`width=grew`),
    // as a card opened on that tree stands. `menu-follow written`: the rename made by the window's own write
    // (`RepoTab.renameBranch`) under a menu raised before it — not greyed, it keeps what it stood on while the write
    // runs, and stands again on what it left once it has ended (`RepoPage.treeAtAsk` back to null: the reading behind
    // the write comes in before that, `RepoTab.settling`). The rows that reach over there stay on what the menu went
    // up with and grey once that moved (`RefBranchMenu.standOn`; デザイン規約 §メニュー の例外): `menu-follow
    // remote-moved`, the left menu on the reading `origin/feature/mine` and that remote branch moved outside
    // (`git update-ref`, as a fetch would) — the menu stays up (§hover「外乱で閉じない」) and its `push --delete`
    // greys; `menu-follow far-moved`, the left menu on `feature/mine` and the same move — its push and the two deletes
    // that reach over there grey; `menu-follow upstream-moved`, the same menu and the branch pointed at another
    // upstream outside (`git branch -u`) — they grey too, still naming the reading they showed (`kept=`). `menu-follow
    // upstream-unset`, the same menu with its BRANCH card open and the upstream taken off outside
    // (`git branch --unset-upstream`): the card's `Unset upstream` row, a row with no chip, greys and keeps its words
    // without the name it no longer has (`withdrawn=Unsetupstream`; `AppMenuItem.sentence`). `same=` holds the rows on
    // show to the ones the menu went up with, through every form: nothing comes or goes under the hand.
    // `menu-follow worktree-moved` (`--preset worktrees`): the left menu on the WORKTREES row `topic` — the menu of
    // `feature/topic-a`, the branch it has out — and that worktree taken off its branch outside (`git switch
    // --detach` there): only the row on the worktree itself, the WORKTREE card's `worktree remove`, still acts on
    // what was clicked, and every other greys where it stands (`RefRowMenu.standOn`). `live=` is the rows on show
    // that can still be chosen, title rows' included — a card's title greys over a card of grey rows
    // (`AppMenuItem.cardOut`), the WORKTREE card's stays live over its one live row —, `card=` that card's own,
    // `links=` whether the GITHUB card's links still apply (`GitHubMenu.stillApplies` — told on a repository with
    // no GitHub remote too, where none is drawn).
    // `menu-follow worktree-removed`: the same menu, and that worktree removed outside (`git worktree remove`): the
    // branch is still there, but what was clicked is gone, so no row acts on it — the WORKTREE card's greys too.
    PageStage {
        id: followTimer
        name: "menu_follow"
        order: ["raised", "changed"]
        /// `rename`, `detach`, `detach-ref`, `renamed`, `renamed-ref`, `dirty`, `written`, `remote-moved`,
        /// `far-moved`, `upstream-moved`, `upstream-unset`, `worktree-moved` or `worktree-removed`, as above.
        property string form: "rename"
        /// The form whose change is the window's own write.
        readonly property bool writes: followTimer.form === "written"
        /// The forms that reach over there, on `--preset others-behind`.
        readonly property bool overThere: followTimer.form === "remote-moved" || followTimer.form === "far-moved"
                                          || followTimer.form === "upstream-moved"
                                          || followTimer.form === "upstream-unset"
        /// The forms in which the worktree whose row raised the menu leaves its branch outside — taken off it, or
        /// removed —, and that worktree's folder.
        readonly property bool worktreeLeaves: followTimer.form === "worktree-moved"
                                               || followTimer.form === "worktree-removed"
        readonly property string worktreeName: "topic"
        /// The forms that rename the branch the menu stands on.
        readonly property bool renames: followTimer.form === "renamed" || followTimer.form === "renamed-ref"
        /// The forms raised from the left menu.
        readonly property bool leftMenu: followTimer.form === "detach-ref" || followTimer.form === "renamed-ref"
                                         || followTimer.overThere || followTimer.worktreeLeaves
        /// The reading the forms over there stand on, and where the moves send it.
        readonly property string reading: "origin/feature/mine"
        readonly property string movedTo: "origin/release"
        /// A form whose change takes rows off what the menu applies to: their tooltip is photographed.
        readonly property bool withdraws: followTimer.form === "detach" || followTimer.form === "detach-ref"
                                          || followTimer.overThere || followTimer.worktreeLeaves
        /// The path of the worktree the form moves; empty while the listing does not hold it.
        function worktreePath() {
            for (let at = 0; at < acts.worktreesModel.total; at++) {
                if (acts.worktreesModel.nameAt(at) === followTimer.worktreeName)
                    return acts.worktreesModel.fullAt(at)
            }
            return ""
        }
        /// The first row on show that greyed where it stands; null for none.
        function firstWithdrawn() {
            const level = followTimer.level
            for (let i = 0; i < level.count; i++) {
                const row = level.itemAt(i)
                if (row && row.present && row.withdrawn === true && !row.subMenu)
                    return row
            }
            return null
        }
        /// The level the menu opens as: the graph row's, or the left menu's — its BRANCH card for the form that takes
        /// the upstream off.
        readonly property var level: followTimer.form === "upstream-unset" ? acts.refBranchCard
                                   : followTimer.leftMenu ? acts.refMenu : acts.commitMenu
        /// What that menu's rows were read off (`CommitMenuState.tree` / `RefRowMenu.tree`).
        function stoodOn() {
            return followTimer.leftMenu ? acts.refRowMenu.tree : acts.commitMenuState.tree
        }
        /// The menu stood again on a newer reading with the card up: its rows hold their shape (`AppMenu.holdShape`).
        /// What the forms over there read, where the move leaves the worktree's own reading as it was.
        function stoodAgain() {
            for (let i = 0; i < followTimer.level.count; i++) {
                const row = followTimer.level.itemAt(i)
                if (row && row.shapeHeld !== undefined)
                    return row.shapeHeld
            }
            return false
        }
        /// The menu raised on `base`: the row's own right-click (`GraphRowDelegate.askMenu`), or the left menu's door.
        /// False while the row has no delegate to answer. The `dirty` form opens the `reset` card too. The forms over
        /// there raise the left menu on `feature/mine`, or on its reading's own row with that remote's fold opened.
        function raise() {
            if (followTimer.worktreeLeaves) {
                const path = followTimer.worktreePath()
                const head = path === "" ? "" : acts.worktreesModel.headOfWorktree(path)
                if (head === ""
                    || !acts.page.openRefMenu("worktree", followTimer.worktreeName, path, head, true))
                    return false
                followTimer.witness("rows", followTimer.level.offeredRows)
                return true
            }
            if (followTimer.overThere) {
                const remote = followTimer.form === "remote-moved"
                const name = remote ? followTimer.reading : "feature/mine"
                const there = remote ? acts.remotesModel.oidOfName(name) : acts.branchesModel.oidOfName(name)
                if (there === "")
                    return false
                if (!acts.page.openRefMenu(remote ? "remote" : "branch", name, name, there, true))
                    return false
                if (followTimer.form === "upstream-unset")
                    acts.refMenu.openSub(acts.refBranchCard)
                followTimer.witness("rows", followTimer.level.offeredRows)
                return true
            }
            const oid = acts.branchesModel.oidOfName("base")
            if (oid === "")
                return false
            if (followTimer.form === "detach-ref" || followTimer.form === "renamed-ref") {
                if (!acts.page.openRefMenu("branch", "base", "base", oid, true))
                    return false
                followTimer.witness("rows", followTimer.level.offeredRows)
                return true
            }
            const item = acts.graphPane.view.itemAtIndex(acts.graphModel.rowOf(oid))
            if (item === null)
                return false
            item.askMenu()
            if (!acts.commitMenu.visible)
                return false
            if (followTimer.form === "dirty") {
                acts.commitMenu.openSub(acts.resetMenu)
                followTimer.witness("cardWidth", acts.resetMenu.width)
            }
            followTimer.witness("rows", followTimer.level.offeredRows)
            return true
        }
        /// The change made outside, as a program other than the window would make it — or, `written`, by the window's
        /// own write. Says whether it was made (put in).
        function change() {
            if (followTimer.writes) {
                return acts.driver.pressWrite("branch", () => {
                    acts.repoTab.renameBranch("main", "trunk", false)
                    return true
                })
            }
            const repo = acts.repoTab.repoPath
            if (followTimer.form === "worktree-moved")
                return Harness.runGit(followTimer.worktreePath(), ["switch", "-q", "--detach"].join("\n"))
                              .startsWith("0 ")
            if (followTimer.form === "worktree-removed")
                return Harness.runGit(repo, ["worktree", "remove", followTimer.worktreePath()].join("\n"))
                              .startsWith("0 ")
            if (followTimer.form === "dirty")
                return Harness.writeFile(repo + "/notes.txt", "written outside\n") === ""
            const args = followTimer.form === "rename" ? ["branch", "-m", "main", "trunk"]
                       : followTimer.renames ? ["branch", "-m", "base", "base-two"]
                       : followTimer.form === "upstream-moved" ? ["branch", "-u", followTimer.movedTo, "feature/mine"]
                       : followTimer.form === "upstream-unset" ? ["branch", "--unset-upstream", "feature/mine"]
                       : followTimer.overThere ? ["update-ref", "refs/remotes/" + followTimer.reading,
                                                  "refs/remotes/" + followTimer.movedTo]
                       : ["switch", "-q", "--detach"]
            return Harness.runGit(repo, args.join("\n")).startsWith("0 ")
        }
        /// The tree read again with the change in it, and the menu standing on that reading.
        function followed() {
            if (followTimer.form === "worktree-moved") {
                const moved = acts.worktreesModel.worktreeFacts(followTimer.worktreePath())
                return moved !== undefined && moved.branch === "" && followTimer.stoodAgain()
            }
            if (followTimer.form === "worktree-removed")
                return followTimer.worktreePath() === "" && followTimer.stoodAgain()
            if (followTimer.form === "upstream-moved")
                return acts.refBranchCard.state.upstream === followTimer.movedTo && followTimer.stoodAgain()
            if (followTimer.form === "upstream-unset")
                return acts.refBranchCard.state.upstream === "" && followTimer.stoodAgain()
            // Read off the tree first: a menu that took the rename for a branch gone has closed by then, and says so
            // (`open=false`) rather than wait out the watchdog.
            if (followTimer.renames)
                return acts.branchesModel.oidOfName("base-two") !== "" && acts.branchesModel.oidOfName("base") === ""
                       && (!followTimer.level.visible || followTimer.aimed() === "base-two" && followTimer.stoodAgain())
            if (followTimer.overThere) {
                const moved = acts.remotesModel.oidOfName(followTimer.reading)
                return moved !== "" && moved === acts.remotesModel.oidOfName(followTimer.movedTo)
                       && followTimer.stoodAgain()
            }
            const tree = followTimer.stoodOn()
            if (followTimer.form === "rename" || followTimer.writes)
                return acts.worktree.branch === "trunk" && tree !== null && tree.branch === "trunk"
            if (followTimer.form === "dirty")
                return acts.worktree.hardResetTakes > 0 && tree !== null && tree.hardResetTakes > 0
            return acts.worktree.detached && tree !== null && tree.detached
        }
        /// The chips of the rows on show that greyed where they stand, title rows' included — each one word, its spaces
        /// left out (`rebase--interactive`), so the line reads as words.
        function withdrawn() {
            const level = followTimer.level
            const chips = []
            for (let i = 0; i < level.count; i++) {
                const row = level.itemAt(i)
                if (row && row.present && row.withdrawn === true)
                    chips.push((row.code !== "" ? row.code : row.text).replace(/ /g, ""))
            }
            return chips.length === 0 ? "none" : chips.join(",")
        }
        /// The chips of the rows on show in `menu` that can still be chosen (`AppMenuItem.greyed`), as `withdrawn`
        /// words them.
        function live(menu) {
            const chips = []
            for (let i = 0; i < menu.count; i++) {
                const row = menu.itemAt(i)
                if (row && row.present && row.greyed === false)
                    chips.push((row.code !== "" ? row.code : row.text).replace(/ /g, ""))
            }
            return chips.length === 0 ? "none" : chips.join(",")
        }
        /// The branch the menu stands on: the left menu's ref, or the name the graph row's menu was aimed at.
        function aimed() {
            return followTimer.leftMenu ? acts.refRowMenu.refId : acts.commitMenuState.menu.targetName
        }
        /// How the `reset` card's width moved from the one it went up with: `grew`, `held` or `shrank`.
        function widthMoved() {
            const was = followTimer.witnessed("cardWidth")
            return acts.resetMenu.width > was ? "grew" : acts.resetMenu.width < was ? "shrank" : "held"
        }
        /// Whether the forced tooltip is up saying the menu has to be raised again.
        function tipSays() {
            const row = followTimer.firstWithdrawn()
            return row !== null && row.tip.shown && row.blockedWhy === row.withdrawnWhy ? "again" : "none"
        }
        function line() {
            const level = followTimer.level
            const named = followTimer.leftMenu ? acts.refRowMenu.rebaseItem.refName : acts.resetMenu.titleRef
            return "menu_follow form=" + followTimer.form
                   + " open=" + level.visible
                   + " same=" + (level.offeredRows === followTimer.witnessed("rows"))
                   + " withdrawn=" + followTimer.withdrawn()
                   + " named=" + named
                   + (followTimer.withdraws ? " tip=" + followTimer.tipSays() : "")
                   + (followTimer.renames ? " aimed=" + followTimer.aimed() : "")
                   + (followTimer.form === "renamed-ref"
                      ? " door=" + (acts.page.menuKey === "branch:" + followTimer.aimed()
                                    && acts.page.refMenuRowId === followTimer.aimed())
                      : "")
                   + (followTimer.form === "dirty"
                      ? " warning=" + (acts.hardResetItem.tag !== "") + " width=" + followTimer.widthMoved()
                      : "")
                   + (followTimer.worktreeLeaves
                      ? " live=" + followTimer.live(level) + " card=" + followTimer.live(acts.refWorktreeCard)
                        + " links=" + acts.refGithubCard.stillApplies
                      : "")
                   + (followTimer.overThere && followTimer.form !== "remote-moved"
                      && followTimer.form !== "upstream-unset"
                      ? " kept=" + acts.refBranchCard.state.remoteCounterpart : "")
        }
        onTriggered: {
            const op = followTimer
            if (op.at("start")) {
                if (!Awaited.all("menu_follow_idle", { "open": acts.repoTab.state === "open",
                                                       "idle": acts.repoTab.busyCount === 0,
                                                       "page": PageSettled.settled(acts.page) }))
                    return
                op.input("raised", () => op.raise())
                return
            }
            if (op.at("raised")) {
                if (!Awaited.all("menu_follow_up", { "menu": op.level.opened,
                                                     "card": op.form !== "dirty" || acts.resetMenu.opened }))
                    return
                op.input("changed", () => op.change())
                return
            }
            if (op.withdraws && op.at("changed")) {
                if (!Awaited.all("menu_follow_read", { "followed": op.followed(),
                                                       "row": op.firstWithdrawn() !== null }))
                    return
                op.input("tipped", () => {
                    op.firstWithdrawn().tipForced = true
                    return true
                })
                return
            }
            op.complete({
                "name": "menu_follow_stood",
                "when": () => ({ "followed": op.followed(), "menu": op.renames || op.level.visible,
                                 "tip": !op.withdraws || op.tipSays() === "again",
                                 "written": !op.writes || acts.driver.wroteAndSettled(),
                                 "page": PageSettled.settled(acts.page) }),
                "report": () => op.line() })
        }
    }
}
