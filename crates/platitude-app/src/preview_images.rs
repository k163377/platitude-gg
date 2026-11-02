//! The diff pane's pictures held in memory: a blob side's bytes
//! (`platitude_core::preview::PictureBytes`), named
//! `image://pgg-preview/<id>`, which a QML `Image` reads through an image
//! provider with no file between ([`qobject::PreviewImageProvider`]). Qt
//! decodes them as it would a file of the same bytes and name.
//!
//! A pane holds a read's pictures until it shows the next read or closes
//! (`DiffModel::apply_preview`); a tab let go of takes whatever its panes
//! still held ([`release_tab`]). The ids are the tab's, numbered for the
//! whole run, so a re-read is a new URL and an `Image` loads it afresh.

#![expect(
    unsafe_code,
    reason = "Qt takes an image provider only as an object handed over by pointer, \
              and hands the provider the size to answer by pointer"
)]

use std::collections::BTreeMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use cxx_qt::casting::Upcast;
use cxx_qt_lib::{
    QImage, QQmlEngine, QQmlImageProviderBase, QQmlImageProviderBaseImageType, QSize, QString,
};
use platitude_core::preview::{ImageFormat, PictureBytes};
use qtbridge::qtbridge_type_lib::QQmlApplicationEngine;

#[cxx_qt::bridge]
mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;
        include!("cxx-qt-lib/qsize.h");
        type QSize = cxx_qt_lib::QSize;
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qqmlimageproviderbase.h");
        type QQmlImageProviderBase = cxx_qt_lib::QQmlImageProviderBase;
        #[namespace = "rust::cxxqtlib1"]
        type QQmlImageProviderBaseImageType = cxx_qt_lib::QQmlImageProviderBaseImageType;
    }

    extern "C++Qt" {
        include!(<QtQuick/QQuickImageProvider>);
        #[qobject]
        #[base = QQmlImageProviderBase]
        type QQuickImageProvider;
    }

    extern "RustQt" {
        /// The provider the engine is handed ([`super::install`]).
        #[qobject]
        #[base = QQuickImageProvider]
        type PreviewImageProvider = super::PreviewImageProviderRust;

        #[cxx_override]
        #[cxx_name = "requestImage"]
        unsafe fn request_image(
            self: Pin<&mut PreviewImageProvider>,
            id: &QString,
            size: *mut QSize,
            requested_size: &QSize,
        ) -> QImage;
    }

    impl cxx_qt::Constructor<(), BaseArguments = (QQmlImageProviderBaseImageType,)>
        for PreviewImageProvider
    {
    }

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");

        #[rust_name = "new_preview_image_provider"]
        fn make_unique() -> UniquePtr<PreviewImageProvider>;
    }
}

/// The provider holds nothing of its own: what it hands out is [`HELD`].
pub struct PreviewImageProviderRust;

/// Made as Qt Quick's `Image` kind of provider: the kind that answers with
/// a `QImage`, decoded on the thread that asks.
impl cxx_qt::Constructor<()> for qobject::PreviewImageProvider {
    type NewArguments = ();
    type BaseArguments = (QQmlImageProviderBaseImageType,);
    type InitializeArguments = ();

    fn route_arguments(_: ()) -> ((), (QQmlImageProviderBaseImageType,), ()) {
        ((), (QQmlImageProviderBaseImageType::Image,), ())
    }

    fn new(_: ()) -> PreviewImageProviderRust {
        PreviewImageProviderRust
    }
}

impl qobject::PreviewImageProvider {
    /// Qt's question for the picture `id`, decoded whole. Asked from the
    /// thread the `Image` loads on — the GUI thread for a synchronous
    /// `Image`, Qt's pixmap reader for the pane's asynchronous cell — so it
    /// reads [`HELD`] behind its lock and nothing of this object.
    ///
    /// # Safety
    ///
    /// `size` is null or a `QSize` Qt holds for the call.
    unsafe fn request_image(
        self: Pin<&mut Self>,
        id: &QString,
        size: *mut QSize,
        _requested_size: &QSize,
    ) -> QImage {
        let image = picture(&String::from(id))
            .and_then(|picture| decode(&picture))
            .unwrap_or_default();
        // SAFETY: Qt passes `size` null or pointing at a `QSize` of its own
        // for the length of the call.
        if let Some(size) = unsafe { size.as_mut() } {
            *size = image.size();
        }
        image
    }
}

/// The host part of the provider's URLs.
const PROVIDER: &str = "pgg-preview";

/// The pictures held, by id. Asked from whichever thread the `Image` loads
/// on (the cell loads asynchronously), so behind a lock rather than in the
/// GUI thread's objects.
static HELD: Mutex<BTreeMap<String, PictureBytes>> = Mutex::new(BTreeMap::new());

