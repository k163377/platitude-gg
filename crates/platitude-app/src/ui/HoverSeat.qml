import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Templates as T
import platitude.ui

/// One thing hover puts out — a row's card, a chip's list, a row's open lines, the window's tooltip — as a state
/// machine with one writer. The parts tell it what happened (the hand on a target, off it, inside what it opened; a
/// menu raised from it; a target gone) and it decides what stands; nobody else writes the answer.
///
/// `idle` → `waiting` (the rest, `restMs`; none for what unfolds at once) → `shown` → `leaving` (the beat,
/// `graceMs`, for a hand walking into what is out) → `idle`. Every target is a plain object with a `key` — a commit
/// and the area it is shown in, a row's key, a chip — never a delegate: delegates are recycled, and a key is what a
/// late timer or a stray leave is weighed against. A target may carry `alive()`, asked before it is opened.
///
/// What keeps the shown target up is a named hold (`hold`): the hand inside the card, a menu raised from it, a tip its
/// rows put out. Holding is not pointing — nothing opens on a hold, and a hold let go starts the beat. A hold is about
/// the target it was placed on: a held target is not handed over, and the hand on another meanwhile is an ask kept for
/// the hold lifting — answered the moment it lifts, as it would have been with no hold (the order Qt tells the card's
/// leave and the next target's enter in decides nothing). What keeps anything from opening is `blockedBy` (a reader's
/// operation: what is out goes as a dismissal, and the ask is kept for the walk that lifts the latch; `exempt` names
/// the targets it does not hold back), `frozen` (what the hand does changes nothing, in every state: a rest or a beat
/// that runs out under it is answered at the thaw — what goes for a reason of its own still goes), a scroll bar held
/// outside the card (`Hand.heldBar`, which also drops the ask), and the hand standing still while the rows moved under
/// it (`HandWatch.stillHeld`). A target the reader put down — any operation of theirs that took it down with the rows
/// where they were (the card closing by itself, the owner's `takeDown`, a blocker) — is not put back under the same
/// still hand: Qt hands the target its hover again the moment the popup is gone (`latchedKey`). A reread is no
/// dismissal (below). **What lifts the hold is the hand's own move**, for every seat (`HandWatch.stillHeld`). **What
/// lifts a latch is the seat's kind**: any move for one that rests — the rest is the judgement, as Qt's tooltips wake
/// on the mouse's own moves (2px and a pause is a hand's own hover); for one that opens at once (`expands`), the hand
/// off its target — as a Qt Quick tooltip a press hid returns only when its item's hover changes
/// (デザイン規約 §左メニューの所作「手が動いたかの見切り」) — and a press, for an owner that gives its target one (`press`).
///
/// What a reread does to a target is the owner's to weigh, and dismisses nothing (デザイン規約 §hover のツールチップ「外乱では閉じない」): one
/// whose showing is gone (a row's commit out of the graph, a chip with no names left) goes by `forget`, picked by what
/// asked for it — the delegate, the anchor — never by a key derived from what it shows now, which has already moved on;
/// one whose showing stands elsewhere is set on its new place (`RowHoverHost.reseat`); one that slid from under the
/// hand is displaced (`displace`), but for the one the hand is in.
///
/// With `card` set, the seat drives that popup: the owner opens it on `openAsked` and nothing else does, the seat
/// closes it, hears it close by itself (a dismissal), reads the hand inside it, and holds it for a tip its rows
/// raised.
///
/// Declared as a readonly property's value (`readonly property HoverSeat cardSeat: HoverSeat { id: cardSeat … }`),
/// never as a bare child — the collector can take a bare non-visual child's storage while the page is built (`TipAsk`).
QtObject {
    id: seat

    /// The hand over the surface the targets stand on, where there is one (`HandWatch`).
    property HandWatch hand: null
    /// The popup this seat shows, or null for a showing that is no popup's (the open row).
    property T.Popup card: null
    property int restMs: Metrics.tipDelayMs
    property int graceMs: Metrics.hoverKeepMs

    // Named to `Hovers` for as long as it stands, so whoever asks whether a hover decision is on its way is answered
    // for every seat at once.
    Component.onCompleted: Hovers.enter(seat)
    Component.onDestruction: Hovers.leave(seat)

    /// `idle` / `waiting` / `shown` / `leaving`.
    readonly property string state: seat.stateNow
    property string stateNow: "idle"
    property var target: null
    readonly property string targetKey: seat.target !== null ? seat.target.key : ""
    readonly property bool shown: seat.stateNow === "shown" || seat.stateNow === "leaving"
    readonly property bool waiting: seat.stateNow === "waiting"
    readonly property bool leaving: seat.stateNow === "leaving"
    /// The hand is on the current target's own trigger area.
    property bool pointed: false
    /// The holds standing, by name.
    property var holds: ({})
    readonly property bool held: Object.keys(seat.holds).length > 0
    /// The owner's reason nothing may open now — "" for none. Non-empty takes down what is out and keeps the ask.
    property string blockedBy: ""
    /// …but for these: a function of the target, true where the blocker does not hold it back (the tooltip's targets
    /// inside a standing menu, a refusal of what is typed). Null holds every target back.
    property var exempt: null
    function blocks(target) {
        return seat.blockedBy !== "" && !(seat.exempt !== null && target !== null && seat.exempt(target) === true)
    }
    /// Nothing the hand does opens or closes anything while this holds; the asks are kept and answered when it lifts
    /// (デザイン規約 §グラフ行のダブルクリック「その間、hover は今の姿のまま」). What goes for a reason of its own
    /// still goes: a blocker, a held bar, a target retired or displaced, the card closing by itself, the owner's
    /// `takeDown`.
    property bool frozen: false
    /// A scroll bar is held outside the card (`Hand.heldBar`): the targets are sliding from under a hand Qt no longer
    /// hears (デザイン規約 §hover のツールチップ「スクロールバーを掴んだら、hover で開いたものは閉じる」). The card's own bar
    /// is the hand reading the card. `ignoresBar` is for what hover did not open (a typed refusal's tip).
    property bool ignoresBar: false
    readonly property bool barHeld: Hand.heldBar !== null && !seat.ignoresBar && !seat.holdsBar(Hand.heldBar)
    /// …and for an ask made while the bar is held: a function of the target, true where it may come out all the same.
    property var barExempt: null
    function barBlocks(target) {
        return Hand.heldBar !== null && !seat.holdsBar(Hand.heldBar)
            && !(seat.barExempt !== null && target !== null && seat.barExempt(target) === true)
    }
    /// The display opens the moment the hand is on its target (規約 §hover のツールチップ「展開は即時に開く」: the rail's peek, the band's
    /// state card, a chip's list, the author cards). No rest judges the hand, so a tremor after a click, an Escape or a
    /// menu that took it down would put it straight back: a dismissal lifts with the hand off the target alone, never
    /// with a move on it — the target's leave, or the surface losing the hand, with nothing standing over the hand
    /// (`mayBeCovered`), or the hand heard where the target does not ask for it (`handWatch`) — and with a press, for
    /// an owner that gives its target one (`press`).
    property bool expands: false
    readonly property bool stillHeld: seat.hand !== null && seat.hand.stillHeld

    /// The owner opens what `target` asks for, or answers `refuse()` in the same call.
    signal openAsked(var target)
    /// What was out is to go, for `why` (`left` / `handover` / `blocked` / `bar` / `gone` / the owner's word).
    signal closeAsked(var target, string why)

    /// The ask kept for when a blocker lifts: `{ target, immediate }`.
    property var pending: null
    /// The rest or the beat ran out under the freeze: answered at the thaw.
    property bool due: false
    /// The target the reader dismissed, not asked for again under the same still hand; where it was then.
    property string latchedKey: ""
    property point latchedAt: Qt.point(-1, -1)
    /// The card is going down because this seat took it down (not a dismissal).
    property string closingKey: ""

    function shownFor(key) {
        return seat.shown && seat.targetKey === key
    }
    /// The key of the target `match` picks — the one out or resting, else the ask a blocker keeps — or "" for none:
    /// for a leave told by the part that asked, whatever that part stands for now.
    function keyOf(match) {
        if (seat.target !== null && match(seat.target))
            return seat.target.key
        if (seat.pending !== null && match(seat.pending.target))
            return seat.pending.target.key
        return ""
    }

    /// The hand is on `target`'s trigger area: on the one already out, it is held; on another, the one out goes and
    /// this one waits its rest (`immediate` opens it now — what unfolds, 規約「展開は即時に開く」).
    function point(target, immediate) {
        const now = immediate === true
        if (seat.frozen || seat.latchedKey === target.key
                || (seat.stillHeld && !(seat.shown && target.key === seat.targetKey))
                || seat.blocks(target)
                || (seat.shown && seat.held && target.key !== seat.targetKey)) {
            seat.pending = { "target": target, "immediate": now }
            return
        }
        if (seat.barBlocks(target))
            return
        if (seat.shown) {
            if (target.key === seat.targetKey) {
                seat.target = target
                seat.pointed = true
                seat.pending = null
                seat.settle()
                return
            }
            seat.drop(seat.target, "handover")
        } else if (seat.waiting && target.key === seat.targetKey) {
            // Asked again for the one already resting: the rest runs on (a hand that stays is not a hand that came).
            seat.target = target
            seat.pointed = true
            if (now) {
                seat.rest.stop()
                seat.show()
            }
            return
        }
        seat.pending = null
        seat.target = target
        seat.pointed = true
        if (now) {
            seat.show()
            return
        }
        seat.stateNow = "waiting"
        seat.rest.restart()
    }
    /// A press on `target`'s trigger area: a new ask whatever the latch says — the hover cannot bring a dismissed
    /// card back under a still hand, a press can — and it opens at once. Only an owner that gives its target a press
    /// calls it; a target with none comes back by the hand leaving it alone.
    function press(target) {
        if (seat.latchedKey === target.key)
            seat.latchedKey = ""
        seat.point(target, true)
    }
    /// The hand left `key`'s trigger area. A key that is not the current target's is a leave from somewhere else and
    /// changes nothing (the target the hand came onto may have asked first).
    function unpoint(key) {
        if (seat.pending !== null && seat.pending.target.key === key)
            seat.pending = null
        // The hand gone from the target and back is a fresh ask — but a leave told while something may stand over the
        // hand is no leave of its (`mayBeCovered`); and whether the surface has heard the hand go says nothing then, as
        // Qt tells the leaves in the order the handlers took the hover: there the surface decides, where the hand is
        // heard next (`handWatch`).
        if (seat.latchedKey === key && !seat.shown && !seat.mayBeCovered())
            seat.latchedKey = ""
        if (key !== seat.targetKey)
            return
        seat.pointed = false
        if (seat.frozen)
            return
        if (seat.waiting) {
            seat.rest.stop()
            seat.idle()
            return
        }
        seat.settle()
    }
    function hold(reason, on) {
        if ((seat.holds[reason] === true) === (on === true))
            return
        const next = ({})
        for (const held in seat.holds)
            next[held] = true
        if (on)
            next[reason] = true
        else
            delete next[reason]
        seat.holds = next
        seat.settle()
        // An ask the hold kept for another target is answered now — the hand on it while the hold stood is the hand
        // on it now — not after the beat the hold's falling started.
        if (!on)
            seat.resume()
    }
    /// Takes the shown target down now, for `why`; `latch` says the reader dismissed it, so the same still hand
    /// does not bring it back — the hand still on it is an ask kept for the latch, as at the card's own close
    /// (`noteClosed`). Any other ask kept goes.
    function takeDown(why, latch) {
        const kept = latch === true && seat.pointed && seat.target !== null ? seat.keep(seat.target) : null
        seat.pending = null
        if (!seat.shown && !seat.waiting)
            return
        if (latch === true)
            seat.latchOn()
        seat.drop(seat.target, why)
        seat.pending = kept
    }
    /// The place the target hangs off moved — the rows scrolled or moved under the hand: what is out goes
    /// at once, its place being stale (デザイン規約 §hover のツールチップ「的が手の下で動けば hover は外れ、出したものは
    /// その瞬間に消える」), and an ask still standing is kept — asked again with its rest once the hold is off
    /// (`HandWatch.stillHeld`), or at once where there is none (a hand the rows cannot hear). The target itself is
    /// not in question: for one that went invalid, `takeDown("gone")` — the asker asks again with a new target.
    function displace(why) {
        const kept = seat.pointed && seat.target !== null ? seat.keep(seat.target) : seat.pending
        seat.takeDown(why, false)
        seat.pending = kept
        seat.resume()
    }
    /// The targets `match` picks stopped standing for what they named — a row rewritten to another commit, a chip
    /// gone: the one out (or resting) goes now, whatever holds it and with no latch, and an ask kept for one goes
    /// too; anything else stands, an ask kept for another target included (answered now if this was its holder).
    function forget(match, why) {
        if (seat.pending !== null && match(seat.pending.target))
            seat.pending = null
        if ((seat.shown || seat.waiting) && seat.target !== null && match(seat.target)) {
            seat.drop(seat.target, why)
            seat.resume()
        }
    }
    /// What is out stands for `target` from here, out as it is — no rest, no beat: the owner's word on what the card
    /// answers (`TipSeat.detach` / `takeBack`).
    function retarget(target) {
        if (seat.shown)
            seat.target = target
    }
    /// An ask kept for `target`, to be made again: a seat that opens at once opens it at once, one that rests rests.
    function keep(target) {
        return { "target": target, "immediate": seat.expands }
    }
    /// The owner could not open what was asked (nothing to show): back to idle in the same call.
    function refuse() {
        seat.idle()
    }
    /// The card closed by itself — a dismissal (Escape with the keyboard in it, a press outside). A hand still on the
    /// target is an ask kept for when the latch is off (`resume`): the walk that takes it off is a new hover.
    function noteClosed() {
        if (!seat.shown)
            return
        seat.latchOn()
        seat.rest.stop()
        seat.beat.stop()
        seat.pending = seat.pointed ? seat.keep(seat.target) : null
        seat.idle()
    }
    /// Decides again whether what is out stays: a hold changed somewhere no binding saw.
    function settle() {
        if (!seat.shown || seat.frozen)
            return
        if (seat.pointed || seat.held) {
            seat.stateNow = "shown"
            seat.beat.stop()
            return
        }
        seat.stateNow = "leaving"
        seat.beat.restart()
    }

    function show() {
        if (seat.target !== null && seat.target.alive !== undefined && !seat.target.alive()) {
            seat.idle()
            return
        }
        seat.stateNow = "shown"
        seat.openAsked(seat.target)
    }
    function drop(target, why) {
        seat.rest.stop()
        seat.beat.stop()
        seat.stateNow = "idle"
        seat.target = null
        seat.pointed = false
        if (seat.card !== null && seat.card.visible) {
            seat.closingKey = target.key
            seat.card.close()
        }
        seat.closeAsked(target, why)
    }
    function idle() {
        seat.stateNow = "idle"
        seat.target = null
        seat.pointed = false
        seat.due = false
    }
    function latchOn() {
        seat.latchedKey = seat.targetKey
        seat.latchedAt = seat.hand !== null ? seat.hand.at : Qt.point(-1, -1)
    }
    /// A blocker lifted: the ask it kept is made again — unless another still holds it, the latch and a held target
    /// included, in which case it stays kept for that one's lifting.
    function resume() {
        if (seat.pending === null || seat.frozen || seat.stillHeld || seat.blocks(seat.pending.target)
                || seat.latchedKey === seat.pending.target.key || seat.barBlocks(seat.pending.target)
                || (seat.shown && seat.held && seat.pending.target.key !== seat.targetKey))
            return
        const kept = seat.pending
        seat.pending = null
        seat.point(kept.target, kept.immediate)
    }
    /// Something may stand over the hand on this seat's surface: the hand was lost under a popup or a card
    /// (`HandWatch.hidden`), or one has it now (`HandWatch.underCover` — a target may tell its leave before the surface
    /// hears the hand go). Qt hands nothing under a popup its hover, so a hand gone unheard then may be a hand that
    /// never moved.
    function mayBeCovered() {
        return seat.hand !== null && (seat.hand.hidden || seat.hand.underCover())
    }
    function holdsBar(bar) {
        if (seat.card === null)
            return false
        for (let at = bar; at !== null; at = at.parent) {
            if (at === seat.card.contentItem)
                return true
        }
        return false
    }

    onBlockedByChanged: {
        if (seat.blockedBy === "") {
            seat.resume()
            return
        }
        if ((seat.shown || seat.waiting) && seat.blocks(seat.target)) {
            const kept = seat.pointed ? seat.keep(seat.target) : null
            // A blocker is the reader's input — the press that raised a menu on the rows, a name box opening — so
            // what it takes down goes as a dismissal does, and is not put back under the same still hand when it
            // lifts; the ask is kept for the walk.
            if (seat.shown)
                seat.latchOn()
            seat.drop(seat.target, "blocked")
            seat.pending = kept
        }
    }
    // The thaw decides again what the freeze held: a rest that ran out opens, a beat that ran out closes, a target the
    // hand left is let go, and the ask kept is made again.
    onFrozenChanged: {
        if (seat.frozen)
            return
        const due = seat.due
        seat.due = false
        if (seat.waiting) {
            if (!seat.pointed) {
                seat.rest.stop()
                seat.idle()
            } else if (due) {
                seat.show()
            }
        } else if (seat.shown) {
            if (due && !seat.pointed && !seat.held)
                seat.drop(seat.target, "left")
            else
                seat.settle()
        }
        seat.resume()
    }
    onStillHeldChanged: if (!seat.stillHeld) seat.resume()
    onBarHeldChanged: {
        if (!seat.barHeld)
            return
        seat.pending = null
        if (seat.shown || seat.waiting)
            seat.drop(seat.target, "bar")
    }
    /// A tip one of the card's rows put out is standing (`Tips.shownIn`): the hand reading it is off the card
    /// (rules-refs/app-ui.md「hover で保つカードは、自分の行が出したツールチップの間も保つ」).
    readonly property bool tipHeld: seat.card !== null && Tips.shownIn(seat.card.contentItem)
    onTipHeldChanged: seat.hold("tip", seat.tipHeld)
    /// The hand inside the card (`AppCard.pointerInside`), for a card that reports it. The hand is told too
    /// (`HandWatch.covered`): a hand in the card is over the rows the card hangs off, though they do not hear it —
    /// by the seat showing the card, where two seats share one.
    readonly property bool cardPointed: seat.card !== null && seat.card.pointerInside === true
    property HandWatch covering: null
    onCardPointedChanged: {
        seat.hold("inside", seat.cardPointed)
        if (seat.covering !== null) {
            seat.covering.uncovered()
            seat.covering = null
        }
        if (seat.cardPointed && seat.shown && seat.hand !== null) {
            seat.hand.covered++
            seat.covering = seat.hand
        }
    }

    // A seat that rests lets a dismissal go on a move of the hand's own: a hand gone unheard (under the menu the press
    // raised, in the card it then dismissed) has not moved, and a dismissal made with the hand unheard is anchored at
    // the first place heard — the move is measured from there. What opens at once lets it go with the hand off its
    // target: the surface losing the hand with nothing standing over it (`mayBeCovered`: it left the surface and every
    // target on it), the hand out from under one to where the surface does not reach (`HandWatch.weighCover`), or the
    // hand heard where the target does not ask for it — back from under a menu, or from a card the reader put down with
    // a press elsewhere. The target hears the hand before the surface tells it here (the rows read `HandWatch.moves`,
    // counted before `moved`; a target with a handler of its own lies under the surface's, which Qt tells last), so a
    // hand on it has asked for it again by then, and that ask is kept for the latch: a move on the target lifts
    // nothing.
    property Connections handWatch: Connections {
        target: seat.hand
        function onMoved(at) {
            if (seat.latchedKey === "")
                return
            if (seat.expands) {
                if (at.x < 0) {
                    if (!seat.mayBeCovered())
                        seat.latchedKey = ""
                } else if (seat.pending === null || seat.pending.target.key !== seat.latchedKey) {
                    seat.latchedKey = ""
                    seat.resume()
                }
                return
            }
            if (at.x < 0)
                return
            if (seat.latchedAt.x < 0) {
                seat.latchedAt = at
                return
            }
            // Heard again at the same place (back from under a menu) is no move.
            if (seat.hand.apart(at, seat.latchedAt) < 1)
                return
            seat.latchedKey = ""
            seat.resume()
        }
    }
    // A dismissal is heard as the card starts to go (`aboutToHide`), not once it is gone: the press outside that took
    // it down may be raising a menu in the same event, and the blocker that follows has to find the latch on already
    // — a card the blocker took down instead would come back as the blocker lifts.
    property Connections cardWatch: Connections {
        target: seat.card
        function onAboutToHide() {
            if (seat.closingKey === "")
                seat.noteClosed()
        }
        function onClosed() {
            seat.closingKey = ""
        }
    }

    // Both timers weigh the state again as they run out: a late one answers nothing. Under the freeze they only say
    // they ran out (`due`).
    property Timer rest: Timer {
        interval: seat.restMs
        onTriggered: {
            if (seat.frozen) {
                seat.due = true
                return
            }
            if (seat.waiting && seat.pointed)
                seat.show()
        }
    }
    property Timer beat: Timer {
        interval: seat.graceMs
        onTriggered: {
            if (seat.frozen) {
                seat.due = true
                return
            }
            if (seat.leaving && !seat.pointed && !seat.held) {
                seat.drop(seat.target, "left")
                // What went down may have been holding an ask back (a held target the hand had left for another).
                seat.resume()
            }
        }
    }
}
