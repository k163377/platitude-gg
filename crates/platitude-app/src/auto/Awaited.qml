pragma Singleton
import QtQuick
import platitude

/// What the verb running now is still waiting on — said once each time that changes, and read back by the ceiling
/// when the run never got there (`AutoShotDriver`). A sampler reads its terms on every beat and keeps none of them,
/// so a run that ends at its watchdog says which one stayed false only through here
/// (app-ui.md「段を持つドライバは段が変わるたびに名乗る」).
///
/// **Said on change, not on every beat.** A run that goes straight through says a handful of lines, and a term that
/// comes and goes says a line each way — which is what a stall in the middle of a travel looks like.
QtObject {
    id: awaited

    /// The last line said, and when, on the run's own clock (`PerfProbe.clockMs`). Empty for a run whose verb said
    /// nothing through here.
    property string said: ""
    property real since: 0

    /// Whether every term has come. `terms` maps a name to whether that part is there; the names still missing are
    /// said under `verb`, the prefix the verb's own report carries.
    function all(verb, terms) {
        const missing = []
        for (const name in terms) {
            if (!terms[name])
                missing.push(name)
        }
        awaited.note(verb + " waiting=" + (missing.length === 0 ? "nothing" : missing.join(",")))
        return missing.length === 0
    }

    /// A staging that goes in steps: the one it is standing at.
    function at(verb, step) {
        awaited.note(verb + " step=" + step)
    }

    function note(line) {
        if (line === awaited.said)
            return
        awaited.said = line
        awaited.since = PerfProbe.clockMs()
        Harness.report(line)
    }
}
