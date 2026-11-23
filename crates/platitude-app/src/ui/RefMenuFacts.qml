import QtQuick
import platitude
import platitude.ui

// ---- what a ref's menu stands on ------------------------------------
// Read off the sections in one place for both entrances — the sidebar's (`RefRowMenu.standOn`) and the graph row's
// (`CommitMenuState.standRow`) — so the two menus on one ref offer the same rows for the same reasons. The rules
// are core's (`GitFacts.refMenu`); this part only asks the sections what those rules read, and hands their records
// on whole.
//
// A `QtObject`: it holds no child, and the entrances hold it as a property value (rules-refs/app-ui.md「Item の中に
// 置く `QtObject` 製の部品は読み取り専用プロパティの値にする」).
QtObject {
    id: refMenuFacts

    required property RepoTab repoTab
    required property GraphModel graphModel
    /// Where the row leads (`leadOf`), the WORKTREE card's worktree and where a new one would go.
    required property NavSectionModel worktreesModel
    /// What a branch's card reads of its upstream, its push, and whether the row's local branch is there.
    required property NavSectionModel branchesModel
    /// The local branch the row lands on (`landsOn` — the one entrance for it).
    required property NavSectionModel remotesModel
    /// What a tag's card reaches (`tagMenu`) and where the tag here points.
    required property NavSectionModel tagsModel

    /// Reads the row `full` of `kind` on the commit `oidHex` off `tree` — the reading the menu stands on, handed to
    /// `GitFacts` as it is — as the menu opens and each time it stands again, never by a binding. `aim` is a remote the
    /// reader named on its own, which the TAG card acts on (empty for its default). `branchAt` is where the BRANCH
    /// card's branch points: the graph row's menu stands on the row's commit, which the branch may have moved off
    /// (`CommitMenuState.aimedAt`). `worktreePath` is the WORKTREES row the menu was opened on, whose worktree the
    /// WORKTREE card stands on; left out, the one the row's lead names (`RowLead.worktree`).
    ///
    /// One answer the menu and its cards read by name:
    /// - `lead`: the row and where it leads (`NavSectionModel.leadOf` — `path`, `called`, `home`, `stoppedBy`);
    /// - `menu`: core's answer for every row and card (`GitFacts.refMenu` — the making rows in `making`);
    /// - `card`: the record the row's card stands on — `branchUpstream` on a branch's or a remote branch's row,
    ///   `tagMenu` on a tag's, `undefined` on any other;
    /// - `oid`, `open`, `push`, `merged`: the row's commit, whether the tab is open, and the BRANCH card's push row
    ///   (`NavSectionModel.branchPush`) and plain delete off the drawn rows (`GraphModel.branchDeleteMerged`);
    /// - `tagAt`: the commit the tag here points at now (`RefTagMenu`);
    /// - `worktree`, `worktreeHere`, `place`: the WORKTREE card's worktree, whether the tab stands in it, and where the
    ///   row's own branch would go out (`NavSectionModel.newWorktreeFor`; `undefined` with no row, or before the
    ///   listing names the repository's own worktree).
    function stand(kind, full, oidHex, tree, aim, branchAt, worktreePath) {
        const local = refMenuFacts.remotesModel.landsOn(kind, full)
        const lead = refMenuFacts.worktreesModel.leadOf(kind, full, local)
        const card = kind === "branch" || kind === "remote" ? refMenuFacts.branchesModel.branchUpstream(kind, full)
                   : kind === "tag" ? refMenuFacts.tagsModel.tagMenu(full, refMenuFacts.repoTab.defaultRemote, aim)
                   : undefined
        const open = refMenuFacts.repoTab.state === "open"
        const menu = GitFacts.refMenu(oidHex, open, tree, lead, card,
                                      local !== "" && refMenuFacts.branchesModel.oidOfName(local) !== "")
        const worktreeAt = worktreePath !== undefined ? worktreePath : lead.worktree
        return {
            "lead": lead,
            "menu": menu,
            "card": card,
            "oid": oidHex,
            "open": open,
            "push": kind === "branch"
                    ? refMenuFacts.branchesModel.branchPush(full, refMenuFacts.repoTab.defaultRemote, open, tree)
                    : undefined,
            "merged": !open || kind !== "branch" ? ""
                      : refMenuFacts.graphModel.branchDeleteMerged(branchAt, card.upstreamOid, tree.headOid),
            "tagAt": kind === "tag" ? refMenuFacts.tagsModel.oidOfName(full) : "",
            "worktree": worktreeAt === "" ? undefined : refMenuFacts.worktreesModel.worktreeFacts(worktreeAt),
            "worktreeHere": GitFacts.samePath(worktreeAt, refMenuFacts.repoTab.repoPath),
            // Looked at here, as the menu opens and each time it stands again: the folder may have filled since.
            "place": menu.making.checkout === "" ? undefined : refMenuFacts.worktreesModel.newWorktreeFor(local)
        }
    }
}
