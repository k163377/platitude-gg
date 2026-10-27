pragma Singleton
import QtQuick
import platitude

/// What the running verb is still waiting on, said only when it changes and read back by the ceiling
/// (`AutoShotDriver`) — a sampler keeps none of its terms, so this is where a stalled run names the one that stayed
/// false (app-ui.md「段を持つドライバは段が変わるたびに名乗る」). Every line here carries the run's id (`runId`), so a
/// line is read only against the run that said it (`xtask::verify::outcome`).
///
/// The same lines are the run's contract (反映前テストの機械化 §検証時点): a wait that comes true is a checkpoint
/// (`checkpoint run= name= seq=`), the claims a verb places (`claim`) are read again at each picture
/// (`pictured run= artifact= name= holds=`), and a verb's completion is said by name (`complete run=`). The parent
/// fails a run whose claims were never pictured, did not hold, or whose checkpoints repeat.
QtObject {
    id: awaited

    /// The run's name: the directory its pictures go to, which the parent chose for this run alone.
    readonly property string runId: {
        const dir = Harness.shotDir
        const cut = Math.max(dir.lastIndexOf("/"), dir.lastIndexOf("\\"))
        return cut < 0 ? dir : dir.substring(cut + 1)
    }

    /// The last line said and when (`PerfProbe.clockMs`); empty if the verb said nothing through here.
    property string said: ""
    property real since: 0

    /// The window the run draws in and how many frames it has swapped, fed by `WindowHarness`: the one render
    /// boundary a wait on a layout reads. A frame swapped is a polish run before it; a scene that stopped swaps
    /// nothing unasked, so a wait here asks for a frame each time it is read
    /// (rules-refs/app-ui.md「`frameSwapped` を待つなら頼むのは `window.update()`」).
    property var window: null
    property int frames: 0
    /// What `still` / `drawnSince` keep per name: the value seen and the frame it was seen at, or the frame asked at.
    property var marks: ({})

    /// Whether `value` has stood across a swapped frame: true once the value read now is the one read at an
    /// earlier frame — a layout that was still moving between them would have changed it. In place of "two
    /// samples agree", whose two samples can both fall before one layout pass: here a polish ran between.
    function still(name, value) {
        const mark = awaited.marks[name]
        if (mark === undefined || mark.value !== value) {
            awaited.marks[name] = { "value": value, "frame": awaited.frames }
            awaited.window.update()
            return false
        }
        if (awaited.frames <= mark.frame) {
            awaited.window.update()
            return false
        }
        return true
    }

    /// Whether two frames have swapped since `name` was first asked about: the first may have been drawn from a
    /// polish that ran before the ask, the second cannot. For a layout asked for (a card opened, a group laid out)
    /// whose shape has no property of its own to read.
    function drawnSince(name) {
        let mark = awaited.marks[name]
        if (mark === undefined) {
            mark = { "frame": awaited.frames }
            awaited.marks[name] = mark
        }
        if (awaited.frames < mark.frame + 2) {
            awaited.window.update()
            return false
        }
        return true
    }

    /// Forgets a mark, for a wait asked again for another thing of the same name.
    function forget(name) {
        delete awaited.marks[name]
    }

    /// Whether a list (`ListView`) has built the rows it shows. A model's rows are there the moment they land; their
    /// delegates come in the polish after (`QQuickItemView::refill`, which fills the viewport in one pass — the cache
    /// buffer beyond it a frame at a time, off screen). So the row under the viewport's top-left corner stands for the
    /// whole viewport, and a verb that reads rows (`itemAtIndex`) or completes on a list reads this first: read in
    /// the beat the rows landed, the list is empty of delegates and the picture is not. A list with no rows has
    /// nothing to build; one with no size shows nothing.
    function built(view) {
        if (!view || view.count === undefined || view.count === 0)
            return true
        if (view.width <= 0 || view.height <= 0)
            return true
        return view.indexAt(view.contentX + 1, Math.max(0, view.contentY) + 1) >= 0
    }

    /// The checkpoints reached so far, in order: a wait's name once every term of it came, a staging's step as
    /// `<verb>/<step>`. The parent fails a run that reaches one name twice.
    property var reached: []

    /// Whether the run has completed (`completed`): a sampler still running after that — the page's own, waiting on
    /// a page a window's act did not need — reaches nothing more, since a checkpoint after the completion is a run
    /// still going (`verify::contract`).
    property bool done: false

    /// The claims the pictures are of, by name: the predicate that has to hold, and the artifacts it is read at
    /// (`app` / `overlay` / `scene`; every one when none is named). Placed by a verb that completes on a state
    /// something else can pass (a card the reads could take down); read at each picture, never cleared.
    property var claims: ({})

    function claim(name, holds, artifacts) {
        const bound = ({})
        for (const key in awaited.claims)
            bound[key] = awaited.claims[key]
        bound[name] = { "holds": holds, "artifacts": artifacts === undefined ? ["app", "overlay", "scene"] : artifacts }
        awaited.claims = bound
    }

    /// Said at each picture (`AutoShotDriver.pictured`), for every claim bound to that artifact:
    /// `pictured run=<id> artifact=<a> name=<n> holds=<true|false|error>`. A predicate that throws is `error`:
    /// a state the verb could not even read is no state it verified.
    function pictured(artifact) {
        for (const name in awaited.claims) {
            const bound = awaited.claims[name]
            if (bound.artifacts.indexOf(artifact) < 0)
                continue
            let holds = "error"
            try {
                holds = bound.holds() === true ? "true" : "false"
            } catch (e) {
                holds = "error"
            }
            Harness.report("pictured run=" + awaited.runId + " artifact=" + artifact + " name=" + name
                           + " holds=" + holds)
        }
    }

    /// The page's reads a verb completes on by design while they are still on their way (`PageSettled.missing`'s
    /// names): a verb that photographs a page arriving says so here before it completes, and the parent holds the
    /// completion to it. Empty for every verb that completes on a settled page — the rule.
    property var arriving: []

    /// The verb's completion, by name, before the pictures: a run whose pictures come with no completion, or whose
    /// completion comes with no checkpoint reached, is one that ended on its own count. `missing` is what the page
    /// still owed in the completing turn (`PageSettled.missing`): said as `page=settled`, or `page=arriving:` with
    /// the names — a report written then read a page still moving, and the parent fails the run unless the verb said
    /// it would (`arriving`, said beside as `expected=`).
    function completed(verb, missing) {
        const owed = missing === undefined ? [] : missing
        awaited.done = true
        Harness.report("complete run=" + awaited.runId + " verb=" + verb + " checkpoints=" + awaited.reached.length
                       + " page=" + (owed.length === 0 ? "settled" : "arriving:" + owed.join(","))
                       + (awaited.arriving.length === 0 ? "" : " expected=" + awaited.arriving.join(",")))
    }

    /// Whether each named wait held at its last reading, so a wait read again on every beat — a guard at the top of
    /// a sampler — is one checkpoint, reached when its terms first all hold; one whose terms went away and came back
    /// is reached again, which the parent names when another checkpoint came between (a page that stood, was stood
    /// elsewhere and stood again).
    property var held: ({})

    /// Whether every term has come. `terms` maps a name to whether it is there; the missing names are said under
    /// `verb`, the verb's report prefix. The first time every term is there, `verb` is a checkpoint reached.
    function all(verb, terms) {
        const missing = []
        for (const name in terms) {
            if (!terms[name])
                missing.push(name)
        }
        awaited.note(verb + " waiting=" + (missing.length === 0 ? "nothing" : missing.join(",")))
        const holds = missing.length === 0
        if (holds && awaited.held[verb] !== true)
            awaited.reach(verb)
        awaited.held[verb] = holds
        return holds
    }

    /// The step each staging stands at (`at`), so a step said on every beat is one checkpoint.
    property var steps: ({})

    /// A staging that goes in steps: the one it is standing at — a checkpoint when the step changes.
    function at(verb, step) {
        const name = verb + "/" + String(step).replace(/\s+/g, "_")
        if (awaited.steps[verb] === name)
            return
        awaited.steps[verb] = name
        awaited.note(verb + " step=" + step)
        awaited.reach(name)
    }

    /// `name` reached: once per name in a row — a wait read true on every beat after is the same checkpoint; a
    /// name reached again after another is a run going back, which the parent names.
    function reach(name) {
        const spelled = String(name).replace(/\s+/g, "_")
        const last = awaited.reached.length === 0 ? "" : awaited.reached[awaited.reached.length - 1]
        if (last === spelled)
            return
        awaited.reached = awaited.reached.concat([spelled])
        Harness.report("checkpoint run=" + awaited.runId + " name=" + spelled + " seq=" + awaited.reached.length)
    }

    function note(line) {
        if (line === awaited.said)
            return
        awaited.said = line
        awaited.since = PerfProbe.clockMs()
        Harness.report(line)
    }
}
