import QtQuick
import QtQuick.Layouts
import platitude.ui

// The corner's line (`GitVersionCorner`): the build's name, then the git it drives after `/git `. Two fields side by
// side, each cut at its end (`LineText`), so each value is taken whole and a press anywhere on the line starts a sweep
// (規約 §右のペインの字は掴める); the second holds `/git ` with the version. A value cut is read whole in the hover's
// tip (`wholeTip`): the corner opens no card. Short of room the line gives way in this order (user decision — ask
// before changing):
//   1. the word `git` — a version that names git itself keeps it (`2.39.5 (Apple Git-154)`): it is the version's own;
//   2. the build's commit, from its end down to its first `cutDigits`;
//   3. the git's version, from its end down to its major, never cut just past a separator.
// Each gives only once the one before it is down to its least. Past the last the line does not fit (`fits`), and the
// corner stands down: only a build from a long-named tree (an app's launch worktree) comes to that at the pane's floor,
// and a release names no tree (user decision: accepted). Every cut is sized to the head it keeps and the mark
// (`cutWidth`), so no ground is left between the mark and what follows it.
RowLayout {
    id: buildLine

    /// The build's name whole: `PGG-v0.1.0`, `v0.1.0-b-c7b549aa`, `v0.1.0-?` (`GitVersionCorner.buildName`).
    required property string name
    /// Its commit, the tail of `name` the line may cut; empty where it has none to cut (a release, a commit the build
    /// could not read).
    required property string hash
    /// The git's version as git says it: `2.51.0`, `2.55.0.windows.3`, `2.39.5 (Apple Git-154)`.
    required property string version
    /// The width the line may take.
    required property real room
    property color color: Theme.textMuted
    property real pixelSize: Theme.fontSm

    /// The commit's digits a cut keeps, as a parent's hash at its least (`HashPlate.cutDigits`).
    readonly property int cutDigits: 3
    /// Where the commit starts in `name`; -1 for none to cut.
    readonly property int hashAt: buildLine.hash.length > buildLine.cutDigits && buildLine.name.endsWith(buildLine.hash)
                                  ? buildLine.name.length - buildLine.hash.length : -1
    /// What separates a version's parts; a cut of the version never keeps one last.
    readonly property string separators: ". (-"
    /// The version's major: what stands before its first `.`.
    readonly property int majorLength: {
        const dot = buildLine.version.indexOf(".")
        return dot > 0 ? dot : buildLine.version.length
    }

    readonly property real nameWhole: nameField.implicitWidth
    readonly property real nameLeast: buildLine.hashAt < 0 ? buildLine.nameWhole
                                                           : buildLine.cutWidth(nameField, buildLine.hashAt
                                                                                           + buildLine.cutDigits)
    /// The second field whole, with the word and without it, and at its least: `/` and the major, and the mark. Off
    /// rulers, so they do not move with what the field shows.
    readonly property real gitWordedWhole: Math.ceil(gitWordedRuler.implicitWidth)
    readonly property real gitWhole: Math.ceil(gitRuler.implicitWidth)
    readonly property real gitLeast: buildLine.majorLength >= buildLine.version.length ? buildLine.gitWhole
                                     : Math.ceil(gitLeastRuler.implicitWidth) + gitField.markWidth
    /// The narrowest the line comes: no word, the commit and the version at their least.
    readonly property real leastWidth: buildLine.nameLeast + buildLine.gitLeast
    readonly property bool fits: buildLine.room >= buildLine.leastWidth
    /// Step 1: the word `git` stands.
    readonly property bool worded: buildLine.room >= buildLine.nameWhole + buildLine.gitWordedWhole
    /// Step 2: the name's width — whole, or cut to as many of the commit's digits as leave the version whole.
    readonly property real nameWidth: {
        if (buildLine.worded || buildLine.room >= buildLine.nameWhole + buildLine.gitWhole || buildLine.hashAt < 0)
            return buildLine.nameWhole
        for (let digits = buildLine.hash.length - 1; digits > buildLine.cutDigits; digits--) {
            const width = buildLine.cutWidth(nameField, buildLine.hashAt + digits)
            if (width + buildLine.gitWhole <= buildLine.room)
                return width
        }
        return buildLine.nameLeast
    }
    /// Step 3: the second field's width — whole, or its version cut to as much as the room leaves, its major at least.
    readonly property real gitWidth: {
        if (buildLine.worded)
            return buildLine.gitWordedWhole
        const left = buildLine.room - buildLine.nameWidth
        if (left >= buildLine.gitWhole)
            return buildLine.gitWhole
        // The field's characters before the version: `/`.
        const before = gitField.text.length - buildLine.version.length
        for (let length = buildLine.version.length - 1; length > buildLine.majorLength; length--) {
            // Never just past a separator: a `.` before the mark reads as one more dot of it (`2.55.0.…`).
            if (buildLine.separators.indexOf(buildLine.version.charAt(length - 1)) >= 0)
                continue
            const width = buildLine.cutWidth(gitField, before + length)
            if (width <= left)
                return width
        }
        return buildLine.gitLeast
    }
    /// What has given way, for runs and tests: `none`, `word`, `hash` or `version` — the last step that gave.
    readonly property string gives: buildLine.worded ? "none"
        : buildLine.gitWidth < buildLine.gitWhole ? "version"
        : buildLine.nameWidth < buildLine.nameWhole ? "hash" : "word"
    /// A value is cut on screen (`LineText.clipped`).
    readonly property bool cut: nameField.clipped || gitField.clipped
    /// The whole line on the hover while a value is cut (規約 §右のペインの字は掴める「切れた名前はカードで読む」).
    readonly property TipAsk wholeTip: TipAsk {
        target: buildLine
        asked: lineHover.hovered && buildLine.cut
        words: buildLine.name + gitWordedRuler.text
    }

    /// A field cut back to its first `chars` characters: their edge as laid out (`LineText.edgeAt`, which a binding
    /// through this follows), rounded up as a layout hands it, and the mark.
    function cutWidth(field, chars) {
        return Math.ceil(field.edgeAt(chars)) + field.markWidth
    }

    // One line: the two fields meet, `/git ` the second's own head.
    spacing: 0

    LineText {
        id: nameField
        text: buildLine.name
        color: buildLine.color
        pixelSize: buildLine.pixelSize
        Layout.preferredWidth: buildLine.nameWidth
    }
    LineText {
        id: gitField
        text: buildLine.worded ? gitWordedRuler.text : gitRuler.text
        color: buildLine.color
        pixelSize: buildLine.pixelSize
        Layout.preferredWidth: buildLine.gitWidth
    }
    HoverHandler {
        id: lineHover
    }

    // The second field's two ways of reading, and its least, measured whole in the fields' face.
    PlainTextLabel {
        id: gitWordedRuler
        visible: false
        text: qsTr("/git %1").arg(buildLine.version)
        font.family: Theme.uiFamily
        font.pixelSize: buildLine.pixelSize
    }
    PlainTextLabel {
        id: gitRuler
        visible: false
        text: qsTr("/%1").arg(buildLine.version)
        font.family: Theme.uiFamily
        font.pixelSize: buildLine.pixelSize
    }
    PlainTextLabel {
        id: gitLeastRuler
        visible: false
        text: qsTr("/%1").arg(buildLine.version.substring(0, buildLine.majorLength))
        font.family: Theme.uiFamily
        font.pixelSize: buildLine.pixelSize
    }
}
