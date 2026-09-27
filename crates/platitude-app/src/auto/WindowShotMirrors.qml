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
    /// …with the window's edge over it where the window draws one (`Main.edgeDrawn`).
    required property Item windowEdge

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
    // Content, the window's edge and the overlay laid over each other: where a popup stands against what it opened
    // off. Named sources, because this mirror is inside the content. Refreshed only when the overlay holds something
    // (else it is app.png).
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
                // Where each stands in the window: the body stands in from the edge (`Main.edgeInset`), and a
                // source stretched over the whole window would draw it wider than it is.
                sceneShot.seat(uiSource)
                sceneShot.seat(edgeSource)
                sceneShot.owed = edgeSource.visible ? 3 : 2
                uiSource.scheduleUpdate()
                if (edgeSource.visible)
                    edgeSource.scheduleUpdate()
                overlaySource.scheduleUpdate()
            }
            function seat(source) {
                const item = source.sourceItem
                const at = sceneShot.mapFromItem(item, 0, 0)
                source.x = at.x
                source.y = at.y
                source.width = item.width
                source.height = item.height
            }
            function landed() {
                sceneShot.owed--
                if (sceneShot.owed === 0)
                    sceneShot.ready()
            }
            ShaderEffectSource {
                id: uiSource
                live: false
                sourceItem: mirrors.gate.visible ? mirrors.gate : mirrors.mainUi
                onScheduledUpdateCompleted: sceneShot.landed()
            }
            // Over the body and under the overlay, as the window stacks them.
            ShaderEffectSource {
                id: edgeSource
                visible: mirrors.windowEdge.visible
                live: false
                sourceItem: mirrors.windowEdge
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
