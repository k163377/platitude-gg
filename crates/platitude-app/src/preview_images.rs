//! The diff pane's pictures that have no file: a blob side
//! (`platitude_core::preview::PictureBlob`), named
//! `image://pgg-preview/<id>`, which a QML `Image` reads through an
//! asynchronous image provider ([`qobject::PreviewImageProvider`]). Asked
//! for a picture, the provider answers at once with a response
//! ([`qobject::PreviewImageResponse`]) and has the tab's session read the
//! blob (`platitude_core::preview::PictureReader`) for a thread of its own
//! to decode as Qt would a file of the same bytes and name — streamed into
//! Qt's reader where that reads in order (`preview_decode`), else whole. An
//! opened picture costs its decoded pixels (`platitude_core::preview`), and
//! no Qt thread waits on git.
//!
//! A pane holds a read's pictures — their objects, not their bytes — until
//! it shows the next read or closes (`DiffModel::apply_preview`); a tab let
//! go of takes whatever its panes still held ([`release_tab`]). The ids are
//! the tab's, numbered for the whole run, so a re-read is a new URL and an
//! `Image` loads it afresh.

#![expect(
    unsafe_code,
    reason = "Qt takes an image provider and its responses only as objects handed over by \
              pointer, and the CXX bridge to them is unsafe code of its own"
)]

use std::collections::BTreeMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

use cxx_qt::casting::Upcast;
use cxx_qt::{CxxQtThread, CxxQtType, Threading};
use cxx_qt_lib::{QImage, QQmlEngine, QQmlImageProviderBase, QSize, QString};
use platitude_core::preview::{ImageFormat, PictureBlob, PictureReader};
use qtbridge::qtbridge_type_lib::QQmlApplicationEngine;
use tokio_util::sync::CancellationToken;

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

        include!(<QtQuick/QQuickImageProvider>);
        type QQuickTextureFactory;

        /// A texture factory holding `image`, which Qt owns once it is
        /// handed over.
        #[Self = "QQuickTextureFactory"]
        #[cxx_name = "textureFactoryForImage"]
        fn texture_factory_for_image(image: &QImage) -> *mut QQuickTextureFactory;

        /// The C++ `QImage` behind `cxx_qt_lib::QImage`, for a member that
        /// crate does not bind.
        #[cxx_name = "QImage"]
        type QImageMembers;

        /// Converts the image to `format` — in place where the two are of
        /// one depth, with no second buffer.
        #[cxx_name = "convertTo"]
        fn convert_to(
            self: Pin<&mut QImageMembers>,
            format: QImageFormat,
            flags: ImageConversionFlags,
        );

        /// Qt's asynchronous provider. It has no `Q_OBJECT` of its own, so
        /// it is no `#[qobject]`, whose downcast is a `qobject_cast`.
        type QQuickAsyncImageProvider;
    }

    #[namespace = "rust::cxxqtlib1"]
    extern "C++" {
        include!("cxx-qt-lib/qimage.h");
        type QImageFormat = cxx_qt_lib::QImageFormat;
    }

    #[namespace = "Qt"]
    extern "C++" {
        include!(<QtCore/qnamespace.h>);
        type ImageConversionFlags = super::ImageConversionFlags;
    }

    extern "C++Qt" {
        #[qobject]
        #[base = QQmlImageProviderBase]
        type QQuickImageProvider;

        #[qobject]
        type QQuickImageResponse;
    }

    #[namespace = "rust::cxxqt1"]
    unsafe extern "C++" {
        include!("cxx-qt/casting.h");

        /// The asynchronous provider as the `QQuickImageProvider` it
        /// derives from — the step its missing `#[qobject]` leaves out.
        #[cxx_name = "upcastPtr"]
        #[rust_name = "as_quick_image_provider"]
        unsafe fn upcast_async_provider(
            provider: *const QQuickAsyncImageProvider,
        ) -> *const QQuickImageProvider;
    }

    extern "RustQt" {
        /// The provider the engine is handed ([`super::install`]).
        #[qobject]
        #[base = QQuickAsyncImageProvider]
        type PreviewImageProvider = super::PreviewImageProviderRust;

        #[cxx_override]
        #[cxx_name = "requestImageResponse"]
        fn request_image_response(
            self: Pin<&mut PreviewImageProvider>,
            id: &QString,
            requested_size: &QSize,
        ) -> *mut QQuickImageResponse;

        /// One picture on its way, finished on its own thread by the read
        /// ([`super::Answer`]).
        #[qobject]
        #[base = QQuickImageResponse]
        type PreviewImageResponse = super::PreviewImageResponseRust;

        #[cxx_override]
        #[cxx_name = "textureFactory"]
        fn texture_factory(self: &PreviewImageResponse) -> *mut QQuickTextureFactory;

        #[cxx_override]
        #[cxx_name = "errorString"]
        fn error_string(self: &PreviewImageResponse) -> QString;

        #[cxx_override]
        fn cancel(self: Pin<&mut PreviewImageResponse>);
    }

    unsafe extern "RustQt" {
        /// Qt's own signal, which it waits for from every response.
        #[inherit]
        #[qsignal]
        fn finished(self: Pin<&mut PreviewImageResponse>);
    }

    impl cxx_qt::Constructor<()> for PreviewImageProvider {}
    impl cxx_qt::Constructor<()> for PreviewImageResponse {}
    impl cxx_qt::Threading for PreviewImageResponse {}

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");

        #[rust_name = "new_preview_image_provider"]
        fn make_unique() -> UniquePtr<PreviewImageProvider>;

        #[rust_name = "new_preview_image_response"]
        fn make_unique() -> UniquePtr<PreviewImageResponse>;
    }
}