fn held() -> MutexGuard<'static, BTreeMap<String, PictureBytes>> {
    HELD.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The start of every id the tab `tab` holds under.
fn tab_prefix(tab: i32) -> String {
    format!("t{tab}-")
}

/// Holds `picture` for a pane of the tab `tab`, answering the id it is
/// held under.
pub fn hold(tab: i32, picture: &PictureBytes) -> String {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let id = format!(
        "{}{}",
        tab_prefix(tab),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    held().insert(id.clone(), picture.clone());
    id
}

/// The URL an `Image` reads the picture held under `id` at.
pub fn url(id: &str) -> String {
    format!("image://{PROVIDER}/{id}")
}

/// Lets the picture `id` go: an `Image` that asks for it after this gets
/// an error, not a picture.
pub fn release(id: &str) {
    held().remove(id);
}

/// Lets go of every picture the tab `tab` holds — its pages are gone.
pub fn release_tab(tab: i32) {
    let prefix = tab_prefix(tab);
    held().retain(|id, _| !id.starts_with(&prefix));
}

/// Whether a picture is held under `id`, for a pane's tests.
#[cfg(test)]
pub fn is_held(id: &str) -> bool {
    held().contains_key(id)
}

/// The picture held under `id`, its bytes shared: the lock is let go
/// before anything is decoded.
fn picture(id: &str) -> Option<PictureBytes> {
    held().get(id).cloned()
}

/// `picture` decoded, read first as the format its path names. That is a
/// hint, as a file's suffix is: bytes the named format cannot read are
/// read again by their own signature — `QImage::fromData` does not read
/// past a format named outright.
fn decode(picture: &PictureBytes) -> Option<QImage> {
    picture
        .format()
        .and_then(|format| QImage::from_data(picture.bytes(), Some(qt_name(format))))
        .or_else(|| QImage::from_data(picture.bytes(), None))
}

/// The name Qt's image readers know `format` by.
fn qt_name(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Png => "png",
        ImageFormat::Jpeg => "jpeg",
        ImageFormat::Gif => "gif",
        ImageFormat::Bmp => "bmp",
        ImageFormat::Webp => "webp",
        ImageFormat::Svg => "svg",
        ImageFormat::Ico => "ico",
        ImageFormat::Tiff => "tiff",
        ImageFormat::Avif => "avif",
    }
}

