pragma Singleton
import QtQuick
import platitude

/// The run's waits, stages and completion, said under the run's id (`runId`) so a line is read only against the run
/// that said it (`xtask::verify::contract`). Three things are kept apart here (反映前テストの機械化 §検証時点):
///
/// - **Observation** (`all`): whether a set of terms holds now. Read as often as a sampler likes; it advances
///   nothing, and a term that holds, falls and holds again is the page moving, not the run going back. What is still
///   missing is said when it changes, for the ceiling (`AutoShotDriver`).
/// - **Transition** (`StageTimer.advance` / `input` / `complete`): one operation's stage moved, once. An operation is
///   one sampler (`PageStage` / `WindowStage`), named by its owner and name; each start of it is a generation with
///   stages of its own. A transition is said (`stage run= owner= op= gen= from= to= seq=`) and refused — a breach,
///   which ends the run — when it repeats a stage of its generation, goes against the order the operation declared,
///   comes from a generation that is over, comes outside its own operation's beat, or comes with the beat's latest
///   observation not holding: a stage moves on what the product answered, never on a count.
/// - **Completion** (`StageTimer.complete`): the run's one ending. The report is handed over unread; the completion
///   route (`CompletionRoute`) holds until the page owes nothing the verb did not declare arriving, then reads
///   the report and walks the scene in the same turn, and says `complete run= …` for that state.
///
/// Independent operations — the page's and the window's — are not ordered against each other: each keeps its own
/// chain, and an operation that needs another's stage first names it (`StageTimer.after`, read by `joined`).
QtObject {
    id: awaited

    /// The run's name: the directory its pictures go to, which the parent chose for this run alone.
    readonly property string runId: {
        const dir = Harness.shotDir
        const cut = Math.max(dir.lastIndexOf("/"), dir.lastIndexOf("\\"))
        return cut < 0 ? dir : dir.substring(cut + 1)
    }

    /// The last line said and when (`PerfProbe.clockMs`); empty if the run said nothing through here.
    property string said: ""
    property real since: 0

    /// The window the run draws in and how many frames it has swapped, fed by `WindowHarness`: the one render
    /// boundary a wait on a layout reads (`StageTimer.still` / `drawnSince`). A frame swapped is a polish run before
    /// it; a scene that stopped swaps nothing unasked, so a wait there asks for a frame each time it is read
    /// (rules-refs/app-ui.md「`frameSwapped` を待つなら頼むのは `window.update()`」).
    property var window: null
    property int frames: 0

    /// The completion route: `WindowHarness`, which holds a completion until its reads are in and then reads its
    /// report (`finishFrom`). Set by the harness as it is built.
    property var route: null
    /// The page's own holds on a completion — its menu still on its way out (`AutoActDriver.completionHolds`) —
    /// handed in by the page that claimed the run.
    property var pageHolds: null

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

    // ---- Observation ----

    /// The operation whose beat is running (`StageTimer`'s own handler runs before the instance's), null between
    /// beats; and the latest observation's answer inside it — null before the beat observed anything.
    property var beating: null
    property var observed: null
    /// Counts beats, so a report said in a beat is known to be this beat's.
    property int beatSerial: 0

    /// Whether every term has come. `terms` maps a name to whether it is there; the missing names are said under
    /// `name` when they change. An observation only: it moves no stage, however often it is read.
    function all(name, terms) {
        const missing = []
        for (const term in terms) {
            if (!terms[term])
                missing.push(term)
        }
        awaited.note(name + " waiting=" + (missing.length === 0 ? "nothing" : missing.join(",")))
        const holds = missing.length === 0
        awaited.observed = holds
        return holds
    }

    function note(line) {
        if (line === awaited.said)
            return
        awaited.said = line
        awaited.since = PerfProbe.clockMs()
        Harness.report(line)
    }

    // ---- Stages ----

    /// Each operation's record, by `owner/name` (a second instance under one name — a second page's driver — is
    /// `name#2`): `{ op, label, gen, live, stage, visited, marks, saidIn }`.
    property var ops: ({})
    /// How many stage lines the run has said: the `seq=` of the next is one more.
    property int stageSeq: 0
    /// The stages reached, as `owner/name/stage` with the instance's mark dropped — what a join names — and a count
    /// that moves with them, for a binding to read them again on.
    property var reachedStages: ({})
    property int stageCount: 0
    /// Whether the run has completed (`completed`), and whether a breach ended it (`breach`): either way nothing
    /// moves after, and a stage said after the completion is a run still going (`verify::contract`).
    property bool done: false
    property bool broken: false

    function recordOf(op) {
        for (const label in awaited.ops) {
            if (awaited.ops[label].op === op)
                return awaited.ops[label]
        }
        const base = op.owner + "/" + op.name
        let label = base
        for (let n = 2; awaited.ops[label] !== undefined; n++)
            label = base + "#" + n
        const record = { "op": op, "label": label, "gen": 0, "live": false, "stage": "start", "visited": [],
                         "marks": ({}), "saidIn": -1 }
        awaited.ops[label] = record
        return record
    }
    /// The words that place a line: `owner=… op=… gen=…`.
    function placeOf(record) {
        const cut = record.label.indexOf("/")
        return "owner=" + record.label.substring(0, cut) + " op=" + record.label.substring(cut + 1)
             + " gen=" + record.gen
    }

    /// A generation of `op` begins: started, or begun again by its own hand (`StageTimer.again`). Its stages, its
    /// marks and its order start over; the old generation can move nothing more.
    function begin(op) {
        const record = awaited.recordOf(op)
        record.gen++
        record.live = true
        record.stage = "start"
        record.visited = []
        record.marks = ({})
        record.saidIn = -1
        if (awaited.broken)
            return
        Harness.report("begin run=" + awaited.runId + " " + awaited.placeOf(record)
                       + (op.order.length === 0 ? "" : " order=" + op.order.join(","))
                       + (op.after.length === 0 ? "" : " after=" + op.after.join(",")))
        if (awaited.done)
            awaited.breach(op, "began after the completion")
    }
    /// `op` stopped: its generation is over.
    function end(op) {
        awaited.recordOf(op).live = false
    }
    /// A beat of `op` starts (`StageTimer`'s own handler, before the instance's).
    function beat(op) {
        awaited.beatSerial++
        awaited.beating = op
        awaited.observed = null
        Qt.callLater(awaited.betweenBeats)
    }
    function betweenBeats() {
        awaited.beating = null
        awaited.observed = null
    }

    function stageOf(op) {
        return awaited.recordOf(op).stage
    }
    function generationOf(op) {
        return awaited.recordOf(op).gen
    }

    /// Whether every stage `after` names (`owner/name/stage`) has been reached, by any generation. `count` is
    /// `stageCount`, passed so a binding reads this again when a stage is reached.
    function joined(after, count) {
        return after.every(path => awaited.reachedStages[path] === true)
    }

    /// Why `op` cannot move to `to` now, or "" when it can. `pressed`: the move is an input's, whose own answer — the
    /// product took it — is the reading it moves on (`StageTimer.input`).
    function refusal(op, record, to, pressed) {
        if (!record.live)
            return "moved to " + to + " in a generation that is over"
        if (awaited.beating !== op)
            return "moved to " + to + " outside its own beat"
        if (pressed !== true && awaited.observed !== true)
            return "moved to " + to + " with the beat's latest observation not holding"
        if (to === "complete" && record.stage === "complete")
            return "completed twice"
        if (record.visited.indexOf(to) >= 0)
            return "reached " + to + " twice in one generation"
        if (!awaited.joined(op.after, awaited.stageCount))
            return "moved to " + to + " before " + op.after.filter(p => awaited.reachedStages[p] !== true).join(",")
        const order = op.order
        if (order.length === 0)
            return ""
        if (to === "complete")
            return record.visited.length < order.length ? "completed before " + order[record.visited.length] : ""
        const at = order.indexOf(to)
        if (at < 0)
            return "moved to " + to + ", which its order does not name"
        if (at < record.visited.length)
            return "went back to " + to
        if (at > record.visited.length)
            return "skipped " + order[record.visited.length] + " on its way to " + to
        return ""
    }

    /// Whether `op` may not move to `to` now — said as the breach it is. Asked before an input is pressed, so a second
    /// input as the same stage never reaches the product.
    function refused(op, to, pressed) {
        if (awaited.broken)
            return true
        const why = awaited.done ? "moved to " + to + " after the completion"
                                 : awaited.refusal(op, awaited.recordOf(op), to, pressed)
        if (why === "")
            return false
        awaited.breach(op, why)
        return true
    }

    /// Moves `op` to `to`, says so, and says the report `report` reads now (a stage's own facts), if given. Answers
    /// whether it moved; a refusal is a breach.
    function transition(op, to, report, pressed) {
        if (awaited.refused(op, to, pressed))
            return false
        const record = awaited.recordOf(op)
        const lines = report === undefined ? [] : awaited.read(op, report)
        if (lines === null)
            return false
        awaited.stageSeq++
        Harness.report("stage run=" + awaited.runId + " " + awaited.placeOf(record) + " from=" + record.stage
                       + " to=" + to + " seq=" + awaited.stageSeq)
        record.visited = record.visited.concat([to])
        record.stage = to
        record.marks = ({})
        const cut = record.label.indexOf("#")
        const path = (cut < 0 ? record.label : record.label.substring(0, cut)) + "/" + to
        const reached = ({})
        for (const key in awaited.reachedStages)
            reached[key] = true
        reached[path] = true
        awaited.reachedStages = reached
        awaited.stageCount++
        awaited.say(record, lines)
        return true
    }

    /// The lines a report reads, or null when it could not be read (a breach). A report is a function, read when it
    /// is called; it answers a line or a list of them.
    function read(op, report) {
        if (typeof report !== "function") {
            awaited.breach(op, "handed a report already read")
            return null
        }
        try {
            const answer = report()
            if (answer === undefined || answer === null || answer === "")
                return []
            return Array.isArray(answer) ? answer.filter(line => line !== "") : [String(answer)]
        } catch (e) {
            awaited.breach(op, "a report that threw: " + e)
            return null
        }
    }
    function say(record, lines) {
        for (const line of lines)
            Harness.report(line)
        if (lines.length > 0)
            record.saidIn = awaited.beatSerial
    }

    /// Says a report of `op`'s stage as it stands, read now.
    function sayAt(op, report) {
        if (awaited.broken)
            return
        const record = awaited.recordOf(op)
        const lines = awaited.read(op, report)
        if (lines !== null)
            awaited.say(record, lines)
    }

    /// `op` completes: the transition to `complete`, then the route holds and reads `report`. `arriving` names the
    /// page's reads this verb completes over on purpose (`PageSettled.missing`'s terms) — its subject is a page still
    /// arriving — and only for this completion. A report said by a stage in the completing beat was read before the
    /// hold: a breach.
    function complete(op, report, arriving) {
        if (awaited.broken)
            return false
        const record = awaited.recordOf(op)
        if (typeof report !== "function") {
            awaited.breach(op, "completed with a report already read")
            return false
        }
        if (record.saidIn === awaited.beatSerial) {
            awaited.breach(op, "said a report in the beat it completed in, before the completion's hold")
            return false
        }
        if (!awaited.transition(op, "complete"))
            return false
        op.stop()
        awaited.route.finishFrom({ "op": op, "record": record, "gen": record.gen, "report": report,
                                   "arriving": arriving === undefined ? [] : arriving })
        return true
    }

    /// What the route asks before it reads a completion's report: the completing generation is still the one its
    /// operation stands in — a generation begun since makes it an old one's completion.
    function current(from) {
        return from.record.gen === from.gen
    }

    /// The marks `still` / `drawnSince` keep for `op`, which die with its stage and its generation.
    function marksOf(op) {
        return awaited.recordOf(op).marks
    }

    /// A breach of the stages: said once with where it happened, and the run ends — its pictures are taken, and the
    /// parent fails on the line (`verify::contract`).
    function breach(op, why) {
        if (awaited.broken)
            return
        awaited.broken = true
        const record = awaited.recordOf(op)
        Harness.report("stage_breach run=" + awaited.runId + " " + awaited.placeOf(record) + " stage=" + record.stage
                       + " why=" + why)
        if (awaited.route !== null && !awaited.done)
            awaited.route.finishFrom(null)
    }

    /// The operations still in a generation and where each stands, for the ceiling's line.
    function standing() {
        const out = []
        for (const label in awaited.ops) {
            const record = awaited.ops[label]
            if (record.live)
                out.push(label + "@" + record.gen + ":" + record.stage)
        }
        return out.length === 0 ? "none" : out.join(" ")
    }

    // ---- Completion and pictures ----

    /// The claims the pictures are of, by name: the predicate that has to hold, and the artifacts it is read at
    /// (`app` / `overlay` / `scene`; every one when none is named). Placed by a verb that completes on a state
    /// something else can pass (a card the reads could take down); read at each picture, never cleared — the
    /// claim is held for as long as the run has pictures to take.
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

    /// The page's reads the completing verb photographs still on their way (`complete`'s `arriving`; `*` for every
    /// read — a verb photographed during the write it presses), set by the completion and read by the pictures.
    property var arriving: []
    /// Whether the completion said it would photograph `term` still arriving.
    function expects(term) {
        return awaited.arriving.indexOf("*") >= 0 || awaited.arriving.indexOf(term) >= 0
    }

    /// The completion, said by the route in the turn it read the report (`CompletionRoute`): the verb, the
    /// operation and generation it came from, the stages said before it, and what the page still owed then
    /// (`PageSettled.missing`) — `page=settled`, or `page=arriving:` with the names, which the parent holds to the
    /// terms the completion declared (`expected=`). `from` is null for a run a breach ended.
    function completed(verb, from, missing) {
        const owed = missing === undefined ? [] : missing
        awaited.done = true
        awaited.arriving = from === null ? [] : from.arriving
        Harness.report("complete run=" + awaited.runId + " verb=" + verb
                       + " " + (from === null ? "owner=none op=none gen=0" : awaited.placeOf(from.record))
                       + " stages=" + awaited.stageSeq
                       + " page=" + (owed.length === 0 ? "settled" : "arriving:" + owed.join(","))
                       + (awaited.arriving.length === 0 ? "" : " expected=" + awaited.arriving.join(",")))
    }
}