/// The provider holds nothing of its own: what it hands out is [`HELD`].
pub struct PreviewImageProviderRust;

impl cxx_qt::Constructor<()> for qobject::PreviewImageProvider {
    type NewArguments = ();
    type BaseArguments = ();
    type InitializeArguments = ();

    fn route_arguments(_: ()) -> ((), (), ()) {
        ((), (), ())
    }

    fn new(_: ()) -> PreviewImageProviderRust {
        PreviewImageProviderRust
    }
}

impl qobject::PreviewImageProvider {
    /// Qt's question for the picture `id`, asked on its image-loading
    /// thread: answered at once with a response the read finishes, so that
    /// thread goes on to its next image while git runs.
    fn request_image_response(
        self: Pin<&mut Self>,
        id: &QString,
        _requested_size: &QSize,
    ) -> *mut qobject::QQuickImageResponse {
        let mut response = qobject::new_preview_image_response();
        let answer = Answer {
            thread: Some(response.qt_thread()),
        };
        // Told on the response's own thread, behind this return — after Qt
        // has heard of the response, which has to see `finished`.
        let stop = match picture(&String::from(id)) {
            Some(Held {
                picture,
                reader: Some(reader),
            }) => Some(draw(&reader, picture, answer)),
            _ => None,
        };
        response.pin_mut().rust_mut().stop = stop;
        let response = response.into_raw();
        // SAFETY: `response` is a live object, just made, which Qt owns from
        // this answer on; the upcast is to the class it derives from.
        unsafe { Upcast::<qobject::QQuickImageResponse>::upcast_ptr(response) }.cast_mut()
    }
}

/// What a response answers with: its picture once read and decoded — null
/// until then, and for good where none came — and the token that stops
/// its read.
#[derive(Default)]
pub struct PreviewImageResponseRust {
    image: QImage,
    stop: Option<CancellationToken>,
}

impl cxx_qt::Constructor<()> for qobject::PreviewImageResponse {
    type NewArguments = ();
    type BaseArguments = ();
    type InitializeArguments = ();

    fn route_arguments(_: ()) -> ((), (), ()) {
        ((), (), ())
    }

    fn new(_: ()) -> PreviewImageResponseRust {
        PreviewImageResponseRust::default()
    }
}

impl qobject::PreviewImageResponse {
    /// The picture, for Qt to own; none where it did not come.
    fn texture_factory(&self) -> *mut qobject::QQuickTextureFactory {
        if self.image.is_null() {
            return std::ptr::null_mut();
        }
        qobject::QQuickTextureFactory::texture_factory_for_image(&self.image)
    }

    /// Why no picture came, which is what makes the `Image` an error.
    fn error_string(&self) -> QString {
        if self.image.is_null() {
            QString::from("the picture could not be read")
        } else {
            QString::default()
        }
    }

