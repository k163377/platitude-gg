import QtQuick
import QtQuick.Controls.Fusion
import QtTest
import platitude.ui

Item {
    id: root
    width: 240
    height: 80

    /// What the test marks are filled with: not the window's own clear colour, so a pixel of it is a mark drawn.
    readonly property color ink: "red"

    /// Spends more wall clock than one time slice the window gives the builder of its background items (two thirds
    /// of a frame at the longest), from inside a binding: the build of whatever holds that binding stops on it.
    function spend() {
        const until = Date.now() + 60
        while (Date.now() < until) {}
        return 1
    }
    /// A chip as `encode::chips_of` hands one to a row, every field spelled out.
    function onRemote(name) {
        return { "kind": "branch", "name": name, "isHead": false, "hasRemote": true, "hasPr": false, "here": true,
                 "held": false, "locked": false, "remote": "", "key": "branch:" + name }
    }

    Component {
        id: fixture
        Item {
            width: 32
            height: 28
            property alias mark: mark
            InkCanvas {
                id: mark
                anchors.fill: parent
                onPaint: {
                    const ctx = getContext("2d")
                    ctx.fillStyle = root.ink
                    ctx.fillRect(0, 0, width, height)
                }
            }
        }
    }
    /// A row as a list builds one past its edge (`cacheBuffer`): in the background, a time slice at a go. The mark is
    /// sized by a binding and its seat is hidden, as a chip's cloud is on a row that carries no name. `spent` ends the
    /// slice after the row has its window and before its sizes — the last bindings the builder runs — are in.
    Component {
        id: lateRow
        Item {
            id: row
            property bool shown: false
            property real side: 16
            property int spent: root.spend()
            property alias mark: lateMark
            width: 32
            height: 28
            Item {
                visible: row.shown
                InkCanvas {
                    id: lateMark
                    width: row.side
                    height: row.side
                    onPaint: {
                        const ctx = getContext("2d")
                        ctx.fillStyle = root.ink
                        ctx.fillRect(0, 0, width, height)
                    }
                }
            }
        }
    }
    /// The same build around the graph's own chip, on a row with no name.
    Component {
        id: lateChip
        Item {
            id: row
            property var records: []
            property int spent: root.spend()
            property alias chip: chip
            width: 220
            height: 28
            RefChip {
                id: chip
                records: row.records
                maxWidth: 200
            }
        }
    }
    /// The chip built at once, in a row of the same size standing clear of the one above: `grabImage` cuts its picture
    /// out of the window's at the item's own `x` / `y`, so two rows on one spot are one picture.
    Component {
        id: chipAtOnce
        Item {
            id: row
            property var records: []
            property alias chip: chip
            y: 40
            width: 220
            height: 28
            RefChip {
                id: chip
                records: row.records
                maxWidth: 200
            }
        }
    }
    SignalSpy {
        id: paints
        signalName: "painted"
    }
    /// A mark in a card, as a menu row's is. A card that closes takes its items out of the window, and a canvas
    /// leaving its window drops its picture.
    Popup {
        id: card
        x: 120
        y: 44
        width: 20
        height: 20
        padding: 0
        closePolicy: Popup.NoAutoClose
        enter: null
        exit: null
        background: Item {}
        contentItem: Item {
            InkCanvas {
                width: 16
                height: 16
                onPaint: {
                    const ctx = getContext("2d")
                    ctx.fillStyle = root.ink
                    ctx.fillRect(0, 0, width, height)
                }
            }
        }
    }

    TestCase {
        name: "VisibleInk"
        when: windowShown

        /// What the background builds left standing, taken down after each test: a row left over one that failed
        /// would stand in the next one's picture.
        property var built: []
        function cleanup() {
            for (const item of built)
                item.destroy()
            built = []
            card.close()
        }

        /// Every mark under `item`, drawn or not.
        function marksOf(item, found) {
            if (item.inked !== undefined && item.kind !== undefined)
                found.push(item)
            for (const child of item.children)
                marksOf(child, found)
            return found
        }
        /// How many pixels of two pictures of one size differ.
        function pixelsApart(drawn, wanted) {
            let differing = 0
            for (let x = 0; x < wanted.width; x++)
                for (let y = 0; y < wanted.height; y++)
                    if (!Qt.colorEqual(drawn.pixel(x, y), wanted.pixel(x, y)))
                        differing++
            return differing
        }
        /// Lets what is queued now, and so everything queued before it, run.
        function letTheQueueRun() {
            let ran = false
            Qt.callLater(() => ran = true)
            tryVerify(() => ran)
        }
        /// `component` built the way a list builds the rows past its edge, and cut where the product's build is cut:
        /// the mark `markOf` picks has its window and its size, Qt's first ask has been and gone, and it has no
        /// picture. Whether the ask lands before the sizes is a race `spent` nearly always loses for the builder; a
        /// build that got its sizes in first has a painted mark, and is thrown away and made again.
        function cutInTheBackground(component, markOf) {
            for (let attempt = 0; attempt < 5; attempt++) {
                const incubator = component.incubateObject(root, {}, Qt.Asynchronous)
                tryVerify(() => incubator.status === Component.Ready)
                const row = incubator.object
                const mark = markOf(row)
                tryCompare(mark, "available", true)
                // The ask is a queued call posted as `available` was set.
                letTheQueueRun()
                if (!mark.inked) {
                    built = built.concat([row])
                    return row
                }
                row.destroy()
            }
            fail("five builds, and none was cut between its window and its sizes")
        }

        function test_hidden_parent_does_not_hold_the_screenshot() {
            const before = Ink.owed
            const item = createTemporaryObject(fixture, root, { visible: false })
            verify(item !== null)
            verify(item.mark.Window.window !== null)
            verify(!item.mark.inked)
            compare(Ink.owed, before)
        }

        function test_hiding_before_paint_releases_and_showing_reacquires() {
            const before = Ink.owed
            const item = createTemporaryObject(fixture, root)
            verify(item !== null)
            compare(Ink.owed, before + 1)
            item.visible = false
            compare(Ink.owed, before)
            item.visible = true
            compare(Ink.owed, before + 1)
            tryCompare(item.mark, "inked", true)
            compare(Ink.owed, before)
            item.visible = false
            item.visible = true
            compare(Ink.owed, before)
        }

        /// Made drawable while it had no size, then sized while hidden, a canvas has had no picture asked of it: Qt's
        /// one first ask finds nothing to paint, and a resize while hidden asks for none. Shown, it draws. The order
        /// by hand; the two tests below reach it the way the product does.
        function test_sized_while_hidden_draws_once_shown() {
            const item = createTemporaryObject(fixture, root, { "width": 0, "height": 0 })
            verify(item !== null)
            tryCompare(item.mark, "available", true)
            letTheQueueRun()
            item.visible = false
            item.width = 32
            item.height = 28
            item.visible = true
            tryVerify(() => Qt.colorEqual(grabImage(item).pixel(16, 14), root.ink), undefined,
                      "the picture is on screen")
            verify(item.mark.inked)
        }

        /// A row built in the background has its window before its bindings have run. When the builder's time slice
        /// ends in between and Qt's first ask runs in the gap, the ask reaches a canvas with no size, and the size
        /// then lands on a hidden seat, which asks for nothing: the mark has never been painted. Shown later — the
        /// row given a commit that carries a name — it draws.
        function test_a_mark_built_in_the_background_draws_once_shown() {
            const row = cutInTheBackground(lateRow, made => made.mark)
            compare(row.mark.width, 16)
            row.shown = true
            tryVerify(() => Qt.colorEqual(grabImage(row).pixel(8, 8), root.ink), undefined,
                      "the picture is on screen")
            verify(row.mark.inked)
        }

        /// The graph's chip through the same build: its row gains a branch that is on a remote, and the cloud is
        /// drawn — read off the pixels against a chip built at once on the same records.
        function test_a_chip_built_in_the_background_draws_the_cloud_its_row_gains() {
            const cloudOf = made => marksOf(made.chip, []).find(mark => mark.kind === "remote")
            const row = cutInTheBackground(lateChip, cloudOf)
            const cloud = cloudOf(row)
            compare(cloud.width, Theme.iconSm)
            row.records = [root.onRemote("sample")]
            const atOnce = createTemporaryObject(chipAtOnce, root, { "records": row.records })
            tryVerify(() => marksOf(atOnce, []).filter(mark => mark.visible).every(mark => mark.inked))
            row.chip.layOutNow()
            atOnce.chip.layOutNow()
            compare(row.chip.width, atOnce.chip.width)

            const wanted = grabImage(atOnce)
            tryVerify(() => pixelsApart(grabImage(row), wanted) === 0, undefined,
                      "the chip reads as one built at once")
            verify(cloud.inked)
        }

        /// Only a mark with no picture is painted for being shown: a pane coming back from behind another shows
        /// every mark in it at once, and painting those again is the whole pane's lanes and marks in one turn.
        function test_a_painted_mark_is_not_painted_again_for_being_shown() {
            const item = createTemporaryObject(fixture, root)
            tryCompare(item.mark, "inked", true)
            paints.target = item.mark
            paints.clear()
            item.visible = false
            item.visible = true
            // A paint asked for by the show would have run in the polish this frame starts with.
            verify(Qt.colorEqual(grabImage(item).pixel(16, 14), root.ink), "the picture it had is still on screen")
            compare(paints.count, 0)
            paints.target = null
        }

        /// A menu row's mark asks for no paint of its own. The first time its card opens, Qt's first ask draws it;
        /// closed, the card leaves the window and the picture goes with it; opened again, the mark counts as painted,
        /// so it is Qt asking a canvas back in a window (`qquickcanvasitem.cpp` `itemChange`) that draws it.
        function test_a_mark_in_a_card_is_drawn_each_time_the_card_opens() {
            for (const time of ["first", "second"]) {
                card.open()
                tryVerify(() => Qt.colorEqual(grabImage(root).pixel(128, 52), root.ink), undefined,
                          "the mark is on screen the " + time + " time")
                card.close()
                tryVerify(() => !Qt.colorEqual(grabImage(root).pixel(128, 52), root.ink), undefined,
                          "and gone with the card")
            }
        }
    }
}
