import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import QtTest
import platitude
import platitude.ui
import "stages.js" as Stages
import "../../src/auto" as Automation

// A log raised over rows already in its model, the way the page raises it (`RepoPage.commandsSeat`), in a window that
// cannot render yet: the list has its rows and no size, and its layout is held — no polish pass runs. The completion
// holds through any number of beats and any frame notice said after the change, and stands once the window shows and
// a pass lays the list out: on the list sized, with its rows built (`CompletionRoute`, `Awaited.polishes`,
// `PageSettled.unsized`).
Item {
    id: root

    width: 100
    height: 100

    property int reportReads: 0
    property int beats: 0
    /// The list as the completion read it: its height, and the row under its top-left corner (-1 for none built).
    property var atRead: null

    readonly property QtObject graphState: QtObject {
        property int finishCount: 1
        property bool loading: false
        property bool failed: false
        property bool stale: false
        property bool wipRow: false
        function readingsSettled() { return true }
    }

    Window {
        id: stage
        width: 400
        height: 300
        visible: false

        Item {
            id: logPage
            anchors.fill: parent
            property var pageTab: ({state: "open", settledWrites: () => []})
            property var pageCommands: ({recorded: () => true})
            property var pageWorktree: ({loaded: true, wipRowStands: false})
            property bool pageRefsLoaded: true
            property bool pageDetailsSettled: true
            property bool pageCarriedSettled: true
            property bool pageLanding: false
            property var pageGraphPane: ({placing: false})
            property var pageGraph: root.graphState
            property bool raised: false
            ListModel {
                id: logRows
                ListElement { a: 1 }
                ListElement { a: 2 }
                ListElement { a: 3 }
            }
            ColumnLayout {
                anchors.fill: parent
                spacing: 0
                SplitView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    orientation: Qt.Vertical
                    Item {
                        SplitView.fillHeight: true
                    }
                    Loader {
                        id: seat
                        visible: logPage.raised
                        active: logPage.raised
                        SplitView.preferredHeight: 120
                        SplitView.minimumHeight: 40
                        sourceComponent: Rectangle {
                            property alias list: logList
                            ColumnLayout {
                                anchors.fill: parent
                                spacing: 0
                                AppListView {
                                    id: logList
                                    Layout.fillWidth: true
                                    Layout.fillHeight: true
                                    model: logRows
                                    delegate: Item {
                                        width: 100
                                        height: 20
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Automation.CompletionRoute {
        id: route
        owed: () => Automation.PageSettled.missing(logPage)
        unsized: () => Automation.PageSettled.unsized(logPage)
        onReadied: {
            const list = seat.item.list
            root.atRead = { "height": list.height, "row": list.indexAt(1, Math.max(0, list.contentY) + 1) }
            Automation.Awaited.completed("stages", route.completing, route.owedAtRead, route.witnessesRead)
        }
    }
    Component.onCompleted: {
        Automation.Awaited.route = route
        Automation.Awaited.window = stage
    }

    Automation.PageStage {
        id: op
        name: "sized"
        onTriggered: {
            root.beats = root.beats + 1
            if (!op.at("start"))
                return
            op.complete({ "name": "sized_ready", "when": () => ({ "ready": true }),
                          "report": () => {
                              root.reportReads = root.reportReads + 1
                              return "sized read"
                          } })
        }
    }
    /// Beats while the operation stands completed: what "held this long" is counted on.
    Automation.PageStage {
        id: clock
        name: "clock"
        running: true
        onTriggered: root.beats = root.beats + 1
    }

    TestCase {
        name: "StageSized"
        when: windowShown

        function test_a_list_held_from_its_layout_holds_the_completion_until_a_polish_pass_takes_it_in() {
            logPage.raised = true
            const list = seat.item.list
            compare(list.count, 3)
            compare(list.height, 0)
            op.start()
            tryVerify(() => op.at("complete"))
            const from = root.beats
            tryVerify(() => root.beats >= from + 6)
            verify(!route.read)
            compare(root.reportReads, 0)
            verify(Harness.said.some(line => line.startsWith("completion waiting=sized:")), Harness.said.join("\n"))
            // Frame notices said after the change — a frame that began before it, its swap arriving late — are no pass.
            stage.frameSwapped()
            Automation.Awaited.frames = Automation.Awaited.frames + 3
            const noticed = root.beats
            tryVerify(() => root.beats >= noticed + 4)
            verify(!route.read)
            compare(list.height, 0, "no pass has run")
            stage.visible = true
            tryVerify(() => route.read)
            verify(root.atRead.height > 0, "read on the list as the pass sized it: " + root.atRead.height)
            verify(root.atRead.row >= 0, "with its rows built")
            compare(root.reportReads, 1)
            compare(Stages.breaches(Harness.said), [])
        }
    }
}
