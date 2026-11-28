import QtQuick
import platitude.ui
import platitude.ui.words

// Everything a tag's name answers for, behind the tag's mark: made here, sent, deleted on either side
// (デザイン規約 §メニュー の入れ子). One card, two entrances like `RefBranchMenu`: the graph row's menu and the
// sidebar's ref menu both carry it and stand it with `standOn`.
//
// Nested, not on the first level: sending and deleting a tag are rare (user decision — ask before changing).
// `Create tag here…` lives here while `Create branch here…` stands on the first level (user decision): a branch is a
// place the reader moves on from, a tag a mark left behind. On every row but a stash's, `Create tag here…` alone where
// the row names no tag; the push and delete rows join only when the target is a tag (core's rows, `MenuLines`), and
// dividers part making, sending and deleting — the groups of core's seats (`menu::CardGroup`).
//
// It reads no model and runs no write: the carrying menu asks core through one part both entrances hold
// (`RefMenuFacts`) and runs the requests. Decided here is only what the rows do with the answer — which greys, which
// push form is drawn — so `tests/qml/tst_tagcard.qml` drives the real card on values.
AppMenu {
    id: tagMenu

    /// The tag this card is about, set by `standOn`. `kind` is `tag` on a row that names one; anything else leaves
    /// the card holding `Create tag here…` alone, and a stash's class has no TAG rows.
    readonly property alias kind: state.kind
    readonly property alias refId: state.refId

    /// Opens the name box, which the page owns.
    signal tagHereRequested(string oidHex)
    signal pushTagRequested(string remote, string tag, string expect)
    signal deleteTagRequested(string tag)
    /// `rowGoes`: the name was only on the remote, so its sidebar row goes too instead of losing the badge — the
    /// write cannot tell from a remote and a name (デザイン規約 §消す操作は先に画面から消す). `expect` on both remote
    /// deletes is the commit this card showed the tag on, which the delete is leased to (`remote::delete_remote_tag`):
    /// the rows are offered only where the remote's copy stands there too.
    signal deleteRemoteTagRequested(string remote, string tag, bool rowGoes, string expect)
    signal deleteTagEverywhereRequested(string remote, string tag, string expect)

    /// The automation's handles into these rows, passed on through `RefRowMenu`.
    readonly property alias tagHereItem: refTagHereItem
    readonly property alias pushTagItem: refPushTagItem
    readonly property alias deleteTagItem: refTagDeleteItem
    readonly property alias deleteRemoteTagItem: refRemoteTagDeleteItem
    readonly property alias deleteTagBothItem: refBothTagDeleteItem
    /// Where this card's deletes reach, for a run's report (`AutoActRefVerbs.reachWords`).
    readonly property alias reach: state.reach

    /// Everything this card reads, worked out by `standOn` alone: as the menu opens, and as it stands again.
    readonly property QtObject state: QtObject {
        id: state
        property string kind: ""
        property string refId: ""
        property string refOid: ""
        /// The commit the tag here points at now, the one a push sends: what the push row's presses lean on
        /// (`premise`). Empty where only a remote has the name.
        property string tagAt: ""
        property string pushRemote: ""
        /// Where the remote was last heard to have this name, when that is somewhere else. Empty for the plain push.
        property string tagDriftOid: ""
        /// The push row stood as the menu went up: the remote and the reading it showed hold through every stand
        /// after, one where it stopped applying for a while included (`standOn`).
        property bool pushShown: false
        /// The menu stood again on a reading where the push's remote or its reading of this name has moved from what
        /// the row showed: the row keeps that and greys asking for the menu to be opened again (`standOn`'s
        /// `again`).
        property bool pushMoved: false
        /// The remote the rows deleting over there reach — empty where none can be named, several carrying it
        /// (`tagCarriers`); why they grey is core's (`MenuLines`).
        property string reach: ""
        property string tagCarriers: ""
        /// The rows the card has, and why each is greyed, as core answered the stand (`MenuLines`).
        readonly property MenuLines lines: MenuLines {
            id: lines
        }
        /// Whether deleting from `reach` leaves the name nowhere (`deleteRemoteTagRequested`).
        property bool rowGoes: false
    }

    /// Why one of the two deletes that reach over there stands greyed, in words — the reading held back names the
    /// remote or the remotes that carry it (`Words.menuWhy`).
    readonly property var farNames: ({ "reach": state.reach, "carriers": state.tagCarriers })
    /// The remotes those words name, each behind the cloud (`AppMenuItem.reasonMarks`).
    function farMarks(row) {
        return row.why === "tag-unnamed" ? Words.listMarks(row.blockedReason, state.tagCarriers, "remote")
             : row.why === "tag-drifted" ? Words.listMarks(row.blockedReason, state.reach, "remote") : []
    }

    /// Stands the card on that row, off answers already in hand. `oidHex` is the row's commit, where a name would be
    /// made, tag or not. `facts` is what the carrying menu read (`RefMenuFacts.stand`): on a tag's row, `card` the
    /// tag's record (`NavSectionModel.tagMenu` — `pushRemote` where pushes go, `lease` where that remote last had this
    /// name when elsewhere, `reach` / `carriers` where the deletes reach, `rowGoes`),
    /// `tagAt` the commit the tag here points at now, and `menu` core's answer for the rows (`GitFacts.menuOn` —
    /// which there are, and why each greys). All of it is in hand before the card
    /// shows, so the push row's shape is settled as it opens. `again`: why the menu stands again with the card up
    /// (`MenuLines.standAgain`) — `follow`, on what moved while it stood, or `wait`, on what the command it waited for
    /// left —; empty as the menu is raised. Standing again, the push stays
    /// on the remote and the reading the menu went up with (デザイン規約 §メニュー「表示も押下も開いた時」): moved
    /// since — a fetch outside, or the one the menu waited for, brought another reading — the row keeps what it showed
    /// and greys (`pushMoved`), since the held overwrite would replace a commit it never showed.
    function standOn(kind, full, oidHex, facts, again) {
        const tagged = kind === "tag"
        const sameTag = (again === "follow" || again === "wait") && tagged && kind === state.kind
                        && full === state.refId
        // Latched as the menu went up, not read off the row as it is: a push that stopped applying for a while (the
        // tag deleted here) and came back on another reading would otherwise take that reading as its lease.
        const pushHeld = sameTag && state.pushShown
        const card = tagged ? facts.card : undefined
        const drift = tagged ? card.lease : ""
        const pushRemote = tagged ? card.pushRemote : ""
        const pushMoved = pushHeld
                          && (state.pushMoved || pushRemote !== state.pushRemote || drift !== state.tagDriftOid)
        lines.answer = facts.menu
        state.kind = kind
        state.refId = full
        state.refOid = oidHex
        state.tagAt = tagged ? facts.tagAt : ""
        state.pushMoved = pushMoved
        state.pushRemote = pushMoved ? state.pushRemote : pushRemote
        state.tagDriftOid = pushMoved ? state.tagDriftOid : drift
        state.reach = tagged ? card.reach : ""
        state.tagCarriers = tagged ? card.carriers : ""
        if (!tagged) {
            state.pushShown = false
            state.rowGoes = false
            return
        }
        state.rowGoes = card.rowGoes
        state.pushShown = sameTag ? state.pushShown : lines.has("push-tag")
    }

    // `dismiss()` folds every level (rules-refs/app-ui.md「メニューを閉じるのは自分」).

    titleKind: "tag"
    titleTint: Theme.refTag
    title: qsTr("TAG")
    word: "tag"

    // Offered on the same answer as `Create branch here…`, the same words at both entrances (§メニュー). Words and a
    // `…`, no chip: the press opens a box, and what git is finally spawned with depends on what is typed into it
    // (§git 用語のコード表記). Lightweight tags only (`tag::create`).
    AppMenuItem {
        id: refTagHereItem
        text: qsTr("Create tag here…")
        word: "tag-here"
        onTriggered: tagMenu.tagHereRequested(state.refOid)
    }
    // Two forms, settled as the menu opens and kept while that reading holds (§メニュー; `state.pushMoved`): a name the
    // remote has on another commit is the held, leased `push --force` (the flag long, as every menu row's), leased to
    // the oid on screen (§相手の履歴を置き換える).
    // Weighed off the last `ls-remote --tags` reading; with no reading the plain `push` — a remote holding it on the
    // same commit only answers that there is nothing to send — and git's refusal stands. The plain push is an ordinary
    // row, plain ink and one click: it adds a name and takes nothing; the hold and the warning are the lease's alone.
    // Where it goes is where this repository pushes (`remote.pushDefault` → `origin` → the first remote; no remote,
    // no row), except a remote the reader named — the open TAGS row's carrier line, a chip drawing one remote's
    // reading — which takes it (`RefsSnapshot::tag_menu`). Only tags and local branches have a send row: a branch's
    // strength comes from its own counts (`BranchPushRow`), and a remote branch row is a reading with nothing to send.
    AppMenuItem {
        id: refPushTagItem
        // Off the running press's length, so the chip cannot change under a hand already on the row
        // (`AppMenuItem.armedPresses`, デザイン規約 §回数押し).
        code: refPushTagItem.armedPresses <= 0 ? "push" : "push --force"
        //: Follows the `push` chip: "push to origin".
        refSentence: qsTr("to %1")
        refName: state.pushRemote
        // As the branch's push row: where it goes, behind its mark.
        refTint: Theme.textPrimary
        refMark: "remote"
        // Moved since the menu went up, the row stops applying and greys where it stands (`state.pushMoved`).
        word: "push-tag"
        leaseMoved: state.pushMoved
        holdPresses: state.tagDriftOid === "" ? 0 : Metrics.holdPressesWarned
        // Reaching past this machine is the warning tone (デザイン規約 §状態).
        holdTone: Theme.warning
        // What it sends and where, leased to what: the tag moved here part way (`git tag -f`) is not the push the
        // presses began.
        premise: [state.pushRemote, state.refId, state.tagAt, state.tagDriftOid].join("\n")
        onTriggered: tagMenu.pushTagRequested(state.pushRemote, state.refId, "")
        onHeld: {
            refPushTagItem.closeMenu()
            tagMenu.pushTagRequested(state.pushRemote, state.refId, state.tagDriftOid)
        }
    }
    // The tag's three deletes (rows and words: §左メニューの所作 の削除の表), assembled from the sides its name stands
    // on (`offers::TagSides`): tags have no namespace, so the copy here and a remote's share one bare name and one
    // left-menu row, and the row alone cannot say which delete has a target. git does not refuse deleting a name the
    // remote lacks (`remote::delete_remote_tag`), so whether a row stands is ours to decide: a remote row only with a
    // reading — one shown on a guess would report "did nothing" as a success.
    AppMenuItem {
        id: refTagDeleteItem
        code: "tag --delete"
        // The name, readable under the hand during the hold (デザイン規約 §メニュー「例外は削除行だけ」), behind its
        // kind's mark (デザイン規約 §ref の種別「名前の印」).
        nameMark: "tag"
        codeName: state.refId
        word: "delete-tag"
        holdPresses: Metrics.holdPressesWarned
        premise: state.refId + "\n" + state.refOid
        onHeld: {
            refTagDeleteItem.closeMenu()
            tagMenu.deleteTagRequested(state.refId)
        }
    }
    AppMenuItem {
        id: refRemoteTagDeleteItem
        code: "push --delete"
        // The name and the remote it goes from: the deletes need not reach where the pushes go (`state.reach`).
        //: Follows the `push --delete` chip: "push --delete v1.5 from origin".
        refSentence: state.reach !== "" ? qsTr("%1 from %2") : ""
        refName: state.refId
        refTint: Theme.textPrimary
        refMark: "tag"
        refName2: state.reach
        refMark2: "remote"
        // No remote to name: the tag alone, behind its mark.
        nameMark: state.reach !== "" ? "" : "tag"
        codeName: state.reach !== "" ? "" : state.refId
        growsForText: false
        // A reading held back keeps its seat and greys; a side that does not exist has none
        // (デザイン規約 §左メニューの所作 の削除の表) — core's rows, here and on `Delete both`.
        // Held back is "not now", both ways (user decision — ask before changing): `drift`, for the reason the branch's
        // remote rows grey (`BranchRemoteDeleteRow`) — a tag's reading has no namespace, so it goes from the menu that
        // names that remote; `unnamed` (the remote pushes go to lacks the name, several others carry it): that it is
        // over there can be said, but which remote to send to is not decided until a step names one.
        word: "delete-remote-tag"
        whyNames: tagMenu.farNames
        reasonMarks: tagMenu.farMarks(refRemoteTagDeleteItem)
        holdPresses: Metrics.holdPressesWarned
        // Warning: only a name goes, and the commit stays (デザイン規約 §タグを作る・送る).
        holdTone: Theme.warning
        premise: [state.reach, state.refId, state.refOid, state.rowGoes].join("\n")
        onHeld: {
            refRemoteTagDeleteItem.closeMenu()
            tagMenu.deleteRemoteTagRequested(state.reach, state.refId, state.rowGoes, state.refOid)
        }
    }
    // Two commands, so words and no chip (§git 用語のコード表記 の 1:1 規則).
    AppMenuItem {
        id: refBothTagDeleteItem
        text: qsTr("Delete both")
        word: "delete-tag-both"
        whyNames: tagMenu.farNames
        reasonMarks: tagMenu.farMarks(refBothTagDeleteItem)
        holdPresses: Metrics.holdPressesWarned
        // Only names go — git keeps the object — so never the danger of the branch's `-D`.
        holdTone: Theme.warning
        premise: [state.refId, state.reach, state.refOid].join("\n")
        onHeld: {
            refBothTagDeleteItem.closeMenu()
            tagMenu.deleteTagEverywhereRequested(state.reach, state.refId, state.refOid)
        }
    }
}
