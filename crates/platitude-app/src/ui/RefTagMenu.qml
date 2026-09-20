import QtQuick
import platitude.ui

// Everything a tag's name answers for, behind the tag's own mark: made here, sent, and taken away from either side
// (デザイン規約 §メニュー の入れ子).
//
// **One card, two entrances**, the same as the branch's (`RefBranchMenu`): the graph row's menu and the sidebar's ref
// menu both carry this instance, and both hand it what it stands on — `standOn` freezes it as the menu opens, the
// way every other menu freezes what it shows (デザイン規約 §メニュー).
//
// **The card reads no model and runs no write.** What a state may offer is core's rule
// (`offers::ref_menu`), and the menu carrying this card is where the models to ask it with are, so that is where it
// is asked (`RefRowMenu.tagFacts` / `CommitMenuState.tagFacts`) — data down, requests up, as every other part of this
// panel is wired (規約 §コンポーネント配線規約). What is decided here is what the rows do with the answer: which of
// them has a seat, which of them greys, and which of the push row's two forms is drawn. That leaves the card
// standing on plain values, which is what lets `tests/qml/tst_tagcard.qml` drive the real one.
AppMenu {
    id: tagMenu

    /// Whether a name can be made on the commit this card stands on. **Handed in**: the two menus that carry the
    /// card ask it of different rules — a graph row asks the commit's, a sidebar row the ref's — and
    /// `Create tag here…` is a row about the commit.
    property bool canBranchHere: false

    /// The tag this card is about, frozen by `standOn`. `kind` is `tag` on a row that names one; anything else leaves
    /// the card holding `Create tag here…` alone, and `stash` takes the card off the menu.
    readonly property alias kind: state.kind
    readonly property alias refId: state.refId

    /// The box a name is typed into, which the page owns.
    signal tagHereRequested(string oidHex)
    /// The three things this card asks for, in the words the write takes. **Requests and not writes**: the menu
    /// carrying the card holds the tab, so the card says what was pressed and that one owner runs it — the same way
    /// every other row of these menus reports (規約 §コンポーネント配線規約).
    signal pushTagRequested(string remote, string tag, string lease)
    signal deleteTagRequested(string tag)
    /// `onlyThere` is this row's own reading: a name only the remote had leaves the sidebar with it, one held here
    /// keeps its row and loses the badge, and the write cannot tell the two apart from a remote and a name
    /// (デザイン規約 §消す操作は先に画面から消す).
    signal deleteRemoteTagRequested(string remote, string tag, bool onlyThere)
    signal deleteTagEverywhereRequested(string tag, string remote)

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

    /// Opens on that tag, if the row names one, off answers already in hand. `oidHex` is the commit the card stands
    /// on — the one a name would be made at — which is the row's own commit whether or not a tag is on it.
    ///
    /// `facts` is what the menu carrying this card read for it: `pushRemote` where this repository's pushes go,
    /// `tagDriftOid` where a remote was last heard to have this name when that is somewhere else, `tagOnlyThere`
    /// whether the name stands over there and not here, and `offers` the words core answered with
    /// (`GitFacts.refMenuOffers` — `offers::RefMenuOffers::words`). **Every one of them settles before this runs**,
    /// so the push row's whole shape is decided by the time the card is on screen.
    function standOn(kind, full, oidHex, facts) {
        state.kind = kind
        state.refId = full
        state.refOid = oidHex
        state.pushRemote = facts.pushRemote
        state.tagDriftOid = kind === "tag" ? facts.tagDriftOid : ""
        if (kind !== "tag") {
            state.canPushTag = false
            state.canDelete = false
            state.canDeleteRemoteTag = false
            state.canDeleteTagEverywhere = false
            state.tagOnlyThere = false
            return
        }
        state.tagOnlyThere = facts.tagOnlyThere
        // Which sides of the name exist is what tells the three delete rows apart, and core has already said it in
        // words (offers::ref_menu). What is read off them here is which row has a seat.
        const offers = facts.offers.split(" ")
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
        // Off the length the press under way was given, so the chip cannot change command under a hand that is
        // already on the row (`AppMenuItem.armedMs`, デザイン規約 §長押し).
        code: refPushTagItem.armedMs <= 0 ? "push" : "push --force"
        //: Follows the `push` chip: "push to origin".
        text: qsTr("to %1").arg(state.pushRemote)
        offered: state.canPushTag
        holdMs: state.tagDriftOid === "" ? 0 : Metrics.holdMs
        // Reaching past this machine is the warning tone, as it is on the remote-branch delete (デザイン規約 §状態).
        holdTone: Theme.warning
        onTriggered: tagMenu.pushTagRequested(state.pushRemote, state.refId, "")
        onHeld: {
            tagMenu.dismiss()
            tagMenu.pushTagRequested(state.pushRemote, state.refId, state.tagDriftOid)
        }
    }
    AppMenuSeparator {}
    // The tag's own three deletes, one per side its name stands on (デザイン規約 §左メニューの所作 の削除の表).
    //
    // **Assembled row by row.** The branch's fixed table stays and greys out because the current branch's menu
    // would otherwise open empty; a tag always has something to press, so its rows follow the ordinary rule and the
    // ones with nothing to name are gone (デザイン規約 §メニュー). What decides that is the sides the name stands on:
    // the qualified `--delete` git needs does not fail on a name the remote has not got (measured), so a row
    // offered on a hunch would report success for having done nothing.
    AppMenuItem {
        id: refTagDeleteItem
        code: "tag --delete"
        // The name is data — the delete rows' one exception to saying nothing twice, so that what is about to go
        // is readable under the hand during the hold (デザイン規約 §メニュー).
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
        text: state.refId
        growsForText: false
        // The drifted reading keeps its seat and greys: there **is** a name over there, and this row is the only place
        // that can say where it is standing (デザイン規約 §左メニューの所作 の削除の表). A side that does not exist at
        // all is gone from the card — the assembled rule this card otherwise follows.
        offered: state.kind === "tag" && (state.canDeleteRemoteTag || state.tagDriftOid !== "")
        // Drift is the only thing that can leave this row standing and unpressable: everything else that takes the
        // offer away takes the seat with it.
        blockedReason: state.canDeleteRemoteTag ? "" : Words.remoteOnAnotherCommit
        holdMs: Metrics.holdMs
        // Reaching past this machine is warning — what goes is a name over there, and whatever it marked stays
        // wherever it is (デザイン規約 §状態).
        holdTone: Theme.warning
        onHeld: {
            tagMenu.dismiss()
            // A name only the remote had leaves the sidebar with it; one held here keeps its row and loses the badge,
            // which the read after the write brings back (デザイン規約 §消す操作は先に画面から消す). Which of the two
            // this row is, is the row's own reading — the write cannot tell from a remote and a name.
            tagMenu.deleteRemoteTagRequested(state.pushRemote, state.refId, state.tagOnlyThere)
        }
    }
    // A composite of two commands is no one command, so the row says it in words — the same row the branch table
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
            tagMenu.deleteTagEverywhereRequested(state.refId, state.pushRemote)
        }
    }
}
