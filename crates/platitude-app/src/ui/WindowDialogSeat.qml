pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls.Fusion
import QtQuick.Dialogs
import platitude
import platitude.ui

// The window's two biggest dialogs — the settings screen and the clone box — with the model and the folder chooser
// the clone owns. Together in one seat because every way into either goes through the doors below, and the doors are
// what lets neither screen exist until it is first opened: the bare window used to pay ~8MB holding the two ready
// (152.7–152.9MB working set against 143.8–145.0 with them built on demand, 2026-08-30 実測 — the settings panes had
// been growing under that eagerness for a day, 実測の刻みは rules-refs/app-ui.md のダイアログの席の行).
//
// The harness is the one caller that gets them up front: its verbs read a dialog's properties before opening it
// (`WindowDialogActs` / `WindowSettingsActs`), and a null there is a dead run, not a refusal.
//
// An `Item` that fills the window, and two Loaders that fill the seat: the dialogs are popups whose sizing reads
// their parent (`AppDialog.fills`), and a loader left unsized hands them a 0x0 to fill (実測 — the screen opened as
// an empty strip).
Item {
    id: seat

    /// The page in front, for where the clone box opens and what the settings screen shows (`RepoPage`).
    required property var curPage
    required property var tabsModel

    /// The three the window's harness reaches for by name (`WindowAutoActDriver`) — the dialogs are null until
    /// their loaders build them.
    readonly property alias cloneModel: cloneModel
    readonly property var cloneDialog: cloneLoader.item
    readonly property var settingsDialog: settingsLoader.item
    /// Whether the settings screen is up — the identity gate holds itself back behind it (`IdentityGate`).
    readonly property bool settingsOpened: settingsLoader.item !== null && settingsLoader.item.opened

    /// The clone box, opened beside the repository that is already open — a second working copy usually goes where
    /// the first one is, which is the reading `openRepositoryPicker` starts from as well. With no tab open there is
    /// nothing to be beside, and the platform dialog's own folder is what stands in.
    function startClone() {
        const near = seat.curPage !== null ? seat.curPage.pageTab.pickerFolderUrl : ""
        cloneLoader.active = true
        cloneLoader.item.start(near !== "" ? near : cloneFolderDialog.currentFolder.toString())
    }
    /// Every door into the settings comes through here, which is what lets the screen not exist until the first one.
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

    // Fetching a repository that has no tab yet, and the folder chooser that belongs to it. A second FolderDialog
    // rather than the picker's: the two ask different questions ("which repository" / "which folder to put one in"),
    // and sharing one would carry each answer into the other's next opening. The model stays built — it is a handful
    // of fields, and the answer to a clone must have somewhere to land even if the box has gone.
    CloneModel {
        id: cloneModel
        // The clone landed, so the box that asked for it goes and the strip does what it always does with a folder
        // that opens. The two objects are joined here rather than to each other: fetching a repository happens before
        // there is a tab, and opening one is the strip's ordinary job (`models/clone.rs`).
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
        active: AppBackend.autoAct !== "" || AppBackend.autoIdentity !== ""
        sourceComponent: CloneDialog {
            cloning: cloneModel.cloning
            onSubmitted: (url, parentUrl, name) => cloneModel.cloneRepository(url, parentUrl, name)
            // Every road out of the box comes through here, Escape included: a clone still on the network is stopped
            // by the same press that takes away the only place its answer could have landed.
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
        // Only the open box can have asked for a folder, so the item is there to be answered.
        onAccepted: if (cloneLoader.item) cloneLoader.item.setFolder(selectedFolder.toString())
    }

    Loader {
        id: settingsLoader
        anchors.fill: parent
        active: AppBackend.autoAct !== "" || AppBackend.autoIdentity !== ""
        sourceComponent: SettingsDialog {
            curPage: seat.curPage
            tabsModel: seat.tabsModel
        }
    }
}
