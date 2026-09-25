pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Window
import platitude
import platitude.ui

// The two grabbable stand-ins a headless shot is taken from: `grabToImage` cannot be pointed at what they mirror.
// Built only while a shot directory is set; refreshed at shot time by `AutoShotDriver`.
// `Item`, not `QtObject`: rules-refs/structure.md「切り出した非表示のホストは `Item` にする」.
Item {
    id: mirrors

    required property Window window
    /// What a scene picture is of: the content, or the gate standing in front of it while git has not answered.
    required property Item mainUi
    required property Item gate

    /// `AutoShotDriver` schedules their updates and waits on the textures.
    readonly property alias overlayMirror: overlayMirror
    readonly property alias sceneMirror: sceneMirror

    // Popups render in the window overlay, whose C++-created items `grabToImage` refuses ("no QML engine"); this
    // QML-declared mirror of it is grabbable.
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
    // Content and overlay laid over each other: where a popup stands against what it opened off. Two named sources,
    // because this mirror is inside the content. Refreshed only when the overlay holds something (else it is app.png).
    Loader {
        id: sceneMirror
        active: Harness.shotDir !== ""
        anchors.fill: parent
        z: -10001
        sourceComponent: Item {
            id: sceneShot
            /// Textures still owed. The refresh is asked once app.png is saved, so this mirrors the scene that picture
            /// came from (`AutoShotDriver`).
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
