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
/// - **Completion** (`StageTimer.complete`): the run's one ending, on evidence of three kinds kept apart. A fact
///   observed before the input, in its press or after it is a witness (`witness`), kept with its operation,
///   generation and stage and said so (`witness run= …`). A condition the verb needs at its completion (`when`) is
///   read when the operation moves and again each time the completion's hold reads. The state the run claims is
///   the report, read by the completion route (`CompletionRoute`) in the turn its reading holds — the verb's
///   conditions, the page's reads but those declared arriving, the page's own holds — and checked against a
///   second reading in that same turn; then the scene is walked, and `complete run= …` says the witnesses the
///   report read beside the state it confirmed.
///
/// What a verb says at a stage (`advance` / `input` / `say`) is said as that stage's (`said run= …: <line>`): the
/// run's history, never its claim — a claim is the completion's report.
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

    /// The window the run draws in, and how many frames it has swapped, fed by `WindowHarness`: what the pictures wait
    /// a frame on (`WindowHarness.settle`) and the ceiling's line counts — no boundary of a layout's, since on a
    /// threaded render loop a swap reaches this thread after the GUI thread may have moved on (`polishes`).
    property var window: null
    property int frames: 0
    /// How many polish passes the window has made. Every render loop polishes the window's items and says
    /// `afterAnimating` in one call on this thread, before the scene graph syncs
    /// (`QSGThreadedRenderLoop::polishAndSync`, `QSGGuiThreadRenderLoop` / `QSGSoftwareRenderLoop::renderWindow`): a
    /// pass counted after a reading began after it, and laid out every item asked of it before then. A swapped frame
    /// says no such thing — on a threaded loop it comes from the render thread, after the GUI thread may have moved
    /// on. A window that cannot render makes no pass.
    property int polishes: 0
    readonly property Connections polishing: Connections {
        target: awaited.window
        function onAfterAnimating() {
            awaited.polishes++
        }
    }

    /// The one boundary every layout wait stands on (`StageTimer.still` / `laidOutSince`, `CompletionRoute`): a mark
    /// of the passes so far, taken where a wait first sees what it waits on, and whether a pass has begun since — one
    /// that laid out every item asked of it before the mark, sizing it or leaving it with none. Not the end of work of
    /// another kind — a read on its way, a font being shaped — which a wait reads off that work's own answer.
    // reach(read)
    function layoutMark() {
        return awaited.polishes
    }
    // reach(read)
    function layoutPassedSince(mark) {
        return awaited.polishes > mark
    }
    /// Asks for the pass a layout wait still short waits on: a still scene runs none unasked; a window that cannot
    /// render runs none at all, and the wait holds.
    // reach(beat)
    function askLayout() {
        if (awaited.window !== null)
            awaited.window.update()
    }

    /// The completion route (`CompletionRoute`), which holds a completion until its reads are in and then reads its
    /// report (`finishFrom`). Set by the harness as it is built.
    property var route: null
    /// The page's own holds on a completion — its menu still on its way out (`AutoActDriver.completionHolds`) —
    /// handed in by the page that claimed the run (`holdPage`).
    property var pageHolds: null
    // reach(beat)
    function holdPage(holds) {
        awaited.pageHolds = holds
    }

    /// Whether a list (`ListView`) has built the rows it shows. A model's rows are there the moment they land; their
    /// delegates come in the polish after (`QQuickItemView::refill`, which fills the viewport in one pass — the cache
    /// buffer beyond it a frame at a time, off screen). So the row under the viewport's top-left corner stands for the
    /// whole viewport, and a verb that reads rows (`itemAtIndex`) or completes on a list reads this first: read in
    /// the beat the rows landed, the list is empty of delegates and the picture is not. A list with no rows has
    /// nothing to build; one with no size shows nothing — or has not been laid out yet, which the completion waits a
    /// polish pass over (`PageSettled.unsized`).
    // reach(read)
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
    /// An observation that did not hold, with the terms it is waiting for — what a fault that holds a read back lets
    /// it through on (`WindowHarness`, `PGG_FAULT_HOLD_RECORDS`).
    signal waited(string name, var missing)

    /// Whether every term has come. `terms` maps a name to whether it is there; the missing names are said under
    /// `name` when they change. An observation only: it moves no stage, however often it is read. `name` is a name —
    /// a line of words under it would be a line the run says outside its report (`complete`).
    // reach(beat)
    function all(name, terms) {
        if (/[\s=]/.test(name) && awaited.beating !== null) {
            awaited.breach(awaited.beating, "observed under a name that is no name: " + name)
            return false
        }
        const missing = []
        for (const term in terms) {
            if (!terms[term])
                missing.push(term)
        }
        awaited.note(name + " waiting=" + (missing.length === 0 ? "nothing" : missing.join(",")))
        const holds = missing.length === 0
        awaited.observed = holds
        if (!holds)
            awaited.waited(name, missing)
        return holds
    }

    function note(line) {
        if (line === awaited.said)
            return
        awaited.said = line
        awaited.since = PerfProbe.clockMs()
        Harness.report(line)
    }
    /// A diagnostic of `op`'s, said as a line of its stage (`said`) when it changes (`StageTimer.note`).
    function noteAt(op, line) {
        if (awaited.broken)
            return
        const record = awaited.recordOf(op)
        if (record.noted === line)
            return
        record.noted = line
        awaited.say(record, [line])
    }

    // ---- Stages ----

    /// Each operation's record, by `owner/name` (a second instance under one name — a second page's driver — is
    /// `name#2`): `{ op, label, gen, live, stage, visited, marks, witnesses }`.
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
                         "marks": ({}), "witnesses": ({}), "noted": "" }
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
        record.witnesses = ({})
        record.noted = ""
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
    /// is called; it answers a line or a list of them. The witnesses it reads are gathered into `readWitnesses`.
    function read(op, report) {
        if (typeof report !== "function") {
            awaited.breach(op, "handed a report already read")
            return null
        }
        awaited.readWitnesses = []
        awaited.reading = op
        try {
            const answer = report()
            if (answer === undefined || answer === null || answer === "")
                return []
            return Array.isArray(answer) ? answer.filter(line => line !== "") : [String(answer)]
        } catch (e) {
            awaited.breach(op, "a report that threw: " + e)
            return null
        } finally {
            awaited.reading = null
        }
    }
    /// The words that place a line at its stage: `owner=… op=… gen=… stage=…`.
    function stagePlaceOf(record) {
        return awaited.placeOf(record) + " stage=" + (awaited.pressingOp === record.op ? awaited.pressingTo
                                                                                       : record.stage)
    }
    /// Says `lines` as the history of `record`'s stage: `said run=… stage=…: <line>`.
    function say(record, lines) {
        for (const line of lines)
            Harness.report("said run=" + awaited.runId + " " + awaited.stagePlaceOf(record) + ": " + line)
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

    // ---- Witnesses ----

    /// The operation whose input is being pressed (`StageTimer.input`), and the stage it presses as: a witness kept
    /// in the press is that stage's, and stands only once the press went in (`pressed`).
    property var pressingOp: null
    property string pressingTo: ""
    /// The names the press under way kept, and their `witness` lines, said once it went in.
    property var pressKept: []
    property var pressSaid: []
    function pressing(op, stage) {
        awaited.pressingOp = op
        awaited.pressingTo = stage
        awaited.pressKept = []
        awaited.pressSaid = []
    }
    /// Ends `op`'s press: one that went in says the witnesses it kept; one that put nothing in drops them, so the
    /// press tried again keeps its own.
    function pressed(op, wentIn) {
        if (wentIn) {
            for (const line of awaited.pressSaid)
                Harness.report(line)
        } else {
            const record = awaited.recordOf(op)
            for (const name of awaited.pressKept)
                delete record.witnesses[name]
        }
        awaited.pressingOp = null
        awaited.pressingTo = ""
        awaited.pressKept = []
        awaited.pressSaid = []
    }
    /// The operation whose report is being read, and the witnesses that report read, as
    /// `<owner>/<op>/<gen>/<stage>/<name>` — the words of the `witness` line each was kept under.
    property var reading: null
    property var readWitnesses: []

    /// Keeps `value` under `name` as a fact `op`'s generation observed now, at the stage it stands at (the press's
    /// stage inside a press), and says so. A generation keeps a name once; one that is over keeps nothing.
    function witness(op, name, value) {
        if (awaited.broken)
            return value
        const record = awaited.recordOf(op)
        if (!record.live) {
            awaited.breach(op, "witnessed " + name + " in a generation that is over")
            return value
        }
        if (record.witnesses[name] !== undefined) {
            awaited.breach(op, "witnessed " + name + " twice in one generation")
            return value
        }
        const inPress = awaited.pressingOp === op
        const stage = inPress ? awaited.pressingTo : record.stage
        record.witnesses[name] = { "value": value, "stage": stage }
        const line = "witness run=" + awaited.runId + " " + awaited.placeOf(record) + " stage=" + stage
                     + " name=" + name
        if (inPress) {
            awaited.pressKept = awaited.pressKept.concat([name])
            awaited.pressSaid = awaited.pressSaid.concat([line])
        } else {
            Harness.report(line)
        }
        return value
    }
    /// Whether `op`'s running generation kept a value under `name`.
    function kept(op, name) {
        return awaited.recordOf(op).witnesses[name] !== undefined
    }
    /// The value `op`'s running generation kept under `name`. A name never kept throws.
    function witnessed(op, name) {
        const record = awaited.recordOf(op)
        const kept = record.witnesses[name]
        if (kept === undefined)
            throw new Error("no witness " + name + " in generation " + record.gen + " of " + record.label)
        if (awaited.reading !== null) {
            const read = record.label + "/" + record.gen + "/" + kept.stage + "/" + name
            if (awaited.readWitnesses.indexOf(read) < 0)
                awaited.readWitnesses = awaited.readWitnesses.concat([read])
        }
        return kept.value
    }

    // ---- Completion ----

    /// `op` completes once `spec.when` holds (`StageTimer.complete`): read here as the beat's observation, then the
    /// transition to `complete`, and the route holds — reading `when` again each time — until it reads `report`.
    /// Answers whether it moved; a spec that is no spec is a breach.
    function complete(op, spec) {
        if (awaited.broken)
            return false
        const record = awaited.recordOf(op)
        if (spec === null || typeof spec !== "object" || typeof spec.when !== "function"
                || typeof spec.report !== "function") {
            awaited.breach(op, "completed without a `when` and a `report` to read at the completion")
            return false
        }
        const lost = spec.lost === undefined ? "wait" : spec.lost
        if (lost !== "wait" && lost !== "fail") {
            awaited.breach(op, "completed with a `lost` of " + lost)
            return false
        }
        const name = spec.name === undefined ? record.label.split("/")[1].split("#")[0] + "_done" : spec.name
        let terms
        try {
            terms = spec.when()
        } catch (e) {
            awaited.breach(op, "a completion's `when` that threw: " + e)
            return false
        }
        if (!awaited.all(name, terms))
            return false
        if (!awaited.transition(op, "complete"))
            return false
        op.stop()
        awaited.route.finishFrom({ "op": op, "record": record, "gen": record.gen, "name": name, "when": spec.when,
                                   "report": spec.report, "lost": lost,
                                   "arriving": spec.arriving === undefined ? [] : spec.arriving })
        return true
    }

    /// What the route asks before it reads a completion's report: the completing generation is still the one its
    /// operation stands in — a generation begun since makes it an old one's completion.
    function current(from) {
        return from.record.gen === from.gen
    }

    /// The marks `still` / `laidOutSince` keep for `op`, which die with its stage and its generation.
    function marksOf(op) {
        return awaited.recordOf(op).marks
    }

    /// A breach of the stages: said once with where it happened, and the run ends — its pictures are taken, and the
    /// parent fails on the line (`verify::contract`).
    // reach(beat, press)
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

    // reach(beat, press; 1=read)
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
    /// operation and generation it came from, the stages said before it, what the page still owed at the reading
    /// the report was read on (`PageSettled.missing`) — `page=settled`, or `page=arriving:` with the names, which
    /// the parent holds to the terms the completion declared (`expected=`) — and the witnesses that report read
    /// (`witnesses=<owner>/<op>/<gen>/<stage>/<name>,…`). `from` is null for a run a breach ended.
    function completed(verb, from, missing, witnesses) {
        const owed = missing === undefined ? [] : missing
        const read = witnesses === undefined ? [] : witnesses
        awaited.done = true
        awaited.arriving = from === null ? [] : from.arriving
        Harness.report("complete run=" + awaited.runId + " verb=" + verb
                       + " " + (from === null ? "owner=none op=none gen=0" : awaited.placeOf(from.record))
                       + " stages=" + awaited.stageSeq
                       + " page=" + (owed.length === 0 ? "settled" : "arriving:" + owed.join(","))
                       + (awaited.arriving.length === 0 ? "" : " expected=" + awaited.arriving.join(","))
                       + (read.length === 0 ? "" : " witnesses=" + read.join(",")))
    }
}
