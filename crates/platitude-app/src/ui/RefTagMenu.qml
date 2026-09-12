import QtQuick
import platitude
import platitude.ui

// Everything a tag's name answers for, behind the tag's own mark: made here, sent, and taken away from either side
// (デザイン規約 §メニュー の入れ子).
//
// **One card, two entrances**, the same as the branch's (`RefBranchMenu`): the graph row's menu and the sidebar's ref
// menu both carry this instance, and it works out its own answers rather than being handed them — `offerOn` freezes
// them as the menu opens, the way every other menu freezes what it shows (デザイン規約 §メニュー).
AppMenu {
    id: tagMenu

    required property RepoTab repoTab
    required property WorkTreeModel workTree
    /// Where a remote was last heard to have a tag that is here as well (`remoteTagDrift`), and which sides of a name
    /// exist (`tagSides`) — the readings live in this section alone.
    required property NavSectionModel tagsModel

    /// Whether a name can be made on the commit this card stands on. **Handed in rather than worked out here**: the
    /// two menus that carry the card ask it of different rules — a graph row asks the commit's, a sidebar row the
    /// ref's — and `Create tag here…` is a row about the commit, not about the tag.
    property bool canBranchHere: false

    /// The tag this card is about, frozen by `offerOn`. `kind` is `tag` on a row that names one; anything else leaves
    /// the card holding `Create tag here…` alone, and `stash` takes the card off the menu.
    readonly property alias kind: state.kind
    readonly property alias refId: state.refId

    /// The box a name is typed into, which the page owns.
    signal tagHereRequested(string oidHex)

    /// The automation's handles into these rows, passed on through `RefRowMenu` (app-ui.md).
    readonly property alias tagHereItem: refTagHereItem
    readonly property alias pushTagItem: refPushTagItem
    readonly property alias deleteTagItem: refTagDeleteItem
    readonly property alias deleteRemoteTagItem: refRemoteTagDeleteItem
    readonly property alias deleteTagBothItem: refBothTagDeleteItem

    /// Everything this card reads, worked out once as the menu opens and left alone while it stands.
    QtObject {
        id: state
        property string kind: ""
        property string refId: ""
        property string refOid: ""
        property string pushRemote: ""
        /// Where the remote was last heard to have this name, when that is somewhere else. Empty for the plain push.
        property string tagDriftOid: ""
        property bool canPushTag: false
        property bool canDelete: false
        property bool canDeleteRemoteTag: false
        property bool canDeleteTagEverywhere: false
        /// Whether the name is only over there, so the sidebar's row goes with the remote copy rather than keeping
        /// its seat and losing the badge (`refRemoteTagDeleteItem`). Read as the menu opens with the rest of what its
        /// rows stand on (app-ui.md §メニューの行が読む状態).
        property bool tagOnlyThere: false
    }

    /// Opens on that tag, if the row names one. `oidHex` is the commit the card stands on — the one a name would be
    /// made at — which is the row's own commit whether or not a tag is on it.
    function offerOn(kind, full, oidHex) {
        state.kind = kind
        state.refId = full
        state.refOid = oidHex
        // Where a push would go, and what that remote already has under this name. Both settle before the offers are
        // asked for, so the push row's whole shape is decided by the time the card is on screen.
        state.pushRemote = tagMenu.repoTab.defaultRemote
        state.tagDriftOid = kind === "tag"
            ? tagMenu.tagsModel.remoteTagDrift(full, state.pushRemote) : ""
        if (kind !== "tag") {
            state.canPushTag = false
            state.canDelete = false
            state.canDeleteRemoteTag = false
            state.canDeleteTagEverywhere = false
            state.tagOnlyThere = false
            return
        }
        state.tagOnlyThere = tagMenu.tagsModel.tagSides(full) === "remote"
        // Which sides of the name exist, which is what tells the three delete rows apart. The lookup is the model's;
        // what the rows may offer on it is core's rule (offers::ref_menu).
        const offers = GitFacts.refMenuOffers(
            kind, full, oidHex,
            // Nothing running, while the doors are held: the hold already answers for every row of this card
            // (`RefRowMenu.askBusy` — the same rule at the other entrance).
            tagMenu.repoTab.state === "open",
            tagMenu.heldReason !== "" ? 0 : tagMenu.repoTab.busyCount,
            tagMenu.workTree.branch, tagMenu.workTree.detached,
            tagMenu.workTree.opText, tagMenu.workTree.conflictCount,
            // The reading over there standing on another commit is what takes the two rows that reach it out — the
            // same answer that shapes the push row above, read the other way (デザイン規約 §左メニューの所作 の削除の表).
            "", "", state.tagDriftOid !== "",
            state.pushRemote, tagMenu.tagsModel.tagSides(full)).split(" ")
        state.canPushTag = offers.includes("push-tag")
        state.canDelete = offers.includes("delete")
        state.canDeleteRemoteTag = offers.includes("delete-remote-tag")
        state.canDeleteTagEverywhere = offers.includes("delete-tag-everywhere")
    }

    // The rows that run something take the whole menu down with `dismiss()` — Qt's own, which walks up from this
    // card through the menu it hangs off (app-ui.md §メニューを閉じるのは自分).

    titleKind: "tag"
    titleTint: Theme.refTag
    title: qsTr("TAG")
    // A stash is nobody's history, so there is no tag to make on it and no card to open.
    applies: state.kind !== "stash"

    // The mark left where this row stands. Same words as the commit menu's row (デザイン規約 §メニュー: 入口が違っても
    // 同じ操作は同じ文), and offered on the same answer as the branch card's own first row: every row that names a
    // commit takes one, a stash — nobody's history — takes neither.
    AppMenuItem {
        id: refTagHereItem
        text: qsTr("Create tag here…")
        offered: tagMenu.canBranchHere
        onTriggered: tagMenu.tagHereRequested(state.refOid)
    }
    // A tag sent to where this repository pushes — the only row in this menu that leaves the machine of its own
    // accord. The branches' own push is the toolbar's, which is where the counts that decide how hard it may push
    // live (デザイン規約 §リモートへ送る).
    //
    // **Two forms, chosen as the menu opens.** A name the remote already has on another commit is refused outright by
    // a plain push, so that case comes up as the leased overwrite instead: warning-coloured, held, and pinned to the
    // commit that was being shown (§相手の履歴を置き換える — the same reason the toolbar's `push` and `push -f` are never
    // both live). The plain form is an ordinary row: it adds a name over there and takes nothing away. The long
    // spelling because a menu row is measured against the widest row (§git 用語のコード表記).
    AppMenuItem {
        id: refPushTagItem
        code: state.tagDriftOid === "" ? "push" : "push --force"
        //: Follows the `push` chip: "push to origin".
        text: qsTr("to %1").arg(state.pushRemote)
        offered: state.canPushTag
        holdMs: state.tagDriftOid === "" ? 0 : Metrics.holdMs
        // Reaching past this machine is the warning tone, as it is on the remote-branch delete (デザイン規約 §状態).
        holdTone: Theme.warning
        onTriggered: tagMenu.repoTab.pushTag(state.pushRemote, state.refId, "")
        onHeld: {
            tagMenu.dismiss()
            tagMenu.repoTab.pushTag(state.pushRemote, state.refId, state.tagDriftOid)
        }
    }
    AppMenuSeparator {}
    // The tag's own three deletes, one per side its name stands on (デザイン規約 §左メニューの所作 の削除の表).
    //
    // **Assembled, not the branch's fixed table.** That table stays and greys out because the current branch's menu
    // would otherwise open empty; a tag always has something to press, so its rows follow the ordinary rule and the
    // ones with nothing to name are gone (デザイン規約 §メニュー). What decides that is the sides the name stands on, not a
    // guess: the qualified `--delete` git needs does not fail on a name the remote has not got (measured), so a row
    // offered on a hunch would report success for having done nothing.
    AppMenuItem {
        id: refTagDeleteItem
        code: "tag --delete"
        // The name is data, not sentence — the delete rows' one exception to saying nothing twice, so that what is
        // about to go is readable under the hand during the hold (デザイン規約 §メニュー).
        text: state.refId
        growsForText: false
        offered: state.kind === "tag" && state.canDelete
        holdMs: Metrics.holdMs
        holdTone: Theme.danger
        onHeld: {
            tagMenu.dismiss()
            tagMenu.repoTab.deleteTag(state.refId)
        }
    }
    AppMenuItem {
        id: refRemoteTagDeleteItem
        code: "push --delete"
        text: state.refId
        growsForText: false
        // The drifted reading keeps its seat and greys: there **is** a name over there, and this row is the only place
        // that can say where it is standing (デザイン規約 §左メニューの所作 の削除の表). A side that does not exist at
        // all still takes no seat — that is the assembled rule this card otherwise follows.
        offered: state.kind === "tag" && (state.canDeleteRemoteTag || state.tagDriftOid !== "")
        // Drift is the only thing that can leave this row standing and unpressable: everything else that takes the
        // offer away takes the seat with it.
        blockedReason: state.canDeleteRemoteTag ? "" : Words.remoteOnAnotherCommit
        holdMs: Metrics.holdMs
        // Reaching past this machine is warning, not danger — what goes is a name over there, and whatever it marked
        // stays wherever it is (デザイン規約 §状態).
        holdTone: Theme.warning
        onHeld: {
            tagMenu.dismiss()
            // A name only the remote had leaves the sidebar with it; one held here keeps its row and loses the badge,
            // which the read after the write brings back (デザイン規約 §消す操作は先に画面から消す). Which of the two
            // this row is, is the row's own reading — the write cannot tell from a remote and a name.
            tagMenu.repoTab.deleteRemoteTag(state.pushRemote, state.refId, state.tagOnlyThere)
        }
    }
    // A composite of two commands is no one command, so words rather than a chip — the same row the branch table
    // carries, for the same reason (§git 用語のコード表記 の 1:1 規則).
    AppMenuItem {
        id: refBothTagDeleteItem
        text: qsTr("Delete both")
        offered: state.kind === "tag" && (state.canDeleteTagEverywhere || state.tagDriftOid !== "")
        blockedReason: state.canDeleteTagEverywhere ? "" : Words.remoteOnAnotherCommit
        holdMs: Metrics.holdMs
        // The local half of a tag throws nothing away that the commit is not still holding — git keeps the object and
        // only the name goes — so this pair never reaches the danger the branch's `-D` does.
        holdTone: Theme.warning
        onHeld: {
            tagMenu.dismiss()
            tagMenu.repoTab.deleteTagEverywhere(state.refId, state.pushRemote)
        }
    }
}
