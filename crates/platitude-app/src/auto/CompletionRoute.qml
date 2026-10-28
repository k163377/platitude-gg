import QtQuick
import platitude
import platitude.ui

/// The route every completion takes (`Awaited.complete`, `Awaited.route`): `from` is the completing operation's
/// `{ op, record, gen, name, when, report, lost, arriving }`, or null for a run a breach ended.
///
/// The completion stands on one reading of three things at once: the verb's own conditions (`when`, which held when
/// the operation moved and are read again here), the page's reads but those the completion declared arriving
/// (`owed`), and the page's own holds (`Awaited.pageHolds` — a menu on its way out). It holds, reading again each
/// beat, until that reading holds; then the report is read, and the reading taken again in the same turn: a report
/// read across a change — a read landing on the page's own thread, a condition falling — is thrown away unsaid and
/// the hold goes on. A report that stands is said, and `readied` goes out in that turn — the walk and the
/// completion's line that follow are of the state the report read, with the witnesses it read named
/// (`witnessesRead`). A verb condition that falls during the hold is waited for again, or ends the run as a breach
/// where the verb said so (`lost: "fail"`); its input is never pressed again. Checked at once, so a completion with
/// nothing to wait for completes in the turn it was asked.
///
/// A list the page shows with rows and no size (`unsized`) holds the completion for a polish pass of the window that
/// began after the reading (`Awaited.polishes`): no size is the layout's answer or a polish not run yet — a log raised
/// in the reading's turn — and only a pass that took the list in tells the two apart, sizing it and building its rows
/// in the one pass, or leaving it with none (`sized:<list>`). A frame swapped is no such pass.
/// A window that cannot render runs none, and the completion holds.
/// Built by `WindowHarness` over the page's reads, and by the QtTests over reads they hold back themselves.
// An `Item` only because `QtObject` has no default property to hold the sampler below.
Item {
    id: route

    /// What the page still owes now (`PageSettled.missing`'s names).
    required property var owed
    /// The lists the page shows with rows and no size now (`PageSettled.unsized`'s names).
    property var unsized: () => []
    /// The lists with rows and no size the hold first read, and how many polish passes the window had made then:
    /// `{ key, polish }`, null while there are none.
    property var sizing: null
    /// The completion asked for, and whether its report has been read and said: the one way to the completion's walk.
    property var completing: null
    property bool asked: false
    property bool read: false
    /// What the page owed at the reading the report was read on and checked against — the state the report read,
    /// which the completion's line says.
    property var owedAtRead: []
    /// The witnesses the report read, as `<owner>/<op>/<gen>/<stage>/<name>` (`Awaited.witnessed`).
    property var witnessesRead: []

    /// The report was read and said; the run completes on what it read, in this turn.
    signal readied()

    function finishFrom(from) {
        // A run ordered to hold its act has none (`PGG_FAULT_HOLD_ACT`): only the ceiling ends it — the shape
        // `cargo xtask wedge-check` reads back (`xtask::verify::faults`).
        if (Harness.faultHoldAct || route.read)
            return
        if (route.asked && from !== null) {
            Awaited.breach(from.op, "completed a run another operation had completed")
            return
        }
        route.asked = true
        route.completing = from
        if (from === null) {
            hold.stop()
            route.owedAtRead = route.owed()
            route.read = true
            route.readied()
            return
        }
        route.attempt()
    }

    /// One reading of what the completion stands on: `{ when, owed, terms }`, or null when it could not be read —
    /// a `when` or page holds that threw, a breach said at once: the hold would otherwise stand to the ceiling naming
    /// nothing.
    function reading() {
        const from = route.completing
        const terms = ({})
        let when
        try {
            when = from.when()
        } catch (e) {
            hold.stop()
            Awaited.breach(from.op, "a completion's `when` that threw: " + e)
            return null
        }
        for (const term in when)
            terms[term] = Boolean(when[term])
        const owed = route.owed()
        for (const term of owed) {
            if (from.arriving.indexOf("*") < 0 && from.arriving.indexOf(term) < 0)
                terms["page:" + term] = false
        }
        let holds = ({})
        try {
            holds = Awaited.pageHolds === null ? ({}) : Awaited.pageHolds()
        } catch (e) {
            hold.stop()
            Awaited.breach(from.op, "the page's holds on the completion could not be read: " + e)
            return null
        }
        for (const name in holds)
            terms[name] = Boolean(holds[name])
        const unsized = route.unsized()
        const sized = route.sizing !== null && route.sizing.key === unsized.join(",")
            && Awaited.layoutPassedSince(route.sizing.polish)
        for (const name of unsized)
            terms["sized:" + name] = sized
        return { "when": when, "owed": owed, "unsized": unsized, "terms": terms }
    }
    /// Marks the lists with rows and no size where the hold first reads them, and asks for the pass it waits on — a
    /// still scene renders none unasked.
    function markSizing(unsized) {
        const key = unsized.join(",")
        if (key === "") {
            route.sizing = null
            return
        }
        if (route.sizing === null || route.sizing.key !== key)
            route.sizing = { "key": key, "polish": Awaited.layoutMark() }
        if (!Awaited.layoutPassedSince(route.sizing.polish))
            Awaited.askLayout()
    }
    /// Whether two readings are of one state: the same terms holding, and the same reads owed.
    function same(a, b) {
        const keys = Object.keys(a.terms)
        return keys.length === Object.keys(b.terms).length
            && keys.every(key => a.terms[key] === b.terms[key])
            && a.owed.length === b.owed.length
            && a.owed.every((term, i) => b.owed[i] === term)
    }

    /// Reads what the completion stands on and, where it holds, the report — kept only if a second reading in the
    /// same turn finds the same state.
    function attempt() {
        const from = route.completing
        const first = route.reading()
        if (first === null)
            return
        route.markSizing(first.unsized)
        const fell = Object.keys(first.when).filter(term => !first.when[term])
        if (fell.length > 0 && from.lost === "fail") {
            hold.stop()
            Awaited.breach(from.op, "lost " + fell.join(",") + " during the completion's hold")
            return
        }
        if (!Awaited.all("completion", first.terms)) {
            hold.start()
            return
        }
        if (!Awaited.current(from)) {
            hold.stop()
            Awaited.breach(from.op, "completed from generation " + from.gen + " after generation "
                           + from.record.gen + " began")
            return
        }
        const lines = Awaited.read(from.op, from.report)
        if (lines === null) {
            hold.stop()
            return
        }
        const witnesses = Awaited.readWitnesses
        const second = route.reading()
        if (second === null)
            return
        if (!route.same(first, second)) {
            Awaited.note("completion moved while its report was read")
            hold.start()
            return
        }
        hold.stop()
        route.owedAtRead = first.owed
        route.witnessesRead = witnesses
        for (const line of lines)
            Harness.report(line)
        route.read = true
        route.readied()
    }

    // Not an operation: the completion's own hold, between the transition the verb made and the line said here.
    SampleTimer {
        id: hold
        onTriggered: route.attempt()
    }
}
