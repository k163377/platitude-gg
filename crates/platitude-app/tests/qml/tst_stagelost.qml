import QtQuick
import QtTest
import platitude
import "stages.js" as Stages
import "../../src/auto" as Automation

// The verb's own condition falls while the completion is held for the page: the route does not read the report on a
// state the verb no longer stands on, and does not press the input again — it reads `when` again until the condition
// is back (`lost: "wait"`, the default). A route that let the page's reads alone release it would say `ok=false`; one
// that went back to the input would press twice.
Item {
    id: root

    width: 100
    height: 100

    property bool ok: false
    property int presses: 0
    property int reportReads: 0
    property int beats: 0

    StageRig {
        id: rig
        owing: ["details"]
    }
    Automation.PageStage {
        id: op
        name: "lost"
        order: ["pressed"]
        onTriggered: {
            if (op.at("start")) {
                op.input("pressed", () => {
                    root.presses = root.presses + 1
                    root.ok = true
                })
                return
            }
            op.complete({ "name": "lost_ready", "when": () => ({ "ok": root.ok }),
                          "report": () => {
                              root.reportReads = root.reportReads + 1
                              return "lost ok=" + root.ok
                          } })
        }
    }
    /// Beats while the completion is held: what "held this long" is counted on.
    Automation.PageStage {
        id: clock
        name: "clock"
        running: true
        onTriggered: root.beats = root.beats + 1
    }

    TestCase {
        name: "StageLost"
        when: windowShown

        function test_a_condition_lost_in_the_hold_is_waited_for_and_the_input_is_not_pressed_again() {
            op.start()
            tryVerify(() => op.at("complete"))
            root.ok = false
            rig.owing = []
            const from = root.beats
            tryVerify(() => root.beats >= from + 6)
            verify(!rig.route.read)
            compare(root.reportReads, 0)
            compare(root.presses, 1)
            root.ok = true
            tryVerify(() => rig.route.read)
            compare(Harness.said.filter(line => line.startsWith("lost ok=")), ["lost ok=true"])
            compare(root.presses, 1)
            compare(Stages.breaches(Harness.said), [])
        }
    }
}