    /// The `Image` no longer wants the picture: its read stops, and the
    /// response finishes with none, which Qt still waits for.
    fn cancel(self: Pin<&mut Self>) {
        if let Some(stop) = &self.stop {
            stop.cancel();
        }
    }
}

/// How many of git's chunks stand between the read and the decoder: what
/// is held of a picture's compressed bytes while it is drawn.
const CHUNKS_IN_FLIGHT: usize = 4;

/// Starts drawing `picture` on a thread of its own: its read streamed into
/// a decoder (`preview_decode`) where Qt's reader of its format reads in
/// order, and, where that is not so or the stream answers no picture, its
/// bytes read whole and decoded from memory. Answers the token that stops
/// the drawing's reads.
///
/// The thread is none of Qt's and runs no event loop: an animated SVG's
/// renderer cannot start its timer there, and Qt says so once per drawing
/// (`qsvgrenderer.cpp`) — the picture is its first frame either way.
fn draw(reader: &PictureReader, picture: PictureBlob, answer: Answer) -> CancellationToken {
    let (reader, stop) = reader.scoped();
    let stopped = stop.clone();
    // Past the stand-down the answer goes untold, which tells of no picture.
    let Some(drawing) = Drawing::start() else {
        return stop;
    };
    let drawn = std::thread::Builder::new()
        .name("pgg-picture".to_string())
        .spawn(move || {
            let _drawing = drawing;
            let format = picture.format();
            let streamed = match format
                .is_none_or(|format| crate::preview_decode::streams(qt_name(format).as_bytes()))
            {
                true => stream(&reader, picture.clone(), format),
                false => None,
            };
            let image = streamed.or_else(|| match stopped.is_cancelled() {
                true => None,
                false => read_whole(&reader, picture),
            });
            answer.tell(image.map(for_texture));
        });
    // The thread not started took the answer with it, which tells of no
    // picture.
    if let Err(error) = drawn {
        tracing::warn!(%error, "no thread to draw a preview picture on");
    }
    stop
}

/// `picture` decoded as it is streamed, on the calling thread, which waits
/// on the read. The decoder stops this read alone where it leaves it: the
/// drawing goes on to read the picture whole.
fn stream(
    reader: &PictureReader,
    picture: PictureBlob,
    format: Option<ImageFormat>,
) -> Option<QImage> {
    let (reader, stop) = reader.scoped();
    let (to, chunks) = tokio::sync::mpsc::channel(CHUNKS_IN_FLIGHT);
    let whole = Arc::new(AtomicBool::new(false));
    let told = Arc::clone(&whole);
    reader.stream(picture, to, move |streamed| match streamed {
        Ok(()) => told.store(true, Ordering::Release),
        Err(error) => tracing::debug!(%error, "preview picture not streamed"),
    });
    crate::preview_decode::decode(chunks, whole, format.map(qt_name), &stop)
}

/// The drawings under way, each on a thread of its own, and whether more
/// may start ([`stand_down`]).
static DRAWINGS: (Mutex<Drawings>, Condvar) = (
    Mutex::new(Drawings {
        under_way: 0,
        stood_down: false,
    }),
    Condvar::new(),
);

struct Drawings {
    under_way: usize,
    stood_down: bool,
}

fn drawings() -> MutexGuard<'static, Drawings> {
    DRAWINGS.0.lock().unwrap_or_else(PoisonError::into_inner)
}

/// One drawing under way, counted while it lives.
struct Drawing;

impl Drawing {
    /// `None` once the drawings have stood down.
    fn start() -> Option<Self> {
        let mut drawings = drawings();
        if drawings.stood_down {
            return None;
        }
        drawings.under_way += 1;
        Some(Self)
    }
}

impl Drop for Drawing {
    fn drop(&mut self) {
        let mut drawings = drawings();
        drawings.under_way = drawings.under_way.saturating_sub(1);
        DRAWINGS.1.notify_all();
    }
}

