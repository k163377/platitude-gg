import QtQuick
import QtTest
import platitude.ui

// A call put off a turn on a page taken down before that turn, as a tab's page is (`RepoPageStack`'s `Loader`):
// `TurnLater`'s answer goes with the page, where a QML function handed to `Qt.callLater` still runs and fails on it.
// Each case is asked of the page itself and of a part inside it with a context of its own, as the product's parts
// have (`RowKeep` in `WipPane`): the clear reaches that one through the page's.
Item {
    id: root
    width: 100
    height: 100

    /// What a page's work reached while the page stood, counted here.
    property int worked: 0

    /// What a page and its parts put off: `bare` hands `Qt.callLater` the QML function, `turned` asks `TurnLater`.
    component Keeper: QtObject {
        id: keeper
        property Item tally: null
        function work() {
            keeper.tally.worked++
        }
        function settle() {
            keeper.work()
        }
        function bare() {
            Qt.callLater(keeper.settle)
        }
        function turned() {
            settleLater.restart()
        }
        readonly property TurnLater settleLater: TurnLater {
            id: settleLater
            onTriggered: keeper.settle()
        }
    }

    Component {
        id: pageShape
        Item {
            id: page
            function work() {
                root.worked++
            }
            function settle() {
                page.work()
            }
            function bare() {
                Qt.callLater(page.settle)
            }
            function turned() {
                settleLater.restart()
            }
            readonly property TurnLater settleLater: TurnLater {
                id: settleLater
                onTriggered: page.settle()
            }
            readonly property Keeper keeper: Keeper {
                tally: root
            }
        }
    }
    Loader {
        id: seat
        active: false
        sourceComponent: pageShape
    }

    TestCase {
        name: "TurnLater"
        when: windowShown

        function init() {
            root.worked = 0
            seat.active = true
        }
        function cleanup() {
            seat.active = false
        }
        /// Waits until the calls put off before this one have had their turn: `Qt.callLater` runs its queue in order.
        function turnPassed() {
            let passed = false
            Qt.callLater(() => passed = true)
            tryVerify(() => passed, undefined, "the turn after the calls put off")
        }
        function holder(data) {
            return data.part ? seat.item.keeper : seat.item
        }
        function test_a_bare_call_runs_on_the_taken_down_page_data() {
            return [{ tag: "page", part: false }, { tag: "part", part: true }]
        }
        /// The premise: the `Loader` clears the page's contexts and deletes it only later, and `Qt.callLater` holds a
        /// call back only for an object deleted or flagged for deletion — so the queued function runs, and the first
        /// QML function it calls on the cleared page is "not a function". Passing without these warnings, Qt holds the
        /// call back itself.
        function test_a_bare_call_runs_on_the_taken_down_page(data) {
            ignoreWarning(/attempted to evaluate a function in an invalid context/)
            ignoreWarning(/Property 'work' of object .* is not a function/)
            holder(data).bare()
            seat.active = false
            turnPassed()
        }
        function test_a_turn_later_goes_with_its_page_data() {
            return [{ tag: "page", part: false }, { tag: "part", part: true }]
        }
        function test_a_turn_later_goes_with_its_page(data) {
            failOnWarning(/invalid context/)
            failOnWarning(/is not a function/)
            holder(data).turned()
            seat.active = false
            turnPassed()
        }
        function test_a_turn_later_comes_once_data() {
            return [{ tag: "page", part: false }, { tag: "part", part: true }]
        }
        /// Asked twice in a turn, it comes once, a turn later.
        function test_a_turn_later_comes_once(data) {
            holder(data).turned()
            holder(data).turned()
            compare(root.worked, 0)
            turnPassed()
            compare(root.worked, 1)
        }
    }
}
