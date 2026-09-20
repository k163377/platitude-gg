import QtQuick
import QtTest
import platitude.ui

Item {
    id: root
    width: 1600
    height: 100

    Component {
        id: fixture
        SettingsHeader {
            blockWidth: 2 * Theme.spaceXxl + Theme.settingsRailWidth + 2 * Theme.spaceLg
                        + Theme.borderWidth + Theme.textWidth
            word: "Application"
            categoryIcon: "app-window"
            unsaved: false
            armed: false
            property int exits: 0
            onClosed: exits++
        }
    }

    TestCase {
        name: "SettingsHeaderExit"
        when: windowShown

        function test_edge_target_data() {
            return [{ tag: "narrow", width: 640 }, { tag: "wide", width: 1440 }]
        }

        function test_edge_target(data) {
            const header = createTemporaryObject(fixture, root, { width: data.width })
            verify(header !== null)
            // Actual pointer clicks cover a full window-button-sized rectangle, including the far edge.
            const top = (header.height - Theme.toolbarHeight) / 2
            for (const x of [header.width - Theme.railWidth + 1, header.width - 1]) {
                for (const y of [top + 1, top + Theme.toolbarHeight - 1]) {
                    const before = header.exits
                    mouseClick(header, x, y)
                    compare(header.exits, before + 1)
                }
            }
            // An unsaved/armed exit remains clickable; the dialog owns the decision to leave.
            header.unsaved = true
            header.armed = true
            mouseClick(header, header.width - 1, header.height / 2)
            compare(header.exits, 5)
        }
    }
}
