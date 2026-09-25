pragma Singleton
import QtQuick
import platitude

/// What the running verb is still waiting on, said only when it changes and read back by the ceiling
/// (`AutoShotDriver`) — a sampler keeps none of its terms, so this is where a stalled run names the one that stayed
/// false (app-ui.md「段を持つドライバは段が変わるたびに名乗る」).
QtObject {
    id: awaited

    /// The last line said and when (`PerfProbe.clockMs`); empty if the verb said nothing through here.
    property string said: ""
    property real since: 0

    /// Whether every term has come. `terms` maps a name to whether it is there; the missing names are said under
    /// `verb`, the verb's report prefix.
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