/// Starts no more drawings, and waits for those under way to end, their
/// reads stopped with the sessions — what is left is the decoding in hand.
/// Qt has to outlive them: a decoder ended mid-read by `exit` on Windows
/// can die holding a lock Qt's teardown then waits on for good, and on
/// Linux `exit` takes Qt's statics down under one still running (`main`).
pub fn stand_down() {
    let mut drawings = drawings();
    drawings.stood_down = true;
    while drawings.under_way > 0 {
        drawings = DRAWINGS
            .1
            .wait(drawings)
            .unwrap_or_else(PoisonError::into_inner);
    }
}

/// `picture`'s bytes read whole and decoded from memory, on the calling
/// thread, which waits on the read: the side's compressed bytes are held
/// until decoded (ci/baseline/preview-pictures-windows-x64.md).
fn read_whole(reader: &PictureReader, picture: PictureBlob) -> Option<QImage> {
    let format = picture.format();
    let (said, heard) = std::sync::mpsc::channel();
    reader.read(picture, move |read| {
        // The asker gone is nobody left to tell.
        said.send(read).ok();
    });
    match heard.recv() {
        Ok(Ok(bytes)) => decode(&bytes, format),
        Ok(Err(error)) => {
            tracing::debug!(%error, "preview picture not read");
            None
        }
        Err(_) => None,
    }
}

/// `Qt::ImageConversionFlags`, a `QFlags` over the `int` this wraps.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct ImageConversionFlags(i32);

impl ImageConversionFlags {
    /// `Qt::AutoColor`, Qt's own default.
    const AUTO_COLOR: Self = Self(0);
}

// SAFETY: `QFlags<Qt::ImageConversionFlag>` holds one `int` and is
// trivially copied, as this is.
unsafe impl cxx::ExternType for ImageConversionFlags {
    type Id = cxx::type_id!("Qt::ImageConversionFlags");
    type Kind = cxx::kind::Trivial;
}

/// `image` as Qt Quick's texture takes it: an ARGB32 picture — an RGBA
/// PNG's, an icon's — premultiplied in place. Handed one as it is, the
/// texture factory converts a copy while the response still holds the
/// original: the picture's decoded size again, at once, for each side drawn
/// (`QQuickDefaultTextureFactory` in `qquickpixmapcache.cpp`;
/// ci/baseline/preview-pictures-windows-x64.md). A format of another depth
/// takes a second buffer wherever it is converted, and is left to the
/// factory. Qt's own file reader turns an opaque picture to RGB32 in place
/// instead, a texture that draws the same.
fn for_texture(mut image: QImage) -> QImage {
    use cxx_qt_lib::QImageFormat as Format;
    if image.format() == Format::Format_ARGB32 {
        // SAFETY: `QImageMembers` is the C++ class `cxx_qt_lib::QImage` is
        // laid out as; `image` is a live one, held only here.
        let members = unsafe {
            Pin::new_unchecked(
                &mut *std::ptr::from_mut(&mut image).cast::<qobject::QImageMembers>(),
            )
        };
        members.convert_to(
            Format::Format_ARGB32_Premultiplied,
            ImageConversionFlags::AUTO_COLOR,
        );
    }
    image
}

/// A picture decoded on a thread of its own, on its way to its response's.
struct Decoded(QImage);

// SAFETY: a `QImage` is a value, moved whole to the one thread that holds
// it next; nothing else holds this one.
unsafe impl Send for Decoded {}

/// A drawing's word to its response, told on the response's thread: the
/// picture, or none, and `finished` either way. Dropped untold — the
/// drawing never ran — it tells of no picture, since Qt waits for every
/// response to finish.
struct Answer {
    thread: Option<CxxQtThread<qobject::PreviewImageResponse>>,
}

impl Answer {
    fn tell(mut self, image: Option<QImage>) {
        if let Some(thread) = self.thread.take() {
            finish(&thread, image.map(Decoded));
        }
    }
}

impl Drop for Answer {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            finish(&thread, None);
        }
    }
}

/// Finishes the response on its thread with `image`.
fn finish(thread: &CxxQtThread<qobject::PreviewImageResponse>, image: Option<Decoded>) {
    let queued = thread.queue(move |mut response| {
        if let Some(Decoded(image)) = image {
            response.as_mut().rust_mut().image = image;
        }
        response.finished();
    });
    // Qt let the response go: nobody is left to tell.
    queued.ok();
}

/// The host part of the provider's URLs.
const PROVIDER: &str = "pgg-preview";

