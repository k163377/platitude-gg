import QtQuick
import QtTest

// What Qt reads for each spelling of a load: every load here names one of
// `tests/loads/`, and the reader of what the QML tests load
// (`gate::plan::imports`, `gate::plan::urls`) names the same file for each
// spelling (`each_spelling_of_the_spelling_test_names_the_file_qt_reads`), so
// a change to it owes this run.
Item {
    id: root

    Loader {
        id: joined

        source: "../loads/" + "Plain.qml"
    }

    Loader {
        id: resolved

        source: Qt.resolvedUrl(`../loads/Plain.qml`)
    }

    Loader {
        id: unreserved

        source: "../loads/./%50lain%2Eqml?at=1#top"
    }

    Loader {
        id: encoded

        source: "../loads/Two%20Words.qml"
    }

    Loader {
        id: escaped

        source: "../loads/Two Words.qml"
    }

    Loader {
        id: assigned
    }

    Loader {
        id: keyed
    }

    Component.onCompleted: {
        (assigned.source = "../loads/" + "Plain.qml")
        keyed["source"] = ("../loads/Two Words.qml")
    }

    TestCase {
        name: "LoadSpellings"

        function test_each_spelling_reads_its_file_data() {
            return [
                { tag: "joined", loader: joined, file: "Plain" },
                { tag: "resolved", loader: resolved, file: "Plain" },
                { tag: "unreserved", loader: unreserved, file: "Plain" },
                { tag: "encoded", loader: encoded, file: "Two Words" },
                { tag: "escaped", loader: escaped, file: "Two Words" },
                { tag: "assigned", loader: assigned, file: "Plain" },
                { tag: "keyed", loader: keyed, file: "Two Words" },
            ]
        }

        function test_each_spelling_reads_its_file(data) {
            compare(data.loader.status, Loader.Ready)
            compare(data.loader.item.objectName, data.file)
        }
    }
}
