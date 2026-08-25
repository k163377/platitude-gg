import QtQuick
import platitude
import platitude.ui

// Everything a tag's name answers for, behind the tag's own mark: made here, sent, and taken away from either side
// (デザイン規約 §メニュー の入れ子).
//
// Its own file for length alone — `RefRowMenu` had reached the ceiling (structure.md §上限). Every answer these rows
// read is decided as the menu opens and handed down from there; nothing here asks the repository anything.
AppMenu {
    id: tagMenu

    required property RepoTab repoTab
    /// The row the menu above stands on, and what it offers — the same values `RefRowMenu` froze as it opened.
    required property string kind
    required property string refId
    required property string refOid
    required property string pushRemote
    /// Where the remote was last heard to have this name, when that is somewhere else. Empty for the plain push.
    required property string tagDriftOid
    required property bool canBranchHere
    required property bool canPushTag
    required property bool canDelete
    required property bool canDeleteRemoteTag
    required property bool canDeleteTagEverywhere

    /// The box a name is typed into, which the page owns.
    signal tagHereRequested(string oidHex)
    /// The row taken off the list ahead of git's answer (デザイン規約 §消す操作は先に画面から消す).
    signal deleting(string kind, string id)
    /// The card above, which a press here has to take down along with this one.
    signal closeRequested()

    /// The automation's handles into these rows, passed on through `RefRowMenu` (app-ui.md).
    readonly property alias tagHereItem: refTagHereItem
    readonly property alias pushTagItem: refPushTagItem
    readonly property alias deleteTagItem: refTagDeleteItem
    readonly property alias deleteRemoteTagItem: refRemoteTagDeleteItem
    readonly property alias deleteTagBothItem: refBothTagDeleteItem

    /// Both cards go at once: this one, and the one it hangs off.
    function dismiss() {
        tagMenu.close()
        tagMenu.closeRequested()
    }

    titleKind: "tag"
    titleTint: Theme.refTag
    title: qsTr("TAG")
    // A stash is nobody's history, so there is no tag to make on it and no card to open.
    applies: tagMenu.kind !== "stash"

    // The mark left where this row stands. Same words as the commit menu's row (デザイン規約 §メニュー: 入口が違っても
    // 同じ操作は同じ文), and offered on the same answer as the branch card's own first row: every row that names a
    // commit takes one, a stash — nobody's history — takes neither.
    AppMenuItem {
        id: refTagHereItem
        text: qsTr("Create tag here…")
        offered: tagMenu.canBranchHere
        onTriggered: tagMenu.tagHereRequested(tagMenu.refOid)
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
        code: tagMenu.tagDriftOid === "" ? "push" : "push --force"
        //: Follows the `push` chip: "push to origin".
        text: qsTr("to %1").arg(tagMenu.pushRemote)
        offered: tagMenu.canPushTag
        holdMs: tagMenu.tagDriftOid === "" ? 0 : Metrics.holdMs
        // Reaching past this machine is the warning tone, as it is on the remote-branch delete (デザイン規約 §状態).
        holdTone: Theme.warning
        onTriggered: tagMenu.repoTab.pushTag(tagMenu.pushRemote, tagMenu.refId, "")
        onHeld: {
            tagMenu.dismiss()
            tagMenu.repoTab.pushTag(tagMenu.pushRemote, tagMenu.refId, tagMenu.tagDriftOid)
        }
    }
    AppMenuSeparator {}
    // The tag's own three deletes, one per side its name stands on (デザイン規約 §左メニューの所作 の削除の表).
    //
    // **Assembled, not the branch's fixed table.** That table stays and greys out because the current branch's menu
    // would otherwise open empty; a tag always has something to press, so its rows follow the ordinary rule and the
    // ones with nothing to name are gone (デザイン規約 §メニュー). What decides that is the sides the name stands on, not a
    // guess: the qualified `--delete` git needs does not fail on a name the remote has not got (実測), so a row
    // offered on a hunch would report success for having done nothing.
    AppMenuItem {
        id: refTagDeleteItem
        code: "tag --delete"
        // The name is data, not sentence — the delete rows' one exception to saying nothing twice, so that what is
        // about to go is readable under the hand during the hold (デザイン規約 §メニュー).
        text: tagMenu.refId
        growsForText: false
        offered: tagMenu.kind === "tag" && tagMenu.canDelete
        holdMs: Metrics.holdMs
        holdTone: Theme.danger
        onHeld: {
            tagMenu.dismiss()
            tagMenu.deleting("tag", tagMenu.refId)
            tagMenu.repoTab.deleteTag(tagMenu.refId)
        }
    }
    AppMenuItem {
        id: refRemoteTagDeleteItem
        code: "push --delete"
        text: tagMenu.refId
        growsForText: false
        offered: tagMenu.kind === "tag" && tagMenu.canDeleteRemoteTag
        holdMs: Metrics.holdMs
        // Reaching past this machine is warning, not danger — what goes is a name over there, and whatever it marked
        // stays wherever it is (デザイン規約 §状態).
        holdTone: Theme.warning
        onHeld: {
            tagMenu.dismiss()
            // A name only the remote had leaves the sidebar with it; one held here keeps its row and loses the badge,
            // which the read after the write brings back (デザイン規約 §消す操作は先に画面から消す).
            if (!tagMenu.canDeleteTagEverywhere)
                tagMenu.deleting("tag", tagMenu.refId)
            tagMenu.repoTab.deleteRemoteTag(tagMenu.pushRemote, tagMenu.refId)
        }
    }
    // A composite of two commands is no one command, so words rather than a chip — the same row the branch table
    // carries, for the same reason (§git 用語のコード表記 の 1:1 規則).
    AppMenuItem {
        id: refBothTagDeleteItem
        text: qsTr("Delete both")
        offered: tagMenu.kind === "tag" && tagMenu.canDeleteTagEverywhere
        holdMs: Metrics.holdMs
        // The local half of a tag throws nothing away that the commit is not still holding — git keeps the object and
        // only the name goes — so this pair never reaches the danger the branch's `-D` does.
        holdTone: Theme.warning
        onHeld: {
            tagMenu.dismiss()
            tagMenu.deleting("tag", tagMenu.refId)
            tagMenu.repoTab.deleteTagEverywhere(tagMenu.refId, tagMenu.pushRemote)
        }
    }
}
