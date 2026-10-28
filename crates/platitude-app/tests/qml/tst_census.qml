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
    /// The writes the fake tab saw settle, and the fake log's receipts (`PageSettled.recorded`).
    property var settledIds: []
    property var receipts: []

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
    }

    readonly property QtObject page: QtObject {
        id: page
        property var pageTab: ({state: "open", settledWrites: () => root.settledIds})
        property var pageCommands: ({recorded: id => root.receipts.indexOf(id) >= 0})
        property var pageWorktree: ({loaded: true, wipRowStands: false})
        property bool pageRefsLoaded: true
        property bool pageDetailsSettled: true
        property bool pageCarriedSettled: true
        property bool pageLanding: false
        property var pageGraphPane: ({placing: false})
        property var pageGraph: graphState
        property bool diffShown: false
        property bool diffDone: true
        property var pageDiff: ({diffSettled: () => page.diffDone})
    }
    /// A page that is an item with a list on it (a `RepoPage` is one): the rows term walks the page for its lists.
    Item {
        id: listedPage
        y: 200
        width: 200
        height: 40
        property var pageTab: ({state: "open", settledWrites: () => root.settledIds})
        property var pageCommands: ({recorded: id => root.receipts.indexOf(id) >= 0})
        property var pageWorktree: ({loaded: true, wipRowStands: false})
        property bool pageRefsLoaded: true
        property bool pageDetailsSettled: true
        property bool pageCarriedSettled: true
        property bool pageLanding: false
        property var pageGraphPane: ({placing: false})
        property var pageGraph: graphState
        ListModel {
            id: listedRows
        }
        AppListView {
            id: listed
            anchors.fill: parent
            cacheBuffer: 0
            model: listedRows
            delegate: Item {
                width: listed.width
                height: 20
            }
        }
    }
    /// A seat deciding what the hand gets (`HoverSeat`), as the product's rows and tips hold one.
    readonly property HoverSeat deciding: HoverSeat {
        id: decidingSeat
        restMs: 20
        graceMs: 20
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
            page.diffShown = false
            page.diffDone = true
            listedRows.clear()
            listed.visible = true
            root.settledIds = []
            root.receipts = []
            decidingSeat.takeDown("cleanup", false)
            census.atCompletion = null
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

        // The completion's walk is held as it was: a later walk moves `met`, never `atCompletion`, so the picture's
        // walk is held against what the verb completed on (`reportAt`'s `lost=`).
        function test_the_completions_walk_is_held_while_later_walks_move() {
            files.model = 1
            files.forceLayout()
            verify(waitForRendering(root))
            window.contentItem = files
            census.markCompletion()
            verify(census.atCompletion.FileRowDelegate === true)
            files.visible = false
            census.walk()
            verify(census.met.FileRowDelegate !== true)
            verify(census.atCompletion.FileRowDelegate === true)
        }

        // A delegate the view keeps beyond its viewport (the cache buffer) is in no picture and is not counted.
        function test_a_delegate_beyond_the_viewport_is_not_counted() {
            files.cacheBuffer = 400
            files.model = 30
            files.forceLayout()
            verify(waitForRendering(root))
            tryVerify(() => files.itemAtIndex(5) !== null)
            verify(census.inList(files.itemAtIndex(0)))
            verify(!census.inList(files.itemAtIndex(5)))
            files.cacheBuffer = 0
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

        // A read ending without a property edge (a field of the JS object, as the core's query has none) is read
        // fresh by the function the completion's hold samples (`CompletionRoute.reading`), not by a binding.
        function test_the_pages_missing_reads_are_read_fresh_each_time() {
            graphState.readings.done = false
            window.curPage = page
            compare(census.pageMissing(), ["readings"])
            graphState.readings.done = true
            compare(census.pageMissing(), [])
        }

        // A diff shown is a read of the page's while the pane is still reading it.
        function test_a_diff_still_reading_is_owed() {
            page.diffShown = true
            page.diffDone = false
            compare(Automation.PageSettled.missing(page), ["diff"])
            page.diffDone = true
            verify(Automation.PageSettled.settled(page))
        }

        // A seat resting before it opens something under the hand, or beating before it takes it down, is a decision
        // on its way (`hover:<key>`, the key spelled safe for the line).
        function test_a_hover_decision_on_its_way_is_owed() {
            verify(Automation.PageSettled.settled(page))
            decidingSeat.point({ "key": "row:1" })
            compare(Automation.PageSettled.missing(page), ["hover:row_1"])
            tryCompare(decidingSeat, "state", "shown")
            verify(Automation.PageSettled.settled(page))
            decidingSeat.unpoint("row:1")
            compare(Automation.PageSettled.missing(page), ["hover:row_1"])
            tryCompare(decidingSeat, "state", "idle")
            verify(Automation.PageSettled.settled(page))
        }

        // The rows of a list the page shows come a polish after the model lands: owed until the view has built them,
        // found by walking the page; a list not shown owes nothing.
        function test_a_lists_rows_are_owed_until_the_view_has_built_them() {
            verify(Automation.PageSettled.settled(listedPage))
            listedRows.append({})
            compare(Automation.PageSettled.missing(listedPage), ["rows:AppListView"])
            verify(waitForRendering(listed))
            verify(Automation.PageSettled.settled(listedPage))
            listed.visible = false
            listedRows.append({})
            verify(Automation.PageSettled.settled(listedPage))
        }

        // A write's answer reaches the tab and its commands the log on two feeds drained in either order: the page owes
        // the records from the tab seeing the write settle until the log has that write's own receipt — another
        // write's receipt answers nothing for it — and a receipt in first owes nothing.
        function test_a_settled_writes_records_are_owed_until_the_log_has_its_receipt_data() {
            return [{ tag: "answer first", logFirst: false }, { tag: "log first", logFirst: true }]
        }
        function test_a_settled_writes_records_are_owed_until_the_log_has_its_receipt(data) {
            verify(Automation.PageSettled.settled(page))
            if (data.logFirst) {
                root.receipts = [7]
                verify(Automation.PageSettled.settled(page))
                root.settledIds = [7]
                verify(Automation.PageSettled.settled(page))
                return
            }
            root.settledIds = [7]
            compare(Automation.PageSettled.missing(page), ["records"])
            root.receipts = [8]
            compare(Automation.PageSettled.missing(page), ["records"])
            root.receipts = [8, 7]
            verify(Automation.PageSettled.settled(page))
        }

        // A window with no page yet is settled for an act that reads nothing of a page, and not for one that does.
        function test_a_window_is_settled_with_no_page_or_with_its_page_settled() {
            verify(Automation.PageSettled.windowSettled(window))
            verify(!Automation.PageSettled.settledUnder(window))
            window.curPage = page
            verify(Automation.PageSettled.windowSettled(window))
            verify(Automation.PageSettled.settledUnder(window))
            graphState.loading = true
            verify(!Automation.PageSettled.windowSettled(window))
            verify(!Automation.PageSettled.settledUnder(window))
        }
    }
}
