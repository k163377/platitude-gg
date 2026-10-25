import QtQuick
import platitude.ui

/// The left panel's open row: which row has its facts open under it on hover (`NavRowFacts`), and its seat. One for
/// the panel (`SidebarRowGestures.openRow`): delegates are recycled, only one row is ever open, and it may be in the
/// sections, the sticky stand-in or the folded rail's peek.
QtObject {
    id: openRow

    /// The panel's hand (`HandWatch`), which the open row's seat reads: a row the rows moved under opens only once
    /// the hand has moved.
    property HandWatch hand: null
    /// The page's standing menu's door (`RepoPage.menuDoor`): where it was raised and on what, null while none stands.
    property var menuDoor: null
    /// The key of the row a name box is open on, "" for none.
    property string editKey: ""
    /// The panel, which says where the open row's lines are (`openFactsItem`).
    property Item host: null
    /// Which BRANCHES row has its facts open, by its key. **Written by the seat alone** (`seat`).
    property string openKey: ""
    /// The open row's seat (`HoverSeat`): the rest before a row opens, the beat after the hand leaves it, what holds
    /// it (a menu raised on it, the tip its lines put out) and what closes it (a name box, a menu raised elsewhere, a
    /// held scroll bar, the rows moving under a still hand). No card: the row grows in place, so the opening is
    /// `openKey` itself. A row that comes under a still hand is refused until the hand moves and the rest runs again
    /// (`HandWatch.stillHeld`): an open row is taller, so the rows below move, and opening whatever lands under a still
    /// pointer would have rows trading places (デザイン規約 §左メニューの所作).
    property HoverSeat seat: HoverSeat {
        hand: openRow.hand
        restMs: Metrics.tipDelayMs
        // No beat: the lines are **inside** the row, so a hand walking down into them has not left it, and a hand
        // off the row is off its lines too. What keeps a row open with the hand elsewhere is a hold — its own tip,
        // its own menu.
        graceMs: 0
        // Nothing opens over a name box or under a menu raised elsewhere: opening moves the rows under both. One
        // expression over the door — `menuOnRow` is derived from it too, and would be read before it moved.
        blockedBy: openRow.editKey !== "" ? "edit"
                 : openRow.menuDoor !== null
                   && !(openRow.openKey !== "" && openRow.menuDoor.key === openRow.openKey) ? "menu" : ""
        onOpenAsked: target => openRow.openKey = target.key
        onCloseAsked: (target, why) => {
            if (openRow.openKey === target.key)
                openRow.openKey = ""
        }
    }
    /// The hand came onto `row`'s own line or its lines — `key` is the row's (`NavList.keyOf`), `at` where the hand
    /// stood, `now` a press going for the words, which skips the rest. The seat asks again at the rest, and when the
    /// section's rows change (`rowsRebuilt`), whether the target is still there: the row, while that delegate shows
    /// the key (delegates are recycled — a pooled one still says the key it last showed); or the section's model still
    /// holding the name on show (`model` / `name`, where the row hands them over — the stand-in has none).
    function openFacts(key, at, row, now, model, name) {
        openRow.seat.point({
            "key": key, "at": at, "row": row,
            "alive": () => (row !== undefined && row !== null && row.visible && row.pooled !== true
                            && (row.factsKey !== undefined ? row.factsKey : row.rowKey) === key)
                          || (model !== undefined && model !== null && model.rowOfName(name) >= 0)
        }, now === true)
    }
    /// The section's rows changed (`NavSectionModel.changed` — a read, or the reader's filter, fold or delete): the row
    /// is open only while the model still shows it — a name gone takes the lines down, held or not. A row still
    /// resting is left to its rest, which asks the same question when it runs out (the stand-in asks before it is on
    /// screen, and a change of another section would take that ask away).
    function rowsRebuilt() {
        const seat = openRow.seat
        if (seat.shown && seat.target !== null && seat.target.alive !== undefined && !seat.target.alive()) {
            seat.takeDown("gone", false)
            return
        }
        // Still there, it says what the read brought — the counts, the reading, a branch another worktree took — in
        // place (§「外乱では閉じない」: what is still there changes its words). Any section's read: a row's lines speak of
        // the others' rows too (`NavFacts.answers`).
        const row = seat.shown && seat.target !== null ? seat.target.row : null
        if (row !== null && row !== undefined && row.factsOpen === true && row.gatherFacts !== undefined)
            row.gatherFacts()
    }
    /// …and left it. Only the open row's leave lets it go (`HoverSeat.unpoint`); `now` is a row that slid out from
    /// under a still hand — the reader's scroll — which goes at once, with no beat (デザイン規約 §hover のツールチップ
    /// 「的が手の下で動けば hover は外れ、出したものはその瞬間に消える」), by its own key alone: the rows read the hand
    /// again in no set order, and the row that slid under it may have asked already — that ask stands.
    function closeFacts(key, now) {
        if (now === true)
            openRow.seat.forget(target => target.key === key, "slid")
        else
            openRow.seat.unpoint(key)
    }
    /// The tip the open row's lines put out is standing (`NavRowFacts` → `TipAsk`): the hand reading it is off the
    /// row, which stays open under it (規約 §hover のツールチップ「hover が出した字は選べて、コピーできる」).
    readonly property bool tipOut:
        Tips.current !== null && openRow.host !== null && openRow.host.openFactsItem !== undefined
        && Tips.shownIn(openRow.host.openFactsItem())
    onTipOutChanged: openRow.seat.hold("tip", openRow.tipOut)

    /// A menu raised on the open row — its own line or its lines — keeps it open: it is about that row, which the
    /// door's key says (the page keys a row's menu as the panel keys the row). Every other menu closes it, or it would
    /// move rows under the hand reading the menu.
    readonly property bool menuOnRow:
        openRow.menuDoor !== null && openRow.openKey !== "" && openRow.menuDoor.key === openRow.openKey
    onMenuOnRowChanged: openRow.seat.hold("menu", openRow.menuOnRow)

    /// A read moved the rows under the still hand further than the list could take back (`HandWatch.rowsMoved(true)`):
    /// the open row stays open where the read put it, the hand on it or not — its lines are what the hand was reading
    /// (デザイン規約 §hover のツールチップ「外乱では閉じない」) — until the hand moves; then the hand decides again, and the
    /// row under it, asked for meanwhile, opens on its rest. A row the read took out goes all the same (`rowsRebuilt`).
    property Connections rereadWatch: Connections {
        target: openRow.hand
        function onRowsShifted() {
            if (openRow.hand.disturbed && openRow.hand.stillHeld && openRow.seat.shown)
                openRow.seat.hold("reread", true)
        }
        function onStillHeldChanged() {
            if (!openRow.hand.stillHeld)
                openRow.seat.hold("reread", false)
        }
    }
}
