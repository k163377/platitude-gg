import QtQuick
import QtTest
import platitude
import "stages.js" as Stages
import "../../src/auto" as Automation

// A verb that said a condition of its completion cannot come back (`lost: "fail"`) ends the run as a breach the moment
// the hold reads it fallen — the report unread, the input not pressed again.
Item {
    id: root

    width: 100
    height: 100

    property bool ok: false
    property int presses: 0
    property int reportReads: 0

    StageRig {
        id: rig
        owing: ["details"]
    }
    Automation.PageStage {
        id: op
        name: "lost_fail"
        order: ["pressed"]
        onTriggered: {
            if (op.at("start")) {
                op.input("pressed", () => {
                    root.presses = root.presses + 1
                    root.ok = true
                })
                return
            }
            op.complete({ "name": "lost_fail_ready", "when": () => ({ "ok": root.ok }), "lost": "fail",
                          "report": () => {
                              root.reportReads = root.reportReads + 1
                              return "lost_fail ok=" + root.ok
                          } })
        }
    }

    TestCase {
        name: "StageLostFail"
        when: windowShown

        function test_a_condition_lost_in_the_hold_ends_the_run_where_the_verb_said_so() {
            op.start()
            tryVerify(() => op.at("complete"))
            root.ok = false
            tryVerify(() => rig.route.read)
            compare(root.reportReads, 0)
            compare(root.presses, 1)
            const breaches = Stages.breaches(Harness.said)
            compare(breaches.length, 1)
            verify(breaches[0].endsWith("why=lost ok during the completion's hold"), breaches[0])
        }
    }
}
