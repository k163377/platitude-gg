//! A picture decoded as git streams it
//! (`platitude_core::preview::PictureReader::stream`): Qt's image reader
//! pulls the blob through a device ([`qobject::PictureStream`]) that waits
//! on the read for each chunk, so no more of the picture's compressed bytes
//! stand in memory than the chunks in flight — the read waits while the
//! decoder is behind — but for a gzipped SVG, which Qt's reader reads whole
//! (`qsvgiohandler.cpp`), as the whole read would.
//!
//! Read as the format the path names where the bytes agree, else as the one
//! the bytes say, else as the one the path names — the formats the whole
//! read tries (`preview_images`), told apart by a peek at the bytes, which
//! a stream cannot read twice — and only where Qt's reader of that format
//! reads its device in order ([`streams`]). Bytes of another format, bytes
//! the format cannot read and a stream broken off answer no picture, and
//! the caller reads the bytes whole.

#![expect(
    unsafe_code,
    reason = "Qt's image reader takes its device by pointer, and the device answers it \
              into a buffer Qt hands over by pointer"
)]

use std::cell::RefCell;
use std::ffi::c_char;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cxx_qt::CxxQtType;
use cxx_qt::casting::Upcast;
use cxx_qt_lib::{QByteArray, QImage, qint64};
use tokio::sync::mpsc::Receiver;
use tokio_util::sync::CancellationToken;

#[cxx_qt::bridge]
mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qimage.h");
        type QImage = cxx_qt_lib::QImage;
        include!("cxx-qt-lib/qbytearray.h");
        type QByteArray = cxx_qt_lib::QByteArray;
        include!("cxx-qt-lib/qtypes.h");
        type qint64 = cxx_qt_lib::qint64;

        include!(<QtGui/QImageReader>);
        /// Qt's image reader, over one device.
        type QImageReader;

        /// The format read as, by Qt's name: the one the reader was made
        /// with, else the one the bytes say — told from a peek at them,
        /// which leaves the device where it was; empty where no reader
        /// knows them.
        fn format(self: &QImageReader) -> QByteArray;

        /// Whether the bytes are of the format read as — told from a peek
        /// at them, which leaves the device where it was.
        #[rust_name = "can_read"]
        fn canRead(self: &QImageReader) -> bool;

        /// The image the device holds; null where it cannot be read.
        fn read(self: Pin<&mut QImageReader>) -> QImage;
    }

    extern "C++Qt" {
        include!(<QtCore/QIODevice>);
        #[qobject]
        type QIODevice;
    }

    extern "RustQt" {
        /// The device Qt's reader pulls a streamed picture from.
        #[qobject]
        #[base = QIODevice]
        type PictureStream = super::PictureStreamRust;

        #[cxx_override]
        #[cxx_name = "readData"]
        unsafe fn read_data(
            self: Pin<&mut PictureStream>,
            data: *mut c_char,
            max: qint64,
        ) -> qint64;

        #[cxx_override]
        #[cxx_name = "writeData"]
        unsafe fn write_data(
            self: Pin<&mut PictureStream>,
            data: *const c_char,
            len: qint64,
        ) -> qint64;

        #[cxx_override]
        #[cxx_name = "isSequential"]
        fn is_sequential(self: &PictureStream) -> bool;

        #[cxx_override]
        #[cxx_name = "bytesAvailable"]
        fn bytes_available(self: &PictureStream) -> qint64;
    }

    unsafe extern "RustQt" {
        /// What Qt's own buffer holds of the stream.
        #[inherit]
        #[cxx_name = "bytesAvailable"]
        fn buffered(self: &PictureStream) -> qint64;
    }

    #[namespace = "rust::cxxqtlib1"]
    unsafe extern "C++" {
        include!("cxx-qt-lib/common.h");

        #[rust_name = "new_picture_stream"]
        fn make_unique() -> UniquePtr<PictureStream>;

        /// A reader over `device`, of the format named, or of the one the
        /// bytes say where the name is empty.
        #[rust_name = "new_image_reader"]
        unsafe fn make_unique(
            device: *mut QIODevice,
            format: QByteArray,
        ) -> UniquePtr<QImageReader>;
    }
}

/// The device's side of the stream. Qt asks of it through `&self` too
/// (`bytesAvailable`), on the one thread that decodes.
#[derive(Default)]
pub struct PictureStreamRust {
    feed: RefCell<Feed>,
}

