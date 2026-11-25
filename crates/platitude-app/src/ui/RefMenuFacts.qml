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
    /// What a branch's or a tag's card stands on (`cardOf`), a branch's push, and whether the row's local branch is
    /// there.
    required property NavSectionModel branchesModel
    /// The local branch the row lands on (`landsOn` — the one entrance for it).
    required property NavSectionModel remotesModel
    /// Where the tag here points (`tagAt`).
    required property NavSectionModel tagsModel

    /// Reads the row `full` of `kind` on the commit `oidHex` off `tree` — the reading the menu stands on, handed to
    /// `GitFacts` as it is — as the menu opens and each time it stands again, never by a binding. `aim` is a remote the
    /// reader named on its own, which the TAG card acts on (empty for its default). `branchAt` is where the BRANCH
    /// card's branch points: the graph row's menu stands on the row's commit, which the branch may have moved off
    /// (`CommitMenuState.aimedAt`). `worktreePath` is the WORKTREES row the menu was opened on, whose worktree the
    /// WORKTREE card stands on; left out, the one the worktrees section names for the row (`RowLead.card`).
    ///
    /// One answer the menu and its cards read by name:
    /// - `lead`: the row and where it leads (`NavSectionModel.leadOf` — `path`, `called`, `home`, `stoppedBy`), and
    ///   the WORKTREE card's worktree (`card`), whether the tab stands in it and where the row's own branch would go
    ///   out (`place`);
    /// - `menu`: core's answer for every row and card (`GitFacts.refMenu` — the making rows in `making`);
    /// - `card`: the record the row's card stands on, the kind picking it in core (`NavSectionModel.cardOf`) — a
    ///   branch's upstream on a branch's or a remote branch's row, a tag's menu on a tag's, `undefined` on any other;
    /// - `oid`, `open`, `push`, `merged`: the row's commit, whether the tab is open, and the BRANCH card's push row
    ///   (`NavSectionModel.branchPushFacts`) and plain delete off the drawn rows (`GraphModel.branchDeleteMerged`);
    /// - `tagAt`: the commit the tag here points at now (`RefTagMenu`).
    function stand(kind, full, oidHex, tree, aim, branchAt, worktreePath) {
        const lead = refMenuFacts.leadOf(kind, full, worktreePath === undefined ? "" : worktreePath)
        const card = refMenuFacts.branchesModel.cardOf(kind, full, refMenuFacts.repoTab.defaultRemote, aim)
        const open = refMenuFacts.repoTab.state === "open"
        const menu = GitFacts.refMenu(oidHex, open, tree, lead, card, refMenuFacts.landsListed(lead))
        return {
            "lead": lead,
            "menu": menu,
            "card": card,
            "oid": oidHex,
            "open": open,
            "push": kind === "branch"
                    ? refMenuFacts.branchesModel.branchPushFacts(full, refMenuFacts.repoTab.defaultRemote, open, tree)
                    : undefined,
            "merged": !open || kind !== "branch" ? ""
                      : refMenuFacts.graphModel.branchDeleteMerged(branchAt, card.upstreamOid, tree.headOid),
            "tagAt": kind === "tag" ? refMenuFacts.tagsModel.oidOfName(full) : ""
        }
    }

    /// The row `full` of `kind` and where it leads (`NavSectionModel.leadOf`), off the local branch it lands on
    /// (`landsOn` — the one entrance for it); `opened` the WORKTREES row a menu was opened on, empty for none. What a
    /// menu stands on and a press on a ref weighs (`RepoPage.switchToRef`) alike.
    function leadOf(kind, full, opened) {
        return refMenuFacts.worktreesModel.leadOf(kind, full, refMenuFacts.remotesModel.landsOn(kind, full), opened)
    }
    /// Whether the local branch that lead lands on is there — a remote row whose local one is takes its way through
    /// that row (`GitFacts.refMenu` / `switchMove`).
    function landsListed(lead) {
        return lead.local !== "" && refMenuFacts.branchesModel.oidOfName(lead.local) !== ""
    }
}
