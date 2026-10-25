import QtQuick
import QtQuick.Layouts
import QtTest
import platitude.ui
import "../../src/auto" as Automation

Item {
    id: root
    width: 400
    height: 300
    property int wipPooled: 0

    Component {
        id: filePane
        StackLayout {
            property alias list: fileList
            ColumnLayout {
                AppListView {
                    id: fileList
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    model: 0
                    delegate: FileRowDelegate {}
                }
            }
            Item {}
        }
    }

    AppListView {
        id: files
        width: 200
        height: 40
        cacheBuffer: 0
        model: 0
        delegate: FileRowDelegate {}
    }

    AppListView {
        id: chosen
        y: 100
        width: 200
        height: 80
        model: 0
        delegate: ChosenCommitRow {}
    }

    AppListView {
        id: graph
        x: 200
        width: 200
        height: 40
        cacheBuffer: 0
        model: 0
        delegate: Item {
            required property int index
            width: graph.width
            height: 40
            ListView.onPooled: {
                if (index === 0)
                    root.wipPooled++
            }
            Loader {
                active: index === 0
                sourceComponent: WipTallyRow {
                    added: 1
                    modified: 0
                    deleted: 0
                    renamed: 0
                    copied: 0
                    conflicted: 0
                }
            }
        }
    }

    readonly property QtObject window: QtObject {
        id: window
        property Item contentItem: root
        property var curPage: null
        property int updateRequests: 0
        signal afterAnimating()
        function requestUpdate() { updateRequests++ }
    }

    readonly property QtObject page: QtObject {
        id: page
        property var pageTab: ({state: "open"})
        property var pageWorktree: ({loaded: true, wipRowStands: false})
        property bool pageRefsLoaded: true
        property bool pageDetailsSettled: true
        property bool pageCarriedSettled: true
        property bool pageLanding: false
        property var pageGraphPane: ({placing: false})
        property var pageGraph: graphState
    }
    readonly property QtObject graphState: QtObject {
        id: graphState
        property int finishCount: 3
        property bool loading: false
        property bool failed: false
        property bool stale: false
        property bool wipRow: false
        property var readings: ({done: true})
        function readingsSettled() { return readings.done }
    }

    Automation.WindowCensus {
        id: census
        window: window
    }

    AppMenu {
        id: submenu
        AppMenuItem { text: "Row" }
    }

    // An item holding a part by name, as the product holds its tips and hover seats: in no `data` of its.
    Item {
        id: holder
        width: 10
        height: 10
        readonly property TipAsk heldTip: TipAsk {
            target: holder
        }
    }

    TestCase {
        name: "Census"
        when: windowShown

        function read(list) {
            window.contentItem = list
            census.seen = ({})
            census.walk()
            return Object.keys(census.met).sort()
        }

        function cleanup() {
            files.model = 0
            files.visible = true
            graph.model = 0
            chosen.model = 0
            files.forceLayout()
            graph.forceLayout()
            root.wipPooled = 0
            graphState.readings.done = true
            graphState.loading = false
            graphState.failed = false
            graphState.stale = false
            page.pageCarriedSettled = true
            census.waiting = false
            census.armed = false
            census.harnessBranch = null
            window.curPage = null
        }

        // A right-click on another worktree's row stands the pane on it before its file list has come: settled there,
        // the census walks a list with no rows, and the run after it walks one with them.
        function test_a_page_standing_on_another_worktree_owes_its_file_list() {
            verify(Automation.PageSettled.settled(page))
            page.pageCarriedSettled = false
            compare(Automation.PageSettled.missing(page), ["carried"])
            page.pageCarriedSettled = true
            verify(Automation.PageSettled.settled(page))
        }

        function test_files_arrive_before_or_after_the_pane_is_hidden_data() {
            return [{tag: "files-first", filesFirst: true}, {tag: "pane-first", filesFirst: false}]
        }

        function test_files_arrive_before_or_after_the_pane_is_hidden(data) {
            // A hidden DetailsPane has never laid out its list, or keeps the rows laid out before it hid.
            const panes = createTemporaryObject(filePane, root,
                                                {width: 200, height: 80, currentIndex: data.filesFirst ? 0 : 1})
            const list = panes.list
            list.model = 1
            if (data.filesFirst) {
                verify(waitForRendering(root))
                verify(read(list).indexOf("FileRowDelegate") >= 0)
                panes.currentIndex = 1
            }
            verify(waitForRendering(root))
            compare(list.itemAtIndex(0) !== null, data.filesFirst)
            compare(read(list).indexOf("FileRowDelegate"), -1)
            panes.currentIndex = 0
            tryVerify(() => list.itemAtIndex(0) !== null)
            tryVerify(() => read(list).indexOf("FileRowDelegate") >= 0)
        }

        function test_a_wip_row_is_pooled_before_or_after_the_picture() {
            graph.model = 30
            graph.forceLayout()
            verify(waitForRendering(graph))
            verify(read(graph).indexOf("WipTallyRow") >= 0)
            graph.currentIndex = -1
            graph.positionViewAtIndex(20, ListView.Beginning)
            graph.forceLayout()
            verify(waitForRendering(graph))
            verify(root.wipPooled > 0, "the WIP delegate really entered Qt's reuse pool")
            compare(graph.itemAtIndex(0), null)
            compare(read(graph).indexOf("WipTallyRow"), -1)
            graph.positionViewAtIndex(0, ListView.Beginning)
            graph.forceLayout()
            verify(waitForRendering(graph))
            verify(read(graph).indexOf("WipTallyRow") >= 0)
        }

        function test_a_shown_row_with_an_action_index_is_still_counted() {
            chosen.model = [{oid: "a", sha8: "a", subject: "Chosen", body: "", author: "A", authorTime: 0,
                             avatar: 0, avatarUrl: "", mates: []}]
            chosen.forceLayout()
            verify(waitForRendering(chosen))
            compare(chosen.itemAtIndex(0).index, -1)
            verify(read(chosen).indexOf("ChosenCommitRow") >= 0)
        }

        function test_the_union_keeps_a_component_shown_at_the_first_edge() {
            files.model = 1
            files.forceLayout()
            verify(waitForRendering(root))
            read(files)
            files.visible = false
            census.atShot = census.met
            census.walk()
            verify(census.met.FileRowDelegate !== true)
            verify(census.seen.FileRowDelegate === true)
            verify(census.lost().indexOf("FileRowDelegate") >= 0)
        }

        function test_a_part_an_item_holds_by_name_is_counted() {
            compare(read(holder), ["TipAsk"])
        }

        function test_a_part_an_item_of_the_harness_holds_by_name_is_not_counted() {
            census.harnessBranch = root
            compare(read(holder), [])
        }

        function test_a_closed_submenu_reached_first_as_owned_then_as_offered_is_counted() {
            census.visited = ({})
            census.met = ({})
            census.seen = ({})
            census.visit(submenu, true, false)
            verify(census.met.AppMenu !== true)
            census.visit(submenu, true, true)
            verify(census.met.AppMenu === true)
        }

        function test_a_completed_graph_with_matching_own_wip_still_owes_the_carried_read_and_replacement() {
            verify(Automation.PageSettled.settled(page))
            graphState.readings.done = false
            compare(Automation.PageSettled.missing(page), ["readings"])
            graphState.readings.done = true
            verify(Automation.PageSettled.settled(page))
        }

        function test_a_new_stream_owes_its_finish_even_when_the_previous_pass_finished() {
            verify(Automation.PageSettled.settled(page))
            // Started applies the new generation before the queued Finished reaches the UI.
            graphState.loading = true
            compare(Automation.PageSettled.missing(page), ["graph"])
            graphState.loading = false
            verify(Automation.PageSettled.settled(page))
        }

        function test_failed_or_stale_graphs_do_not_owe_a_new_stream_data() {
            return [{tag: "failed", failed: true, stale: false}, {tag: "stale", failed: false, stale: true}]
        }

        function test_failed_or_stale_graphs_do_not_owe_a_new_stream(data) {
            graphState.loading = true
            graphState.readings.done = false
            graphState.failed = data.failed
            graphState.stale = data.stale
            verify(Automation.PageSettled.settled(page))
        }

        function test_an_unchanged_read_ending_without_a_property_edge_still_arms_the_walk() {
            graphState.readings.done = false
            window.curPage = page
            census.waiting = true
            compare(census.settled, false)
            verify(!census.armed)
            const asked = window.updateRequests
            // A field of the JS object emits no property change: the core query has the same absence of an edge.
            graphState.readings.done = true
            compare(census.settled, false)
            tryCompare(census, "armed", true)
            compare(window.updateRequests, asked + 1)
        }
    }
}