/// The read's chunks as they come, the one being handed over, whether the
/// decoder has met the read's end, and whether the read ended whole.
#[derive(Default)]
struct Feed {
    chunks: Option<Receiver<Vec<u8>>>,
    chunk: Vec<u8>,
    at: usize,
    ended: bool,
    whole: Option<Arc<AtomicBool>>,
}

impl Feed {
    /// How many bytes of the chunk in hand are left, waiting for the read's
    /// next where it is spent: none once the read has ended.
    fn ready(&mut self) -> usize {
        while self.at == self.chunk.len() {
            let Some(next) = self.chunks.as_mut().and_then(Receiver::blocking_recv) else {
                self.ended = true;
                return 0;
            };
            self.chunk = next;
            self.at = 0;
        }
        self.chunk.len() - self.at
    }

    fn ended_whole(&self) -> bool {
        self.whole
            .as_ref()
            .is_some_and(|whole| whole.load(Ordering::Acquire))
    }
}

impl qobject::PictureStream {
    /// Hands the reader `max` bytes, across as many of the read's chunks as
    /// that takes, waiting on each: Qt takes a read shorter than it asked
    /// for as the end of the device (`QIODevicePrivate::read`). Fewer only
    /// at the end; none once the read ended whole, an error where it did
    /// not.
    ///
    /// # Safety
    ///
    /// `data` has room for `max` bytes.
    unsafe fn read_data(self: Pin<&mut Self>, data: *mut c_char, max: qint64) -> qint64 {
        let room = usize::try_from(i64::from(max)).unwrap_or(0);
        if room == 0 {
            return qint64::from(0);
        }
        let mut feed = self.rust().feed.borrow_mut();
        let mut given = 0;
        while given < room {
            let n = feed.ready().min(room - given);
            if n == 0 {
                break;
            }
            let from = &feed.chunk[feed.at..feed.at + n];
            // SAFETY: Qt hands `data` with room for `max` bytes, and
            // `given + n` is no more than `max`.
            unsafe {
                std::ptr::copy_nonoverlapping(from.as_ptr().cast::<c_char>(), data.add(given), n);
            }
            feed.at += n;
            given += n;
        }
        match given {
            0 if !feed.ended_whole() => qint64::from(-1),
            given => qint64::from(i64::try_from(given).unwrap_or(i64::MAX)),
        }
    }

    /// Nothing is written to a picture.
    unsafe fn write_data(self: Pin<&mut Self>, _data: *const c_char, _len: qint64) -> qint64 {
        qint64::from(-1)
    }

    /// The chunks go by once.
    fn is_sequential(&self) -> bool {
        true
    }

    /// What Qt's buffer holds and what is left of the chunk in hand —
    /// waiting for the read's next where it is spent: a stream between two
    /// chunks is not at its end, which Qt tells by this (`atEnd`, which
    /// GIF's reader asks of a frame cut short and SVG's XML reader of a
    /// document — `qgifhandler.cpp`, `qxmlstream.cpp`).
    fn bytes_available(&self) -> qint64 {
        let ready = self.rust().feed.borrow_mut().ready();
        let ready = i64::try_from(ready).unwrap_or(i64::MAX);
        qint64::from(i64::from(self.buffered()).saturating_add(ready))
    }
}

/// Whether Qt's reader of the format it names `name` reads its device in
/// order, as the stream hands it over: PNG's, JPEG's, GIF's and SVG's do.
/// BMP's, on a stream, skips its seek to the pixels and reads them from
/// where its headers ended, a picture shifted where the file leaves a gap;
/// ICO's and TIFF's seek; WebP's takes a stream only once all of it is in
/// hand (`qbmphandler.cpp`, `qicohandler.cpp`, `qtiffhandler.cpp`,
/// `qwebphandler.cpp`). AVIF's is no reader of Qt's, so how it reads is not
/// known.
pub fn streams(name: &[u8]) -> bool {
    matches!(name, b"png" | b"jpeg" | b"gif" | b"svg" | b"svgz")
}

