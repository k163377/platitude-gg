import QtQuick
import QtTest
import platitude
import "../../src/auto" as Automation

// A page read changes an already sized layout. Hold the window's polish, not elapsed time, to expose a report
// taken after the read but before its layout. A late swap notification cannot release the report.
Item {
    id: root
    width: 100
    height: 100
    property int reportedX: -1

    Window {
        id: stage
        width: 200
        height: 100
        visible: true
        Row {
            Rectangle { id: first; width: 20; height: 20 }
            Rectangle { id: second; width: 20; height: 20 }
        }
    }
    StageRig { id: rig; owing: ["details"] }
    Automation.PageStage {
        id: op
        name: "arrival_layout"
        onTriggered: op.complete({ "name": "arrival_layout_ready", "when": () => ({ "ready": true }),
                                   "report": () => {
                                       root.reportedX = second.x
                                       return "arrival_layout x=" + second.x
                                   } })
    }
    TestCase {
        name: "StageArrivalLayout"
        when: windowShown
        function test_an_arrival_is_laid_out_before_the_report_reads_it() {
            Automation.Awaited.window = stage
            tryCompare(second, "x", 20)
            op.start()
            tryVerify(() => rig.route.asked)
            stage.visible = false
            first.width = 80
            rig.owing = []
            rig.route.attempt()
            compare(second.x, 20, "the new placement still needs a polish")
            compare(root.reportedX, -1, "no report on the old placement")
            stage.frameSwapped()
            rig.route.attempt()
            verify(!rig.route.read)
            stage.visible = true
            tryVerify(() => rig.route.read)
            compare(root.reportedX, 80)
            stage.close()
        }
    }
}
