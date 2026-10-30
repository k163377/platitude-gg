import QtQuick
import QtTest
import platitude
import "../../src/auto" as Automation

// An accepted input can invalidate an already sized layout without starting a page read.
Item {
    id: root
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
    StageRig { id: rig }
    Automation.PageStage {
        id: op
        name: "initial_layout"
        order: ["changed"]
        onTriggered: {
            op.input("changed", () => { first.width = 80 })
            op.complete({ "name": "initial_layout_ready", "when": () => ({ "accepted": first.width === 80 }),
                            "report": () => {
                                root.reportedX = second.x
                                return "initial_layout x=" + second.x
                            } })
        }
    }
    TestCase {
        name: "StageInitialLayout"
        when: windowShown
        function test_an_input_is_laid_out_even_when_no_read_was_pending() {
            Automation.Awaited.window = stage
            tryCompare(second, "x", 20)
            stage.visible = false
            op.start()
            tryVerify(() => rig.route.asked)
            compare(second.x, 20, "the new placement still needs a polish")
            compare(root.reportedX, -1, "an immediately true condition does not settle the layout")
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
