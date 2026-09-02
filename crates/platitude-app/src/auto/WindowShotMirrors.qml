pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Window
import platitude
import platitude.ui

// The two grabbable stand-ins a headless shot is taken from, and nothing else.
// Both sit behind everything the window draws and are never looked at: what
// they are for is `grabToImage`, which cannot be pointed at what they mirror.
// Built only while a shot directory is set, so an ordinary run carries none of
// it, and refreshed at shot time by `AutoShotDriver`.
//
// An `Item`, not a `QtObject`: these are Loaders, and a `QtObject` has nowhere
// to put a child (rules-refs/structure.md).
Item {
    id: mirrors

    required property Window window
    /// The two things a scene picture can be of: the content, or the screen
    /// that stands in front of it while git has not answered.
    required property Item mainUi
    required property Item gate

    /// The mirrors themselves — `AutoShotDriver` schedules their updates and
    /// waits on the textures.
    readonly property alias overlayMirror: overlayMirror
    readonly property alias sceneMirror: sceneMirror

    // Popups (dialogs, menus) render in the window overlay, whose C++-created items grabToImage refuses ("no QML
    // engine"). This QML-declared mirror of the overlay is grabbable, which is what makes popups photographable on the
    // offscreen platform, where no OS window exists to shoot from outside.
    Loader {
        id: overlayMirror
        active: Harness.shotDir !== ""
        anchors.fill: parent
        z: -10000
        sourceComponent: ShaderEffectSource {
            sourceItem: mirrors.window.Overlay.overlay
            live: false
        }
    }
    // The two laid over each other, for the one question a pair of pictures cannot answer: *where* the thing that
    // opened stands against the thing it opened off. Two sources rather than one of the window's root item: this
    // mirror is inside the content, so a mirror of everything would be a mirror of itself. Refreshed only when the
    // overlay was holding something — with nothing open this picture is app.png again, and a board of doubles is a
    // board nobody reads.
    Loader {
        id: sceneMirror
        active: Harness.shotDir !== ""
        anchors.fill: parent
        z: -10001
        sourceComponent: Item {
            id: sceneShot
            /// How many of the two textures are still owed. The refresh is asked for once the app's own picture is
            /// saved, so what is mirrored here is the scene that picture came out of — ink and all (`AutoShotDriver`).
            property int owed: 0
            signal ready()
            function refresh() {
                sceneShot.owed = 2
                uiSource.scheduleUpdate()
                overlaySource.scheduleUpdate()
            }
            function landed() {
                sceneShot.owed--
                if (sceneShot.owed === 0)
                    sceneShot.ready()
            }
            ShaderEffectSource {
                id: uiSource
                anchors.fill: parent
                live: false
                sourceItem: mirrors.gate.visible ? mirrors.gate : mirrors.mainUi
                onScheduledUpdateCompleted: sceneShot.landed()
            }
            ShaderEffectSource {
                id: overlaySource
                anchors.fill: parent
                live: false
                sourceItem: mirrors.window.Overlay.overlay
                onScheduledUpdateCompleted: sceneShot.landed()
            }
        }
    }
}
