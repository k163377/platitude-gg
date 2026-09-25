pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Dialogs
import platitude
import platitude.ui

// The settings screen and the clone box (with the clone's model and folder chooser), each built on its first opening
// through the doors below (rules-refs/app-ui.md「窓が黙って実体化するダイアログは Loader の後ろに置く」).
//
// The Loaders fill the seat: the dialogs size off their parent (`AppDialog.fills`), and an unsized loader opens them
// as an empty strip.
Item {
    id: seat

    /// The page in front, for where the clone box opens and what the settings screen shows (`RepoPage`).
    required property var curPage
    required property var tabsModel

    /// Both screens built up front, for a caller that reads their properties before opening them (the harness).
    /// False in every window a person opens.
    property bool keepBuilt: false

    /// The dialogs are null until built — on first opening, or up front with [`keepBuilt`].
    readonly property alias cloneModel: cloneModel
    readonly property var cloneDialog: cloneLoader.item
    readonly property var settingsDialog: settingsLoader.item
    /// Whether the settings screen is up — the identity gate holds itself back behind it (`IdentityGate`).
    readonly property bool settingsOpened: settingsLoader.item !== null && settingsLoader.item.opened

    /// Opens the clone box at the open repository's folder (as `openRepositoryPicker` does), else the folder
    /// dialog's own.
    function startClone() {
        const near = seat.curPage !== null ? seat.curPage.pageTab.pickerFolderUrl : ""
        cloneLoader.active = true
        cloneLoader.item.start(near !== "" ? near : cloneFolderDialog.currentFolder.toString())
    }
    function openSettingsAt(category) {
        settingsLoader.active = true
        settingsLoader.item.openAt(category)
    }
    /// The same door, told whom it was opened on before it opens (デザイン規約 §アバターを与える).
    function openAvatarSettings(name, email) {
        settingsLoader.active = true
        settingsLoader.item.prefillName = name
        settingsLoader.item.prefillEmail = email
        settingsLoader.item.openAt("app")
    }

    // A FolderDialog of its own: shared with the repository picker, each answer would carry into the other's next
    // opening. The model stays built: a clone's answer must land even after the box has gone.
    CloneModel {
        id: cloneModel
        // Joined here: the clone happens before there is a tab, and opening one is the strip's job
        // (`models/clone.rs`).
        onCloneDone: path => {
            if (cloneLoader.item)
                cloneLoader.item.landed()
            seat.tabsModel.openRepositoryPath(path)
        }
        onCloneFailed: message => {
            if (cloneLoader.item)
                cloneLoader.item.said(message)
        }
    }
    Loader {
        id: cloneLoader
        anchors.fill: parent
        active: seat.keepBuilt
        sourceComponent: CloneDialog {
            cloning: cloneModel.cloning
            onSubmitted: (url, parentUrl, name) => cloneModel.cloneRepository(url, parentUrl, name)
            // Every way out of the box, Escape included, stops a clone still on the network.
            onCancelled: cloneModel.cancelClone()
            onChooseFolder: near => {
                if (near !== "")
                    cloneFolderDialog.currentFolder = near
                cloneFolderDialog.open()
            }
        }
    }
    FolderDialog {
        id: cloneFolderDialog
        title: qsTr("Where to put it")
        onAccepted: if (cloneLoader.item) cloneLoader.item.setFolder(selectedFolder.toString())
    }

    Loader {
        id: settingsLoader
        anchors.fill: parent
        active: seat.keepBuilt
        sourceComponent: SettingsDialog {
            curPage: seat.curPage
            tabsModel: seat.tabsModel
        }
    }
}
