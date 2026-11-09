import QtQuick
import QtTest
import platitude.ui

// What a lines link's row says (`GitHubMenu.sentenceFor`): one line or a run, chosen by the line numbers and spelled
// with them. The headless `diff-github` drags over every line of its diff, so only the run is said there.
Item {
    GitHubMenu {
        id: card
    }

    TestCase {
        name: "GitHubSentence"

        function test_a_line_alone_or_a_run() {
            compare(card.sentenceFor("lines", 7, 7), "Copy link to line 7 at %1 on %2")
            compare(card.sentenceFor("lines", 3, 4), "Copy link to lines 3–4 at %1 on %2")
        }
    }
}
