import QtQuick
import platitude.ui

/// One operation of a run: a sampler whose stages move only through here (`Awaited` — 反映前テストの機械化 §検証時点).
/// Each start is a generation, with stages, marks, witnesses and an order of its own; `again` begins the next by hand,
/// for an operation that repeats on purpose. Its beat reads the product (`Awaited.all`) as often as it likes; it moves
/// a stage (`advance`) or completes the run (`complete`) on the yes of the beat's latest reading, and puts an input
/// in (`input`) on the press's own answer, once per stage per generation. Not instantiated as itself: `PageStage`
/// and `WindowStage` say whose operation it is.
///
/// Each function says where it may be called (`// reach(…)`, `cargo xtask waits`): in the operation's beat, in the
/// press of an input, or in a reading — a report, a completion's `when` — which only reads.
SampleTimer {
    id: op

    /// Whose operation this is — `page` or `window` — fixed by the two types that are this one.
    property string owner: ""
    /// The operation's name in its lines: unique to its owner within a run.
    required property string name
    /// The stages a generation goes through, in order, when there are more than `complete`: a stage out of this
    /// order, a stage skipped and a completion short of the last are breaches. Empty for an operation whose one
    /// transition is its completion or whose stages are its own to name.
    property var order: []
    /// The other operations' stages this one moves after, as `owner/name/stage`: what it needs from them, named, so
    /// that nothing else orders it against them (`joined`).
    property var after: []

    onRunningChanged: {
        if (op.running)
            Awaited.begin(op)
        else
            Awaited.end(op)
    }
    // Runs before the instance's own handler: the beat is this operation's from here on.
    onTriggered: Awaited.beat(op)

    /// Whether the generation stands at `stage` — `start` before its first transition.
    // reach(read)
    function at(stage) {
        return Awaited.stageOf(op) === stage
    }
    /// The generation running.
    // reach(read)
    function generation() {
        return Awaited.generationOf(op)
    }
    /// Whether the stages `after` names have been reached — the term to read beside the others before moving.
    // reach(read)
    function joined() {
        return Awaited.joined(op.after, Awaited.stageCount)
    }

    /// Moves to `stage`, saying what `report` reads now — the stage's own facts, said as its lines (`said`), which
    /// are the stage's history and never the run's claim. Answers whether it moved.
    // reach(beat; 1=read)
    function advance(stage, report) {
        return Awaited.transition(op, stage, report)
    }
    /// Puts an input in, once per generation, as the move to `stage`: `press` is the product's own input path, and
    /// answers false where the input did not go in (a row not there yet) — then nothing moved, nothing the press
    /// witnessed stands, and the beat after tries again. Answering nothing is an input that went in. `report`, if
    /// given, is read once the input went in, with what `press` answered, as the stage's lines. A second input as the
    /// same stage is refused before it is pressed. A fact the completion's report needs from the press is a witness
    /// (`witness`), not a value carried out of it.
    // reach(beat; 1=press, 2=read)
    function input(stage, press, report) {
        if (Awaited.refused(op, stage, true))
            return false
        Awaited.pressing(op, stage)
        let answer
        try {
            answer = press()
        } catch (e) {
            // Part of the input may have gone in: pressed again on the next beat it would go in twice.
            Awaited.pressed(op, true)
            Awaited.breach(op, "a press that threw: " + e)
            return false
        }
        Awaited.pressed(op, answer !== false)
        if (answer === false)
            return false
        return Awaited.transition(op, stage, report === undefined ? undefined : () => report(answer), true)
    }
    /// Says what `report` reads now, as lines of the stage the generation stands at (`said`).
    // reach(beat; 0=read)
    function say(report) {
        Awaited.sayAt(op, report)
    }
    /// Says a diagnostic line of this operation's when it changes, as a line of its stage (`said`): what it waits
    /// on, for the reader of a run that stood.
    // reach(beat)
    function note(line) {
        Awaited.noteAt(op, line)
    }
    /// Keeps `value` as a fact this generation observed now — before its input, in its press, or after it — under
    /// `name`, with the stage it was observed at (`witness` line). The completion's report reads it back
    /// (`witnessed`), and names it in the completion's line: a past fact, said as past, beside the state read at
    /// the completion. Once per name per generation. Answers `value`.
    // reach(beat, press)
    function witness(name, value) {
        return Awaited.witness(op, name, value)
    }
    /// The fact this generation kept under `name` (`witness`); a reading of a name never kept throws, which a report
    /// reading it turns into a breach.
    // reach(read)
    function witnessed(name) {
        return Awaited.witnessed(op, name)
    }
    /// Whether this generation has kept a fact under `name` — an edge an answer keeps when it comes (a frame's grab
    /// answering in a later turn), read as the edge it is.
    // reach(read)
    function kept(name) {
        return Awaited.kept(op, name)
    }
    /// Completes the run from here once `spec.when` holds — `{ when, report, arriving, lost, name }`:
    ///
    /// - `when`: the terms the completion stands on (`{ name: holds }`), read now — the move happens on their yes —
    ///   and again by the completion's hold each time it reads, until the report is read: a condition the verb needs
    ///   at its completion, not only before it.
    /// - `report`: read by the completion's route in the turn its reading holds — `when`, the page's reads (but the
    ///   `arriving` ones) and the page's own holds all at once — and checked against a second reading in that turn;
    ///   the run's claim, from the state that reading confirmed, and the witnesses the generation kept.
    /// - `arriving`: the page's reads this verb photographs on their way (`PageSettled.missing`'s names; `*` for
    ///   every read).
    /// - `lost`: what a `when` term falling during the hold is: `wait` (the default) holds on and reads again until
    ///   it is back — the input is never pressed again — and `fail` ends the run as a breach.
    /// - `name`: the observation's name in the lines, `<operation>_done` if not given.
    ///
    /// Answers whether it moved; stops this operation when it did.
    // reach(beat; 0=spec)
    function complete(spec) {
        return Awaited.complete(op, spec)
    }
    /// Begins the next generation by hand: the stages, marks, witnesses and order start over.
    // reach(beat)
    function again() {
        Awaited.begin(op)
    }

    /// Whether `value` has stood across a polish pass of the window begun after it was read (`Awaited.layoutMark`):
    /// true once the value read now is the one read before such a pass — a layout still moving across it would have
    /// changed it, and a value laid out to nothing stands as well as any. In place of "two samples agree", whose two
    /// samples can both fall before one layout pass, and of a frame said swapped, which can be one begun before the
    /// read. Kept for this stage of this generation only: a value that stood before the stage moved says nothing of
    /// the next.
    // reach(beat)
    function still(name, value) {
        const marks = Awaited.marksOf(op)
        const mark = marks[name]
        if (mark === undefined || mark.value !== value) {
            marks[name] = { "value": value, "polish": Awaited.layoutMark() }
            Awaited.askLayout()
            return false
        }
        if (!Awaited.layoutPassedSince(mark.polish)) {
            Awaited.askLayout()
            return false
        }
        return true
    }
    /// Whether a polish pass of the window has begun since `name` was first asked about in this stage: a layout asked
    /// for before the ask (a card opened, a group laid out) whose shape has no property of its own to read is laid out
    /// by then. `name` names what was asked for — ask again under another name for another thing. A layout, not a
    /// picture: what is painted is the grab's to say (`grabToImage`).
    // reach(beat)
    function laidOutSince(name) {
        const marks = Awaited.marksOf(op)
        let mark = marks[name]
        if (mark === undefined) {
            mark = { "polish": Awaited.layoutMark() }
            marks[name] = mark
        }
        if (!Awaited.layoutPassedSince(mark.polish)) {
            Awaited.askLayout()
            return false
        }
        return true
    }
}
