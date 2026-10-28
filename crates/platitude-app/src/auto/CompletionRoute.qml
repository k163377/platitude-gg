import QtQuick
import platitude
import platitude.ui

/// The route every completion takes (`Awaited.complete`, `Awaited.route`): `from` is the completing operation's
/// `{ op, record, gen, report, arriving }`, or null for a run a breach ended. Held until the page owes nothing but the
/// reads the completion declared arriving, and nothing the page's own holds name (`Awaited.pageHolds` — a menu on its
/// way out); then the report is read and said, and `readied` goes out in that same turn — the walk and the
/// completion's line that follow are of the state the report read. A report read before the hold would be of a state
/// the hold then passed over. Checked at once, so a completion with nothing owed completes in the turn it was asked.
/// Built by `WindowHarness` over the page's reads, and by `tst_stages.qml` over reads it holds back itself.
// An `Item` only because `QtObject` has no default property to hold the sampler below.
Item {
    id: route

    /// What the page still owes now (`PageSettled.missing`'s names).
    required property var owed
    /// The completion asked for, and whether its report has been read and said: the one way to the completion's walk.
    property var completing: null
    property bool asked: false
    property bool read: false

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
        if (from === null || route.held()) {
            hold.stop()
            route.readReport()
        } else {
            hold.start()
        }
    }
    /// Whether the completion asked for may read its report now: the page owes nothing the completion did not declare
    /// arriving, and the page's own holds are clear.
    function held() {
        const declared = route.completing.arriving
        const terms = ({})
        for (const term of route.owed()) {
            if (declared.indexOf("*") < 0 && declared.indexOf(term) < 0)
                terms["page:" + term] = false
        }
        const holds = Awaited.pageHolds === null ? ({}) : Awaited.pageHolds()
        for (const name in holds)
            terms[name] = holds[name]
        return Awaited.all("completion", terms)
    }
    /// Reads the completion's report and says it. A generation of the completing operation begun since the completion
    /// was asked makes it an old generation's: a breach.
    function readReport() {
        const from = route.completing
        if (from !== null) {
            if (!Awaited.current(from)) {
                Awaited.breach(from.op, "completed from generation " + from.gen + " after generation "
                               + from.record.gen + " began")
                return
            }
            const lines = Awaited.read(from.op, from.report)
            if (lines === null)
                return
            for (const line of lines)
                Harness.report(line)
        }
        route.read = true
        route.readied()
    }

    // Not an operation: the completion's own hold, between the transition the verb made and the line said here.
    SampleTimer {
        id: hold
        onTriggered: {
            if (!route.held())
                return
            hold.stop()
            route.readReport()
        }
    }
}
