import QtQuick
import QtTest
import platitude
import "stages.js" as Stages
import "../../src/auto" as Automation

// The harness's operations with their order held by the test (反映前テストの機械化 §検証時点): the real `Awaited`,
// `StageTimer` and `CompletionRoute`, over terms the test releases one at a time — so which side moves first, a term
// that comes and goes, another operation reading the product meanwhile, and a read the completion waits on are each
// put in place by hand, not left to a machine's pace. What each operation reached is read back off the lines the
// machinery said, as `xtask::verify::contract` reads a run's.
Item {
    id: root

    width: 100
    height: 100

    // What the two operations observe, released by the test in the order a case wants.
    property bool pageReady: false
    property bool pageResult: false
    property bool windowReady: false
    property bool windowResult: false
    /// A second term of the window's result, held false while a case takes the first away and gives it back.
    property bool windowConfirmed: true
    /// The page's reads the completion's hold waits on, and the state the completion's report reads.
    property var owing: []
    property string subject: "before"
    property int reportReads: 0
    /// How many lines had been said when the route read the report (`CompletionRoute.readied`).
    property int readiedAt: -1
    /// Beats of an operation that only observes: what "so many beats and nothing moved" is counted on.
    property int observerBeats: 0

    Automation.CompletionRoute {
        id: route
        owed: () => root.owing
        onReadied: {
            root.readiedAt = Harness.said.length
            Automation.Awaited.completed("stages", route.completing, route.owedAtRead)
        }
    }

    /// The page's operation: ready, one press, its result.
    Automation.PageStage {
        id: pageOp
        name: "page_op"
        order: ["pressed", "read"]
        property int presses: 0
        onTriggered: {
            if (pageOp.at("start")) {
                if (!Automation.Awaited.all("page_ready", { "ready": root.pageReady }))
                    return
                pageOp.input("pressed", () => { pageOp.presses = pageOp.presses + 1 })
                return
            }
            if (!Automation.Awaited.all("page_result", { "result": root.pageResult }))
                return
            pageOp.advance("read")
            pageOp.stop()
        }
    }
    /// The window's, alike, and independent of the page's: no join orders them.
    Automation.WindowStage {
        id: windowOp
        name: "window_op"
        order: ["pressed", "read"]
        property int presses: 0
        onTriggered: {
            if (windowOp.at("start")) {
                if (!Automation.Awaited.all("window_ready", { "ready": root.windowReady }))
                    return
                windowOp.input("pressed", () => { windowOp.presses = windowOp.presses + 1 })
                return
            }
            if (!Automation.Awaited.all("window_result", { "result": root.windowResult,
                                                           "confirmed": root.windowConfirmed }))
                return
            windowOp.advance("read")
            windowOp.stop()
        }
    }
    /// An operation that only reads the product, its answer flipping every beat: it moves nothing.
    Automation.PageStage {
        id: observer
        name: "observer"
        running: true
        onTriggered: {
            root.observerBeats = root.observerBeats + 1
            Automation.Awaited.all("observer", { "even": root.observerBeats % 2 === 0 })
        }
    }
    /// An operation that keeps layout marks across one stage move.
    property bool marksMove: false
    Automation.PageStage {
        id: marksOp
        name: "marks"
        order: ["moved"]
        onTriggered: {
            if (!marksOp.at("start") || !Automation.Awaited.all("marks_move", { "move": root.marksMove }))
                return
            marksOp.advance("moved")
        }
    }
    /// The operation that completes the run, on a report handed over unread.
    Automation.PageStage {
        id: completer
        name: "completer"
        onTriggered: {
            if (!Automation.Awaited.all("completer_ready", { "ready": true }))
                return
            completer.complete(() => {
                root.reportReads = root.reportReads + 1
                return "stages subject=" + root.subject
            })
        }
    }

    TestCase {
        name: "Stages"
        when: windowShown

        function initTestCase() {
            Automation.Awaited.route = route
            Automation.Awaited.window = root.Window.window
        }

        /// Both operations to a new generation, every term taken back.
        function again() {
            root.pageReady = false
            root.pageResult = false
            root.windowReady = false
            root.windowResult = false
            root.windowConfirmed = true
            // Stopped first: a start on a running timer begins nothing.
            pageOp.stop()
            windowOp.stop()
            pageOp.start()
            windowOp.start()
        }
        function beatsLater(count) {
            const from = root.observerBeats
            tryVerify(() => root.observerBeats >= from + count)
        }
        function windowThrough() {
            root.windowReady = true
            tryVerify(() => windowOp.at("pressed"))
            root.windowResult = true
            tryVerify(() => !windowOp.running)
        }
        function pageThrough() {
            root.pageReady = true
            tryVerify(() => pageOp.at("pressed"))
            root.pageResult = true
            tryVerify(() => !pageOp.running)
        }

        /// The window first, the page first, and both released at once with their results already in: each
        /// operation's chain is the same, one press each, and no order is a breach — another operation reading the
        /// product beside them changes nothing.
        function test_1_independent_operations_move_alike_whichever_moves_first() {
            again()
            windowThrough()
            pageThrough()
            again()
            pageThrough()
            windowThrough()
            again()
            root.pageReady = true
            root.windowReady = true
            root.pageResult = true
            root.windowResult = true
            tryVerify(() => !pageOp.running && !windowOp.running)
            for (let gen = 1; gen <= 3; gen++) {
                compare(Stages.chain(Harness.said, "page", "page_op", gen), ["pressed", "read"], "page gen " + gen)
                compare(Stages.chain(Harness.said, "window", "window_op", gen), ["pressed", "read"],
                        "window gen " + gen)
            }
            compare(pageOp.presses, 3)
            compare(windowOp.presses, 3)
            compare(Stages.chain(Harness.said, "page", "observer", 1), [])
            compare(Stages.breaches(Harness.said), [])
        }

        /// A term that holds, falls and holds again moves nothing while the stage still waits on another: the stage
        /// moves once, when every term holds — an observation is no transition, however often it comes true.
        function test_2_a_term_that_comes_and_goes_moves_no_stage() {
            again()
            root.windowReady = true
            tryVerify(() => windowOp.at("pressed"))
            root.windowConfirmed = false
            const from = Harness.said.length
            root.windowResult = true
            beatsLater(4)
            root.windowResult = false
            beatsLater(4)
            root.windowResult = true
            beatsLater(4)
            verify(windowOp.at("pressed"))
            compare(Stages.stageLines(Harness.said, from), [])
            root.windowConfirmed = true
            tryVerify(() => !windowOp.running)
            compare(Stages.chain(Harness.said, "window", "window_op", 4), ["pressed", "read"])
            compare(Stages.breaches(Harness.said), [])
        }

        /// A press whose result is held back stands at its stage for as long as it is held, and moves once the result
        /// comes — the same chain as a result that was in at once.
        function test_3_a_result_held_back_holds_its_stage() {
            again()
            root.pageReady = true
            tryVerify(() => pageOp.at("pressed"))
            beatsLater(6)
            verify(pageOp.at("pressed"))
            compare(Stages.chain(Harness.said, "page", "page_op", 5), ["pressed"])
            root.pageResult = true
            tryVerify(() => !pageOp.running)
            compare(Stages.chain(Harness.said, "page", "page_op", 5), ["pressed", "read"])
            compare(pageOp.presses, 4)
            compare(Stages.breaches(Harness.said), [])
        }

        /// A layout mark is the stage's and the generation's: one that stood across a frame in one stage satisfies no
        /// wait of the next, nor of the next generation — each starts over from the frame it is first asked at. The
        /// frames are counted here by hand, the boundary `WindowHarness` feeds the machinery with.
        function test_4_a_layout_mark_dies_with_its_stage_and_its_generation() {
            marksOp.start()
            verify(!marksOp.still("width", 5))
            verify(!marksOp.drawnSince("card"))
            Automation.Awaited.frames = Automation.Awaited.frames + 2
            verify(marksOp.still("width", 5))
            verify(marksOp.drawnSince("card"))
            root.marksMove = true
            tryVerify(() => marksOp.at("moved"))
            verify(!marksOp.still("width", 5), "a mark of the stage before")
            verify(!marksOp.drawnSince("card"), "a frame count of the stage before")
            Automation.Awaited.frames = Automation.Awaited.frames + 2
            verify(marksOp.still("width", 5))
            marksOp.stop()
            marksOp.start()
            verify(!marksOp.still("width", 5), "a mark of the generation before")
            marksOp.stop()
        }

        /// The completion is asked while the page still owes a read: its report is not read, nothing is completed, and
        /// the state moves on meanwhile. Once the read is in, the report is read once — of the state then — and the run
        /// completes in that turn on a settled page.
        function test_9_a_completion_reads_its_report_only_once_the_reads_are_in() {
            root.owing = ["details"]
            root.subject = "before"
            completer.start()
            tryVerify(() => completer.at("complete"))
            beatsLater(4)
            verify(!route.read)
            compare(root.reportReads, 0)
            root.subject = "after"
            beatsLater(4)
            compare(root.reportReads, 0)
            verify(!Harness.said.some(line => line.startsWith("complete run=")))
            root.owing = []
            tryVerify(() => route.read)
            compare(root.reportReads, 1)
            compare(Harness.said[root.readiedAt - 1], "stages subject=after")
            const completed = Harness.said.filter(line => line.startsWith("complete run="))
            compare(completed.length, 1)
            compare(Stages.field(completed[0], "op"), "completer")
            compare(Stages.field(completed[0], "page"), "settled")
            compare(Stages.breaches(Harness.said), [])
        }
    }
}