/// One picture held: its object, and what reads it — the session of the
/// tab that read its diff, `None` where that tab had none open.
#[derive(Clone)]
struct Held {
    picture: PictureBlob,
    reader: Option<PictureReader>,
}

/// The pictures held, by id. Asked from Qt's image-loading thread, so
/// behind a lock rather than in the GUI thread's objects.
static HELD: Mutex<BTreeMap<String, Held>> = Mutex::new(BTreeMap::new());

fn held() -> MutexGuard<'static, BTreeMap<String, Held>> {
    HELD.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The start of every id the tab `tab` holds under.
fn tab_prefix(tab: i32) -> String {
    format!("t{tab}-")
}

/// Holds `picture` for a pane of the tab `tab`, to be read by `reader`,
/// answering the id it is held under.
pub fn hold(tab: i32, picture: PictureBlob, reader: Option<PictureReader>) -> String {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let id = format!(
        "{}{}",
        tab_prefix(tab),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    held().insert(id.clone(), Held { picture, reader });
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

/// The picture held under `id`: the lock is let go before anything is
/// read.
fn picture(id: &str) -> Option<Held> {
    held().get(id).cloned()
}

/// `bytes` decoded, read first as `format`, the one their path names. That
/// is a hint, as a file's suffix is: bytes the named format cannot read are
/// read again by their own signature — `QImage::fromData` does not read
/// past a format named outright.
fn decode(bytes: &[u8], format: Option<ImageFormat>) -> Option<QImage> {
    format
        .and_then(|format| QImage::from_data(bytes, Some(qt_name(format))))
        .or_else(|| QImage::from_data(bytes, None))
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
        let asynchronous: *const qobject::QQuickAsyncImageProvider = Upcast::upcast_ptr(provider);
        Upcast::upcast_ptr(qobject::as_quick_image_provider(asynchronous))
    };
    // SAFETY: `QQmlEngine::addImageProvider` takes ownership of the
    // provider, which nothing else holds.
    unsafe { engine.add_image_provider(&QString::from(PROVIDER), base.cast_mut()) };
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use platitude_core::preview::{ImageFormat, PictureBlob, PictureReader};
    use platitude_core::process::GitCommand;
    use platitude_core::{GitExecutor, Oid};
    use tokio_util::sync::CancellationToken;

    /// One pixel, to load.
    const PNG: [u8; 70] = [
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0x64,
        0x60, 0xf8, 0x5f, 0x0f, 0x00, 0x02, 0x87, 0x01, 0x80, 0xeb, 0x47, 0xba, 0x92, 0x00, 0x00,
        0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];

    /// The pixel as an icon: one entry, the PNG itself as its image — which
    /// Qt's reader seeks to.
    fn ico() -> Vec<u8> {
        let mut ico = vec![0, 0, 1, 0, 1, 0, 1, 1, 0, 0, 1, 0, 32, 0];
        ico.extend_from_slice(&(PNG.len() as u32).to_le_bytes());
        ico.extend_from_slice(&22u32.to_le_bytes());
        ico.extend_from_slice(&PNG);
        ico
    }

    /// A drawing of 3 × 2 whose `<svg` stands past the bytes Qt's reader
    /// peeks at to tell an SVG: told by its name alone.
    fn svg() -> Vec<u8> {
        let mut svg = b"<?xml version=\"1.0\"?>\n<!--".to_vec();
        svg.resize(svg.len() + 5000, b' ');
        svg.extend_from_slice(
            b"-->\n<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"3\" height=\"2\"/>\n",
        );
        svg
    }

    /// An object no repository here holds.
    fn absent() -> Oid {
        Oid::from_hex_str("0123456789abcdef0123456789abcdef01234567").expect("an object id")
    }

    /// What is held under `id`.
    fn picture(id: &str) -> Option<PictureBlob> {
        super::picture(id).map(|held| held.picture)
    }

    /// An ARGB32 picture goes to its texture premultiplied in its own
    /// buffer — the pixels Qt's premultiplied format holds for its colours —
    /// and a picture of a format the texture takes goes as it is.
    #[test]
    fn an_argb32_picture_is_premultiplied_in_its_own_buffer() {
        use cxx_qt_lib::{QColor, QImage, QImageFormat};
        let colours = [
            QColor::from_rgba(255, 0, 0, 255),
            QColor::from_rgba(255, 128, 0, 128),
            QColor::from_rgba(0, 0, 255, 0),
        ];
        let painted = |format: QImageFormat| {
            let mut image = QImage::from_width_height_and_format(3, 1, format);
            for (x, colour) in (0..).zip(&colours) {
                image.set_pixel_color(x, 0, colour);
            }
            image
        };
        let straight = painted(QImageFormat::Format_ARGB32);
        // The buffer's serial number, which a buffer of its own renews.
        let buffer = straight.cache_key() >> 32;
        let drawn = super::for_texture(straight);
        assert_eq!(drawn.cache_key() >> 32, buffer, "the same buffer");
        assert_eq!(drawn, painted(QImageFormat::Format_ARGB32_Premultiplied));

        let opaque = painted(QImageFormat::Format_RGB32);
        let key = opaque.cache_key();
        assert_eq!(super::for_texture(opaque).cache_key(), key, "left alone");
    }

    /// Every hold is a new id, so a re-read is a new URL; a tab let go of
    /// takes its own pictures and no other tab's — `t1-` is not `t12-`'s
    /// start.
    #[test]
    fn a_picture_goes_by_its_id_or_with_its_tab() {
        let at = |path: &str| PictureBlob::new(absent(), 70, path);
        let first = super::hold(1, at("logo.PNG"), None);
        let again = super::hold(1, at("logo.png"), None);
        let other = super::hold(12, at("photo.jpg"), None);
        assert_ne!(first, again);
        assert_eq!(super::url(&first), format!("image://pgg-preview/{first}"));
        assert_eq!(picture(&first), Some(at("logo.PNG")));

        super::release(&first);
        assert_eq!(picture(&first), None);
        assert!(picture(&again).is_some());

        super::release_tab(1);
        assert_eq!(picture(&again), None);
        assert_eq!(
            picture(&other).and_then(|picture| picture.format()),
            Some(ImageFormat::Jpeg)
        );
        super::release_tab(12);
    }

    /// The git the child reads with, kept off this machine's configuration.
    fn executor(dir: &Path) -> GitExecutor {
        GitExecutor::new().with_env([
            ("GIT_CONFIG_GLOBAL", dir.join("no-config").into_os_string()),
            ("GIT_CONFIG_NOSYSTEM", "1".into()),
            (
                "XDG_CONFIG_HOME",
                dir.join("no-config-home").into_os_string(),
            ),
        ])
    }

    /// A repository at `dir/repo` holding each of `contents` as an object,
    /// and their objects, in order.
    async fn repository(
        executor: &GitExecutor,
        dir: &Path,
        contents: &[&[u8]],
    ) -> (PathBuf, Vec<Oid>) {
        let repo = dir.join("repo");
        std::fs::create_dir_all(&repo).expect("the repository's folder");
        let cancel = CancellationToken::new();
        executor
            .run(GitCommand::new().cwd(&repo).args(["init", "-q"]), &cancel)
            .await
            .expect("git init");
        let mut oids = Vec::new();
        for bytes in contents {
            std::fs::write(repo.join("content"), bytes).expect("the content");
            let out = executor
                .run(
                    GitCommand::new()
                        .cwd(&repo)
                        .args(["hash-object", "-w", "content"]),
                    &cancel,
                )
                .await
                .expect("the content's object");
            oids.push(Oid::from_hex_str(out.stdout_utf8().trim()).expect("an object id"));
        }
        (repo, oids)
    }

    /// A picture whose format streams is drawn from its stream, with no
    /// whole read behind it: its read is told whole before its chunks stop.
    #[test]
    fn a_picture_that_streams_is_drawn_from_its_stream() {
        let dir = crate::temp::root().join(format!("pgg-preview-stream-{}", std::process::id()));
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("a runtime");
        let executor = executor(&dir);
        let (repo, oids) = runtime.block_on(repository(&executor, &dir, &[&PNG]));
        let reader = PictureReader::new(
            executor,
            repo,
            runtime.handle().clone(),
            CancellationToken::new(),
        );
        let picture = PictureBlob::new(oids[0], 70, "logo.png");
        let image = super::stream(&reader, picture, Some(ImageFormat::Png));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(
            image.map(|image| (image.width(), image.height())),
            Some((1, 1))
        );
    }

    /// The child: a Qt application with the provider installed, holding
    /// pictures by their objects with a reader on its own runtime — and one
    /// with none, one whose reader was stopped, one let go — writing the
    /// QML that reads them by their ids and saying how each `Image` came
    /// out.
    #[test]
    #[ignore = "started by held_pictures_are_read_from_git_through_the_provider, offscreen, as a process of its own"]
    fn provider_side() {
        let Some(dir) = std::env::var_os("PREVIEW_IMAGES_DIR") else {
            return;
        };
        let dir = PathBuf::from(dir);
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("a runtime");
        let executor = executor(&dir);
        let (ico, svg) = (ico(), svg());
        let (repo, oids) = runtime.block_on(repository(&executor, &dir, &[&PNG, &ico, &svg]));
        let (oid, icon, drawing) = (oids[0], oids[1], oids[2]);
        let reader = |cancel: CancellationToken| {
            PictureReader::new(
                executor.clone(),
                repo.clone(),
                runtime.handle().clone(),
                cancel,
            )
        };
        let live = Some(reader(CancellationToken::new()));
        let stopped = CancellationToken::new();
        stopped.cancel();
        let mut app = qtbridge::qtbridge_type_lib::QGuiApplication::new();
        let mut engine = qtbridge::qtbridge_type_lib::QQmlApplicationEngine::new();
        crate::qml_engine::arm(engine.pin_mut());
        super::install(engine.pin_mut());
        let blob = |oid: Oid, path: &str| PictureBlob::new(oid, 70, path);
        let one = super::hold(7, blob(oid, "logo.png"), live.clone());
        let misnamed = super::hold(7, blob(oid, "photo.jpg"), live.clone());
        let unnamed = super::hold(7, blob(oid, "data.bin"), live.clone());
        let icon_named = super::hold(7, blob(icon, "favicon.ico"), live.clone());
        let icon_misnamed = super::hold(7, blob(icon, "favicon.png"), live.clone());
        let drawing = super::hold(
            7,
            PictureBlob::new(drawing, svg.len() as u64, "drawing.svg"),
            live.clone(),
        );
        let unread = super::hold(7, blob(absent(), "logo.png"), live.clone());
        let closed = super::hold(7, blob(oid, "logo.png"), Some(reader(stopped)));
        let unserved = super::hold(7, blob(oid, "logo.png"), None);
        let gone = super::hold(7, blob(oid, "logo.png"), live);
        super::release(&gone);
        let urls = [
            ("one", super::url(&one)),
            ("misnamed", super::url(&misnamed)),
            ("unnamed", super::url(&unnamed)),
            ("icon", super::url(&icon_named)),
            ("misnamed icon", super::url(&icon_misnamed)),
            ("drawing", super::url(&drawing)),
            ("unread", super::url(&unread)),
            ("closed", super::url(&closed)),
            ("unserved", super::url(&unserved)),
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
        // `main`'s order: the reads stop, the drawings end, then Qt goes.
        runtime.shutdown_background();
        super::stand_down();
        drop(engine);
        qtbridge::collect_garbage();
        drop(app);
    }

    /// A blob held by its object shows in an `Image` through
    /// `image://pgg-preview/<id>`, read from git by its reader when the
    /// `Image` asks, while Qt's image-loading thread goes on — under an
    /// extension that names another format too, as a misnamed file would,
    /// and from a path that names none; an icon, whose reader seeks, read
    /// whole by its name, and streamed first where its name says otherwise;
    /// a drawing its bytes do not tell soon enough, by its name. An object
    /// git cannot read, a reader whose session closed, a picture no session
    /// can read, an id let go and one never held are each an error, not a
    /// picture, and none leaves its `Image` loading.
    #[test]
    fn held_pictures_are_read_from_git_through_the_provider() {
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
                "icon\t1\t1x1",
                "misnamed icon\t1\t1x1",
                "drawing\t1\t3x2",
                "unread\t3\t0x0",
                "closed\t3\t0x0",
                "unserved\t3\t0x0",
                "gone\t3\t0x0",
                "none\t3\t0x0"
            ],
            "{errors}"
        );
    }
}
