import QtQuick
import platitude.ui

/// A target's ask for the window's one tooltip (`TipSeat`): what to say, whether it is asked right now, and how the
/// box stands against the target. Declared beside the target, naming it; the words may change while the box is up
/// and the box follows them (規約 §hover のツールチップ「出ている tip は出たまま字だけ入れ替わり」).
///
/// `asked` is the target's own condition — a hand resting on it, a stand-in a run wrote, a refusal of what is typed.
/// The seat does the rest: the wait before the box comes out, the beat for a hand walking into it, the hand-over to
/// the next target, and what a held scroll bar or a menu does to it. A target that takes its ask back for a reason of
/// its own (a tab taken up) says `drop()`, so the box goes for good and is not read as a hand walking into it.
///
/// **A target that moves under the hand** — a row in a list, a line in the diff — moves with a surface whose hand
/// knows it (`hand`, a `HandWatch` told `rowsMoved` as the rows go): what the rows brought under a hand that did not
/// move asks nothing until the hand has moved (デザイン規約 §左メニューの所作「手が動いていない所へ来た行は開かない」 —
/// Qt hands its hover to whatever arrives under a still pointer), and the rows moving takes the box down at once
/// (§hover のツールチップ「的が手の下で動けば hover は外れ、出したものはその瞬間に消える」). The surface is the nearest
/// list over the target that names its hand (`rowsHand`, found once the ask is built), unless the target hands one in;
/// a target in none — a band's button, a box's own line — does not move under the hand.
///
/// **Declared as a readonly property's value**, named like its id (`readonly property TipAsk buttonTip: TipAsk {
/// id: buttonTip … }`), never as a bare child: Qt 6.12's incremental collector keeps an item's child items through the
/// item but a plain QObject child only through its ancestors, and can lose one while the page is built — its storage
/// goes, and every property reads empty from then on. A property holds it from its owner's own storage.
/// TODO(QTBUG-122800): a workaround for Qt holding that storage weakly — recheck on every Qt update and drop the rule
/// once the storage lives as long as its object (internal-docs/P3-確認事項.md, the incremental collector's item).
QtObject {
    id: ask

    /// The item the box stands against.
    required property Item target
    property bool asked: false
    property string words: ""
    /// The ask answers what is typed, not a hand (a name box's refusal): the box stands until the ask falls, wherever
    /// the hand goes, and no hover takes the box off it (規約 §hover のツールチップ「入力の拒否への補足は一律」).
    property bool typed: false
    /// The box stands beside the target, level with the hand, rather than over its words (`NavRowFacts`).
    property bool beside: false
    /// The target is a row among rows, naming the side its box stands on first (`"left"` / `"right"`); empty for any
    /// other target (規約 §hover のツールチップ「行の的は、行の横に立つ」).
    property string rowSide: ""
    /// A word in `words` that is a place to go, and its href (`HoverToolButton.tipPlace`).
    property string place: ""
    property string href: ""
    /// The word the tree mark stands in front of, and which mark (規約 §ref の種別「ツールチップだけは字の中」).
    property string markWord: ""
    property string markKind: "tree"
    /// The names the box says, each behind its mark — `[at, kind, tint, length]` as `CardText.marks` takes them —
    /// where the target seats them itself (a menu row's whole line, a cut sentence). Null reads the target's own
    /// `tipMarks`, the way the one word above falls back to its `tipMarkWord` / `tipMarkKind`.
    property var marks: null
    readonly property var marksNow:
        ask.marks !== null ? ask.marks
        : ask.target.tipMarks !== undefined && ask.target.tipMarks !== null ? ask.target.tipMarks : []
    readonly property string markWordNow:
        ask.markWord !== "" ? ask.markWord : ask.target.tipMarkWord !== undefined ? ask.target.tipMarkWord : ""
    readonly property string markKindNow:
        ask.markKind !== "" && ask.markKind !== "tree" ? ask.markKind
        : ask.target.tipMarkKind !== undefined && ask.target.tipMarkKind !== "" ? ask.target.tipMarkKind : "tree"

    /// The hand over the surface the target moves with (above), null for a target that does not move under it.
    property HandWatch hand: null
    /// The rows moved under that hand and it has not moved since: the target under it now was brought there.
    readonly property bool handStill: ask.hand !== null && ask.hand.stillHeld
    /// The ask as the seat hears it: `asked`, but for a target the rows brought under a still hand. A refusal of what
    /// is typed answers the typing, wherever the hand is.
    readonly property bool live: ask.asked && (ask.typed || !ask.handStill)

    /// Named so a late answer is weighed against the ask it was for.
    readonly property string key: ask.keyNow
    property string keyNow: ""
    /// The seat answering this target's window, found at the first ask.
    property var seat: null
    /// The box stands on this target.
    readonly property bool shown: ask.seat !== null && ask.seat.shownNow && ask.seat.current === ask

    /// The ask is taken back for the target's own reason: the box goes now and is not put back.
    function drop() {
        if (ask.seat !== null)
            ask.seat.drop(ask)
    }

    onLiveChanged: {
        if (ask.live) {
            if (ask.seat === null)
                ask.seat = Tips.seatFor(ask.target)
            if (ask.seat !== null)
                ask.seat.ask(ask)
        } else if (ask.seat !== null) {
            ask.seat.unask(ask)
        }
    }
    // The rows moved: the box out on this target, or its rest, goes now — its place is stale (`TipSeat.slid`). Under a
    // hand the surface hears the ask has already fallen (`live`) and is made again once the hand has moved; one still
    // standing — a hand the surface cannot hear, a run's stand-in — is asked again from where the target stands now.
    property Connections rowsWatch: Connections {
        target: ask.typed ? null : ask.hand
        function onRowsShifted() {
            if (ask.seat !== null)
                ask.seat.slid(ask)
        }
    }
    Component.onCompleted: {
        ask.keyNow = Tips.key()
        if (ask.hand === null)
            ask.hand = Tips.rowsHandOf(ask.target)
        if (ask.live)
            ask.liveChanged()
    }
    Component.onDestruction: {
        if (ask.seat !== null)
            ask.seat.forget(ask)
    }
}