/// Hands `engine` the provider, which the engine owns from then on.
pub fn install(engine: Pin<&mut QQmlApplicationEngine>) {
    let engine: Pin<&mut QQmlEngine> = engine.upcast_pin();
    let provider = qobject::new_preview_image_provider().into_raw();
    // SAFETY: `provider` is a live object, just made and owned by nobody;
    // each upcast is to the class it derives from.
    let base: *const QQmlImageProviderBase = unsafe {
        let quick: *const qobject::QQuickImageProvider = Upcast::upcast_ptr(provider);
        Upcast::upcast_ptr(quick)
    };
    // SAFETY: `QQmlEngine::addImageProvider` takes ownership of the
    // provider, which nothing else holds.
    unsafe { engine.add_image_provider(&QString::from(PROVIDER), base.cast_mut()) };
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use platitude_core::preview::{ImageFormat, PictureBytes};

    /// One pixel, to load.
    const PNG: [u8; 70] = [
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0x64,
        0x60, 0xf8, 0x5f, 0x0f, 0x00, 0x02, 0x87, 0x01, 0x80, 0xeb, 0x47, 0xba, 0x92, 0x00, 0x00,
        0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    /// The pixel, as read from `path`.
    fn png(path: &str) -> PictureBytes {
        PictureBytes::new(PNG.to_vec(), path)
    }

    /// The format of what the provider would decode for `id`.
    fn asked(id: &str) -> Option<Option<ImageFormat>> {
        super::picture(id).map(|picture| picture.format())
    }

    /// Every hold is a new id, so a re-read is a new URL; a tab let go of
    /// takes its own pictures and no other tab's — `t1-` is not `t12-`'s
    /// start.
    #[test]
    fn a_picture_goes_by_its_id_or_with_its_tab() {
        let first = super::hold(1, &png("logo.PNG"));
        let again = super::hold(1, &png("logo.png"));
        let other = super::hold(12, &png("photo.jpg"));
        assert_ne!(first, again);
        assert_eq!(super::url(&first), format!("image://pgg-preview/{first}"));
        assert_eq!(
            super::picture(&first).map(|picture| picture.bytes().to_vec()),
            Some(PNG.to_vec())
        );
        assert_eq!(asked(&first), Some(Some(ImageFormat::Png)));

        super::release(&first);
        assert_eq!(asked(&first), None);
        assert!(asked(&again).is_some());

        super::release_tab(1);
        assert_eq!(asked(&again), None);
        assert_eq!(asked(&other), Some(Some(ImageFormat::Jpeg)));
        super::release_tab(12);
    }

    /// The child: a Qt application with the provider installed, holding
    /// one picture and letting another go, writing the QML that reads them
    /// by their ids and saying how each `Image` came out.
    #[test]
    #[ignore = "started by pictures_held_here_load_through_the_provider, offscreen, as a process of its own"]
    fn provider_side() {
        let Some(dir) = std::env::var_os("PREVIEW_IMAGES_DIR") else {
            return;
        };
        let mut app = qtbridge::qtbridge_type_lib::QGuiApplication::new();
        let mut engine = qtbridge::qtbridge_type_lib::QQmlApplicationEngine::new();
        crate::qml_engine::arm(engine.pin_mut());
        super::install(engine.pin_mut());
        let one = super::hold(7, &png("logo.png"));
        let misnamed = super::hold(7, &png("photo.jpg"));
        let unnamed = super::hold(7, &png("data.bin"));
        let gone = super::hold(7, &png("logo.png"));
        super::release(&gone);
        let urls = [
            ("one", super::url(&one)),
            ("misnamed", super::url(&misnamed)),
            ("unnamed", super::url(&unnamed)),
            ("gone", super::url(&gone)),
            ("none", super::url("t7-0")),
        ];
        let model = urls
            .iter()
            .map(|(name, url)| format!("[{name:?}, {url:?}]"))
            .collect::<Vec<_>>()
            .join(", ");
        let qml = Path::new(&dir).join("Images.qml");
        std::fs::write(
            &qml,
            format!(
                "import QtQuick\nItem {{\n    id: root\n    Repeater {{\n        id: images\n        \
                 model: [{model}]\n        delegate: Image {{\n            \
                 required property var modelData\n            source: modelData[1]\n            \
                 asynchronous: true\n            cache: false\n        }}\n    }}\n    \
                 function report() {{\n        for (let i = 0; i < images.count; i++) {{\n            \
                 const image = images.itemAt(i)\n            \
                 if (image.status === Image.Loading) {{\n                \
                 Qt.callLater(root.report)\n                return\n            }}\n        }}\n        \
                 for (let i = 0; i < images.count; i++) {{\n            \
                 const image = images.itemAt(i)\n            \
                 console.log('IMAGE\\t' + image.modelData[0] + '\\t' + image.status + '\\t' \
                 + image.implicitWidth + 'x' + image.implicitHeight)\n        }}\n        \
                 Qt.quit()\n    }}\n    Component.onCompleted: Qt.callLater(root.report)\n}}\n"
            ),
        )
        .expect("the QML");
        crate::qml_engine::load(engine.pin_mut(), &crate::urlpath::file_url(&qml))
            .expect("the images load");
        app.pin_mut().exec();
        drop(engine);
        qtbridge::collect_garbage();
        drop(app);
    }

    /// Bytes held in this process show in an `Image` through
    /// `image://pgg-preview/<id>`, loaded off the GUI thread as the pane's
    /// cell loads them, with no file between — under an extension that
    /// names another format too, as a misnamed file would, and from a path
    /// that names none; an id let go, or never held, is an error, not a
    /// picture.
    #[test]
    fn pictures_held_here_load_through_the_provider() {
        let dir = crate::temp::root().join(format!("pgg-preview-images-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder for the QML");
        let out = std::process::Command::new(std::env::current_exe().expect("this test binary"))
            .args([
                "preview_images::tests::provider_side",
                "--exact",
                "--ignored",
                "--nocapture",
            ])
            .env("PREVIEW_IMAGES_DIR", &dir)
            .env("QT_QPA_PLATFORM", "offscreen")
            .env("QT_FORCE_STDERR_LOGGING", "1")
            .output()
            .expect("the child starts");
        let errors = String::from_utf8_lossy(&out.stderr);
        let said: Vec<&str> = errors
            .lines()
            .filter_map(|line| line.split_once("IMAGE\t").map(|(_, rest)| rest))
            .collect();
        let _ = std::fs::remove_dir_all(&dir);
        // `Image.Ready` is 1, `Image.Error` 3.
        assert_eq!(
            said,
            [
                "one\t1\t1x1",
                "misnamed\t1\t1x1",
                "unnamed\t1\t1x1",
                "gone\t3\t0x0",
                "none\t3\t0x0"
            ],
            "{errors}"
        );
    }
}
