import QtQuick
import QtTest
import platitude.ui

// The toolbar's fetch and push rings (`BandFetchButton` / `BandPushButton`) turn on the tab's word that a fetch or a
// push is still on its way to the screen (`RepoTab.fetching` / `pushing`) and on nothing else of the tab's: git's
// answer, where `busyCount` goes back to 0, leaves them turning. When the tab lets that word go is its own
// (`repo_tab::tests::drain_boundaries`). Driven by their own inputs, the page's shape stood in for.
Item {
    id: root
    width: 400
    height: 200

    readonly property QtObject tab: QtObject {
        id: tab
        property int busyCount: 0
        property bool fetching: false
        property bool pushing: false
        property int fetchFailures: 0
        property bool autoFetchSuspended: false
        // No remote, so the fetch button is off: its tip reads the application's settings, which no test has.
        property int remoteCount: 0
    }
    readonly property QtObject page: QtObject {
        id: page
        property int tab_id: 1
        property var pageTab: tab
        property QtObject pageWorktree: QtObject {
            property string branch: "main"
        }
        property string pushState: "ready"
        property string pushTargetLabel: "origin/main"
        property int pushAhead: 1
        property int pushBehind: 0
        property bool pushFailed: false
        property string pushFailReason: ""
        property bool planShown: false
        property bool canPush: true
        property bool canForcePush: true
    }

    BandFetchButton {
        id: fetch
        curPage: page
    }
    BandPushButton {
        id: push
        y: 80
        curPage: page
    }

    TestCase {
        name: "BandRings"
        when: windowShown

        function init() {
            tab.busyCount = 0
            tab.fetching = false
            tab.pushing = false
        }

        function test_the_fetch_ring_turns_until_what_the_fetch_left_is_on_screen() {
            verify(!fetch.ringing)
            tab.busyCount = 1
            verify(!fetch.ringing, "a command running that is no fetch")
            tab.fetching = true
            verify(fetch.ringing, "a fetch out, whoever asked for it")
            tab.busyCount = 0
            verify(fetch.ringing, "git answered, and what the fetch left is still being read")
            verify(!push.ringing, "the fetch's ring alone")
            tab.fetching = false
            verify(!fetch.ringing, "the screen has it")
        }

        function test_the_push_ring_turns_until_what_the_push_left_is_on_screen() {
            verify(!push.ringing)
            tab.busyCount = 1
            verify(!push.ringing, "a command running that is no push")
            tab.pushing = true
            verify(push.ringing)
            tab.busyCount = 0
            verify(push.ringing, "git answered, and what the push left is still being read")
            verify(!fetch.ringing, "the push's ring alone")
            tab.pushing = false
            verify(!push.ringing, "the screen has it")
        }

        /// A press met by a turning ring goes nowhere (`ActionButton.live`), the answer in or not.
        function test_a_turning_ring_takes_no_press() {
            tab.pushing = true
            verify(!push.live)
            tab.pushing = false
            verify(push.live)
        }
    }
}
