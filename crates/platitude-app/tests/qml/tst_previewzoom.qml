import QtQuick
import QtTest
import platitude.ui

// A preview side's drawing is drawn at the zoom it is shown at, learnt from its first answer, while pixels and a
// drawing that needs no zoom are decoded once as they are (`ImagePreviewCell.drawnZoom`). A file's side, read by Qt:
// a blob's goes through the image provider, which this runner does not have.
Item {
    id: root
    width: 900
    height: 700

    Component {
        id: cellMaker
        ImagePreviewCell {
            width: 400
            height: 400
            caption: "After"
            sizeText: "1 KB"
        }
    }

    TestCase {
        name: "PreviewZoom"
        when: windowShown

        // Made with no picture and read on this thread before it is handed one: Qt's pixmap reader thread stands at
        // the lowest priority on Windows, and its answer on a busy machine runs past a QtTest wait (rules-refs
        // `Canvas.loadImage`). Read here, both answers — the first, and the drawing asked for again — land inside the
        // assignment.
        function cell(url, isVector) {
            const made = createTemporaryObject(cellMaker, root, { url: "", isVector: isVector })
            verify(made !== null)
            image(made).asynchronous = false
            made.url = url
            verify(made.settled, "the side answers")
            verify(made.shown, "the side shows its picture")
            return made
        }
        // The side's one `Image`, found by what only an image has.
        function image(item) {
            for (const child of item.children) {
                if (child.sourceSize !== undefined && child.fillMode !== undefined)
                    return child
                const found = image(child)
                if (found !== null)
                    return found
            }
            return null
        }

        function test_a_small_drawing_is_drawn_at_the_zoom_it_is_shown_at() {
            const made = cell(Qt.resolvedUrl("../../assets/platitude-16.svg"), true)
            compare(made.drawnZoom, 8)
            const picture = image(made)
            compare(picture.sourceSize, Qt.size(128, 128), "asked again at 8 times its 16px")
            compare(picture.implicitWidth, 128, "the picture is drawn at that size")
            compare(picture.width, 128, "and shown at it")
        }

        function test_a_drawing_too_big_to_enlarge_is_drawn_at_its_own_size() {
            const made = cell(Qt.resolvedUrl("../../assets/platitude.svg"), true)
            compare(made.drawnZoom, 1)
            compare(image(made).implicitWidth, 256)
        }

        function test_pixels_are_decoded_as_they_are_and_enlarged_whole() {
            const made = cell(Qt.resolvedUrl("../../assets/icon-16.png"), false)
            compare(made.drawnZoom, 1)
            const picture = image(made)
            compare(picture.implicitWidth, 16, "the stored pixels, not rescaled at decode")
            compare(picture.width, 128, "shown 8 times over")
        }
    }
}