/// The picture the read sends, decoded as its chunks arrive: as `hint` (a
/// name Qt's readers know) where the bytes agree, else as the format they
/// say, else, where no reader knows them, as `hint`; `None` where that
/// format does not stream, where its reader cannot read the bytes, or where
/// the decoder met the read's end and the read did not end whole (`whole`,
/// told before the chunks stop) — a reader handed a stream broken off can
/// answer what it had (JPEG's fills in the rest). A picture done before the
/// read's end — bytes past it, a GIF's later frames — needs no more.
///
/// The read is stopped through `stop` before its chunks are let go, so a
/// read the decoder leaves ends stopped, not broken off. Runs on the
/// calling thread, which waits on the read — never one of Qt's.
pub fn decode(
    chunks: Receiver<Vec<u8>>,
    whole: Arc<AtomicBool>,
    hint: Option<&str>,
    stop: &CancellationToken,
) -> Option<QImage> {
    let stream = qobject::new_picture_stream();
    // Dropped ahead of `stream`, whose chunks go with it.
    let _stop = stop.clone().drop_guard();
    *stream.rust().feed.borrow_mut() = Feed {
        chunks: Some(chunks),
        whole: Some(Arc::clone(&whole)),
        ..Feed::default()
    };
    // SAFETY: `stream` is a live object; the upcast is to the class it
    // derives from.
    let device = unsafe {
        <qobject::PictureStream as Upcast<qobject::QIODevice>>::upcast_ptr(stream.as_ptr())
    }
    .cast_mut();
    // SAFETY: `device` is `stream`'s, which outlives every reader made here.
    let reader_of =
        |format: &str| unsafe { qobject::new_image_reader(device, QByteArray::from(format)) };
    let mut reader = match hint.map(reader_of) {
        Some(named) if named.can_read() => named,
        named => match reader_of("") {
            said if said.format().is_empty() => named?,
            said => said,
        },
    };
    if !streams(reader.format().as_slice()) {
        return None;
    }
    let image = reader.pin_mut().read();
    if image.is_null() {
        return None;
    }
    let feed = stream.rust().feed.borrow();
    (!feed.ended || feed.ended_whole()).then_some(image)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use cxx_qt_lib::QImage;
    use tokio_util::sync::CancellationToken;

    /// CRC-32 (ISO-HDLC), as a PNG's chunks carry it.
    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = !0u32;
        for &byte in bytes {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xedb8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    /// An RGB PNG of `width` × `height`, every pixel told apart by where it
    /// stands, its pixels stored uncompressed: as many bytes as pixels.
    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut pixels = Vec::new();
        for y in 0..height {
            pixels.push(0);
            for x in 0..width {
                pixels.extend_from_slice(&[x as u8, y as u8, (x ^ y) as u8]);
            }
        }
        let mut zlib = vec![0x78, 0x01];
        let blocks: Vec<&[u8]> = pixels.chunks(0xffff).collect();
        for (i, block) in blocks.iter().enumerate() {
            zlib.push(u8::from(i + 1 == blocks.len()));
            let len = block.len() as u16;
            zlib.extend_from_slice(&len.to_le_bytes());
            zlib.extend_from_slice(&(!len).to_le_bytes());
            zlib.extend_from_slice(block);
        }
        let (mut a, mut b) = (1u32, 0u32);
        for &byte in &pixels {
            a = (a + u32::from(byte)) % 65521;
            b = (b + a) % 65521;
        }
        zlib.extend_from_slice(&((b << 16) | a).to_be_bytes());
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut chunk = |kind: &[u8], data: &[u8]| {
            png.extend_from_slice(&(data.len() as u32).to_be_bytes());
            let start = png.len();
            png.extend_from_slice(kind);
            png.extend_from_slice(data);
            let crc = crc32(&png[start..]);
            png.extend_from_slice(&crc.to_be_bytes());
        };
        let mut header = Vec::new();
        header.extend_from_slice(&width.to_be_bytes());
        header.extend_from_slice(&height.to_be_bytes());
        header.extend_from_slice(&[8, 2, 0, 0, 0]);
        chunk(b"IHDR", &header);
        chunk(b"IDAT", &zlib);
        chunk(b"IEND", &[]);
        png
    }

    /// A 24-bit BMP of `width` × `height`, its rows padded to four bytes.
    fn bmp(width: u32, height: u32) -> Vec<u8> {
        let pixels = (width * 3).next_multiple_of(4) * height;
        let mut bmp = b"BM".to_vec();
        for word in [54 + pixels, 0, 54, 40, width, height] {
            bmp.extend_from_slice(&word.to_le_bytes());
        }
        bmp.extend_from_slice(&1u16.to_le_bytes());
        bmp.extend_from_slice(&24u16.to_le_bytes());
        for word in [0, pixels, 2835, 2835, 0, 0] {
            bmp.extend_from_slice(&word.to_le_bytes());
        }
        bmp.resize(bmp.len() + pixels as usize, 0x80);
        bmp
    }

    /// What `decode` makes of `bytes`, sent `piece` bytes at a time from a
    /// thread of their own, the read told as ended whole or not once they
    /// are through.
    fn streamed(
        bytes: &[u8],
        piece: usize,
        ended_whole: bool,
        hint: Option<&str>,
    ) -> Option<QImage> {
        let (to, chunks) = tokio::sync::mpsc::channel(2);
        let whole = Arc::new(AtomicBool::new(false));
        let told = Arc::clone(&whole);
        let bytes = bytes.to_vec();
        let sender = std::thread::spawn(move || {
            for piece in bytes.chunks(piece) {
                if to.blocking_send(piece.to_vec()).is_err() {
                    return;
                }
            }
            told.store(ended_whole, Ordering::Release);
        });
        let image = super::decode(chunks, whole, hint, &CancellationToken::new());
        sender.join().expect("the sender");
        image
    }

    /// A picture streamed in pieces is the one its bytes make read whole —
    /// in pieces smaller than the peek that tells its format (PNG's eight
    /// bytes), and as one per git chunk; under its own name, under another
    /// format's, and with none.
    #[test]
    fn a_picture_streamed_in_pieces_is_the_one_read_whole() {
        let bytes = png(300, 200);
        let whole = QImage::from_data(&bytes, Some("png")).expect("the PNG read whole");
        for hint in [Some("png"), Some("jpeg"), None] {
            for piece in [5, 4096, 64 * 1024] {
                assert_eq!(
                    streamed(&bytes, piece, true, hint),
                    Some(whole.clone()),
                    "{piece}-byte pieces named {hint:?}"
                );
            }
        }
    }

    /// A stream the decoder read to its end is no picture where the read
    /// did not end whole: every byte came, or half of them.
    #[test]
    fn a_stream_not_ended_whole_is_no_picture() {
        let bytes = png(300, 200);
        assert_eq!(streamed(&bytes, 4096, false, Some("png")), None);
        assert_eq!(
            streamed(&bytes[..bytes.len() / 2], 4096, false, Some("png")),
            None
        );
    }

    /// A picture done before its read's end needs none of the rest: the
    /// read, still going and never to end whole, is stopped rather than
    /// read out.
    #[test]
    fn a_picture_done_before_its_read_ends_stops_the_read() {
        let bytes = png(300, 200);
        let whole = QImage::from_data(&bytes, Some("png")).expect("the PNG read whole");
        let (to, chunks) = tokio::sync::mpsc::channel(2);
        let sender = std::thread::spawn(move || {
            let past = vec![0; 1024 * 1024];
            for piece in bytes.chunks(4096).chain(past.chunks(4096)) {
                if to.blocking_send(piece.to_vec()).is_err() {
                    return;
                }
            }
        });
        let stop = CancellationToken::new();
        let image = super::decode(chunks, Arc::new(AtomicBool::new(false)), Some("png"), &stop);
        sender.join().expect("the sender");
        assert_eq!(image, Some(whole));
        assert!(stop.is_cancelled(), "the read is stopped");
    }

    /// Bytes whose head does not tell their format — a drawing whose `<svg`
    /// stands past the bytes Qt's reader peeks at — are read as their name
    /// says, and are no picture with no name.
    #[test]
    fn bytes_that_do_not_tell_their_format_are_read_as_their_name_says() {
        let mut bytes = b"<?xml version=\"1.0\"?>\n<!--".to_vec();
        bytes.resize(bytes.len() + 5000, b' ');
        bytes.extend_from_slice(
            b"-->\n<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"3\" height=\"2\"/>\n",
        );
        let drawn = streamed(&bytes, 4096, true, Some("svg")).expect("the drawing");
        assert_eq!((drawn.width(), drawn.height()), (3, 2));
        assert_eq!(streamed(&bytes, 4096, true, None), None);
    }

    /// Bytes of a format whose reader does not read in order are no
    /// picture streamed, though they are one read whole — under their own
    /// name, and under one that streams.
    #[test]
    fn bytes_of_a_format_that_does_not_stream_are_left_to_the_whole_read() {
        let bytes = bmp(4, 3);
        assert!(
            QImage::from_data(&bytes, None).is_some(),
            "the BMP read whole"
        );
        assert_eq!(streamed(&bytes, 4096, true, Some("bmp")), None);
        assert_eq!(streamed(&bytes, 4096, true, Some("png")), None);
    }
}
