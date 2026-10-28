import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Layouts
import QtTest
import platitude.ui
import "../../src/auto" as Automation

// A log raised over rows already in its model, the way the page raises it (`RepoPage.commandsSeat`: a seat in a
// split, built when raised, the list in a column): in the turn it is raised the list has its rows and no size, and
// builds none — `built` answers for it, as for a list the layout left with nothing — until the polish that lays it
// out, which sizes it and builds its rows in one pass. So the page names it apart (`PageSettled.unsized`), for the
// completion to wait a polish pass over (`CompletionRoute`).
Item {
    id: root

    width: 400
    height: 300

    readonly property QtObject graphState: QtObject {
        property int finishCount: 1
        property bool loading: false
        property bool failed: false
        property bool stale: false
        property bool wipRow: false
        function readingsSettled() { return true }
    }

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
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: 20
                            }
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

    TestCase {
        name: "Unsized"
        when: windowShown

        function test_a_list_raised_over_its_rows_is_unsized_until_the_pass_that_lays_it_out() {
            compare(Automation.PageSettled.unsized(logPage), [])
            logPage.raised = true
            const list = seat.item.list
            compare(list.count, 3)
            compare(list.height, 0)
            compare(list.indexAt(1, 1), -1, "no row built")
            verify(Automation.Awaited.built(list), "a list with no size reads as built")
            verify(Automation.PageSettled.settled(logPage), "and owes the page nothing")
            const unsized = Automation.PageSettled.unsized(logPage)
            compare(unsized.length, 1)
            verify(unsized[0].endsWith("AppListView"), unsized[0])
            verify(waitForRendering(root))
            verify(list.height > 0)
            verify(list.indexAt(1, 1) >= 0, "the pass that sized it built its rows")
            compare(Automation.PageSettled.unsized(logPage), [])
            verify(Automation.PageSettled.settled(logPage))
        }
    }
}
