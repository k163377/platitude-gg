import QtQuick
import platitude.ui

// Everything a tag's name answers for, behind the tag's mark: made here, sent, deleted on either side
// (デザイン規約 §メニュー の入れ子). One card, two entrances like `RefBranchMenu`: the graph row's menu and the
// sidebar's ref menu both carry it and freeze it with `standOn`.
//
// It reads no model and runs no write: the carrying menu asks core (`RefRowMenu.tagFacts` /
// `CommitMenuState.tagFacts`) and runs the requests. Decided here is only what the rows do with the answer — which
// has a seat, which greys, which push form is drawn — so `tests/qml/tst_tagcard.qml` drives the real card on values.
AppMenu {
    id: tagMenu

    /// Whether a name can be made on this commit. Handed in: the graph row asks the commit's rule, the sidebar row
    /// the ref's.
    property bool canBranchHere: false

    /// The tag this card is about, frozen by `standOn`. `kind` is `tag` on a row that names one; anything else leaves
    /// the card holding `Create tag here…` alone, and `stash` takes the card off the menu.
    readonly property alias kind: state.kind
    readonly property alias refId: state.refId

    /// Opens the name box, which the page owns.
    signal tagHereRequested(string oidHex)
    signal pushTagRequested(string remote, string tag, string lease)
    signal deleteTagRequested(string tag)
    /// `onlyThere`: the name was only on the remote, so its sidebar row goes too instead of losing the badge — the
    /// write cannot tell from a remote and a name (デザイン規約 §消す操作は先に画面から消す).
    signal deleteRemoteTagRequested(string remote, string tag, bool onlyThere)
    signal deleteTagEverywhereRequested(string tag, string remote)

    /// The automation's handles into these rows, passed on through `RefRowMenu`.
    readonly property alias tagHereItem: refTagHereItem
    readonly property alias pushTagItem: refPushTagItem
    readonly property alias deleteTagItem: refTagDeleteItem
    readonly property alias deleteRemoteTagItem: refRemoteTagDeleteItem
    readonly property alias deleteTagBothItem: refBothTagDeleteItem
    /// Where this card's deletes reach, for a run's report (`AutoActRefVerbs.reachWords`).
    readonly property alias reach: state.reach

    /// Everything this card reads, worked out once as the menu opens and left alone while it stands.
    QtObject {
        id: state
        property string kind: ""
        property string refId: ""
        property string refOid: ""
        property string pushRemote: ""
        /// Where the remote was last heard to have this name, when that is somewhere else. Empty for the plain push.
        property string tagDriftOid: ""
        /// The remote the rows deleting over there reach — empty where none can be named — and why they stand greyed:
        /// `drift` (that remote has the name elsewhere than here) / `unnamed` (several carry it, `tagCarriers`).
        property string reach: ""
        property string heldBack: ""
        property string tagCarriers: ""
        property bool tagHere: false
        property bool canPushTag: false
        property bool canDelete: false
        property bool canDeleteRemoteTag: false
        property bool canDeleteTagEverywhere: false
        /// Whether deleting from `reach` leaves the name nowhere (`deleteRemoteTagRequested`).
        property bool tagOnlyThere: false
        /// Why those rows stand greyed, in words (`Words.differsFrom` / `Words.severalRemotesHave`).
        readonly property string heldWhy: state.heldBack === "unnamed" ? Words.severalRemotesHave(state.tagCarriers)
                                        : state.heldBack === "drift" ? Words.differsFrom(state.reach) : ""
    }

    /// Stands the card on that row, off answers already in hand. `oidHex` is the row's commit, where a name would be
    /// made, tag or not. `facts` is what the carrying menu read (`NavSectionModel.tagMenu`): `pushRemote` (where
    /// pushes go), `tagDriftOid` (where that remote last had this name, when elsewhere), `tagReach` / `tagHeldBack` /
    /// `tagCarriers` (where the deletes reach and why they grey), `tagOnlyThere`, `tagHere`, and `offers`
    /// (`GitFacts.refMenuOffers` — `offers::RefMenuOffers::words`). All of it is in hand before the card shows, so
    /// the push row's shape is fixed.
    function standOn(kind, full, oidHex, facts) {
        state.kind = kind
        state.refId = full
        state.refOid = oidHex
        state.pushRemote = facts.pushRemote
        const tagged = kind === "tag"
        state.tagDriftOid = tagged ? facts.tagDriftOid : ""
        state.reach = tagged ? facts.tagReach : ""
        state.heldBack = tagged ? facts.tagHeldBack : ""
        state.tagCarriers = tagged ? facts.tagCarriers : ""
        state.tagHere = tagged && facts.tagHere
        if (!tagged) {
            state.canPushTag = false
            state.canDelete = false
            state.canDeleteRemoteTag = false
            state.canDeleteTagEverywhere = false
            state.tagOnlyThere = false
            return
        }
        state.tagOnlyThere = facts.tagOnlyThere
        // Which delete rows have a seat is core's answer (offers::ref_menu).
        const offers = facts.offers
        state.canPushTag = offers.includes("push-tag")
        state.canDelete = offers.includes("delete")
        state.canDeleteRemoteTag = offers.includes("delete-remote-tag")
        state.canDeleteTagEverywhere = offers.includes("delete-tag-everywhere")
    }

    // `dismiss()` folds every level (rules-refs/app-ui.md「メニューを閉じるのは自分」).

    titleKind: "tag"
    titleTint: Theme.refTag
    title: qsTr("TAG")
    // A stash is nobody's history: no tag to make on it.
    applies: state.kind !== "stash"

    // Offered on the same answer as `Create branch here…` (デザイン規約 §タグを作る・送る).
    AppMenuItem {
        id: refTagHereItem
        text: qsTr("Create tag here…")
        offered: tagMenu.canBranchHere
        onTriggered: tagMenu.tagHereRequested(state.refOid)
    }
    // Two forms, fixed as the menu opens: a name the remote has on another commit is the held, leased `push --force`
    // (デザイン規約 §タグを作る・送る).
    AppMenuItem {
        id: refPushTagItem
        // Off the running press's length, so the chip cannot change under a hand already on the row
        // (`AppMenuItem.armedMs`, デザイン規約 §長押し).
        code: refPushTagItem.armedMs <= 0 ? "push" : "push --force"
        //: Follows the `push` chip: "push to origin".
        text: qsTr("to %1").arg(state.pushRemote)
        offered: state.canPushTag
        holdMs: state.tagDriftOid === "" ? 0 : Metrics.holdMs
        // Reaching past this machine is the warning tone (デザイン規約 §状態).
        holdTone: Theme.warning
        onTriggered: tagMenu.pushTagRequested(state.pushRemote, state.refId, "")
        onHeld: {
            tagMenu.dismiss()
            tagMenu.pushTagRequested(state.pushRemote, state.refId, state.tagDriftOid)
        }
    }
    AppMenuSeparator {}
    // The tag's three deletes, assembled from the sides its name stands on: git does not refuse deleting a name the
    // remote lacks (デザイン規約 §タグを作る・送る).
    AppMenuItem {
        id: refTagDeleteItem
        code: "tag --delete"
        // The name, readable under the hand during the hold (デザイン規約 §メニュー「例外は削除行だけ」).
        text: state.refId
        growsForText: false
        offered: state.kind === "tag" && state.canDelete
        holdMs: Metrics.holdMs
        holdTone: Theme.danger
        onHeld: {
            tagMenu.dismiss()
            tagMenu.deleteTagRequested(state.refId)
        }
    }
    AppMenuItem {
        id: refRemoteTagDeleteItem
        code: "push --delete"
        // The name and the remote it goes from: the deletes need not reach where the pushes go (`state.reach`).
        //: Follows the `push --delete` chip: "push --delete v1.5 from origin".
        text: state.reach !== "" ? qsTr("%1 from %2").arg(state.refId).arg(state.reach) : state.refId
        growsForText: false
        // A reading held back keeps its seat and greys; a side that does not exist is gone
        // (デザイン規約 §左メニューの所作 の削除の表).
        offered: state.kind === "tag" && (state.canDeleteRemoteTag || state.heldBack !== "")
        blockedReason: state.canDeleteRemoteTag ? "" : state.heldWhy
        holdMs: Metrics.holdMs
        // Warning: only a name goes, and the commit stays (デザイン規約 §タグを作る・送る).
        holdTone: Theme.warning
        onHeld: {
            tagMenu.dismiss()
            tagMenu.deleteRemoteTagRequested(state.reach, state.refId, state.tagOnlyThere)
        }
    }
    // Two commands, so words and no chip (§git 用語のコード表記 の 1:1 規則).
    AppMenuItem {
        id: refBothTagDeleteItem
        text: qsTr("Delete both")
        offered: state.kind === "tag" && (state.canDeleteTagEverywhere || (state.heldBack !== "" && state.tagHere))
        blockedReason: state.canDeleteTagEverywhere ? "" : state.heldWhy
        holdMs: Metrics.holdMs
        // Only names go — git keeps the object — so never the danger of the branch's `-D`.
        holdTone: Theme.warning
        onHeld: {
            tagMenu.dismiss()
            tagMenu.deleteTagEverywhereRequested(state.refId, state.reach)
        }
    }
}
