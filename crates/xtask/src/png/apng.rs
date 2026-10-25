//! An animated PNG put together from PNG frames of one shape, chunk by
//! chunk: each frame's own compressed stream is carried over whole
//! (`IDAT` for the first, `fdAT` for the rest), so nothing is decoded or
//! compressed again and the frames come out byte for byte.

use super::checksum::crc32;
use super::write::chunk;

const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// One frame: a whole PNG file, and how long it stands.
pub(crate) struct Frame<'a> {
    pub(crate) file: &'a [u8],
    pub(crate) ms: u32,
}

/// The frames as one APNG that loops for ever. Every frame has the first
/// one's header: an APNG frame here covers the whole picture.
pub(crate) fn animate(frames: &[Frame]) -> Result<Vec<u8>, String> {
    let first = frames.first().ok_or("an animation needs a frame")?;
    let (header, _) = parts(first.file)?;
    let mut out = SIGNATURE.to_vec();
    chunk(&mut out, b"IHDR", &header);
    let mut control = Vec::with_capacity(8);
    control.extend_from_slice(
        &u32::try_from(frames.len())
            .map_err(|e| e.to_string())?
            .to_be_bytes(),
    );
    control.extend_from_slice(&0u32.to_be_bytes());
    chunk(&mut out, b"acTL", &control);
    let mut sequence = 0u32;
    for (index, frame) in frames.iter().enumerate() {
        let (own, data) = parts(frame.file)?;
        if own != header {
            return Err(format!("frame {index} is not the first frame's shape"));
        }
        chunk(
            &mut out,
            b"fcTL",
            &frame_control(sequence, &header, frame.ms),
        );
        sequence += 1;
        for body in data {
            if index == 0 {
                chunk(&mut out, b"IDAT", body);
                continue;
            }
            let mut numbered = Vec::with_capacity(body.len() + 4);
            numbered.extend_from_slice(&sequence.to_be_bytes());
            numbered.extend_from_slice(body);
            chunk(&mut out, b"fdAT", &numbered);
            sequence += 1;
        }
    }
    chunk(&mut out, b"IEND", &[]);
    Ok(out)
}

/// The image data of a PNG, chunk bodies in order: two frames that carry
/// the same are the same picture.
pub(crate) fn image_data(file: &[u8]) -> Result<Vec<&[u8]>, String> {
    parts(file).map(|(_, data)| data)
}

/// A frame's header body and its image data chunks. A palette picture is
/// refused: two frames could carry two palettes, and an APNG has one.
fn parts(file: &[u8]) -> Result<(Vec<u8>, Vec<&[u8]>), String> {
    if file.get(..8) != Some(&SIGNATURE[..]) {
        return Err("not a png: the file does not open with the signature".into());
    }
    let mut header = None;
    let mut data = Vec::new();
    let mut at = 8;
    while at + 8 <= file.len() {
        let length = u32::from_be_bytes(word(file, at)?) as usize;
        let kind = &file[at + 4..at + 8];
        let body = file
            .get(at + 8..at + 8 + length)
            .ok_or("a chunk ends before the bytes it claims")?;
        if u32::from_be_bytes(word(file, at + 8 + length)?) != crc32(&file[at + 4..at + 8 + length])
        {
            return Err(format!(
                "the {} chunk fails its own checksum",
                String::from_utf8_lossy(kind)
            ));
        }
        match kind {
            b"IHDR" if body.len() == 13 && body[9] == 3 => {
                return Err("a palette picture cannot be one frame of several".into());
            }
            b"IHDR" => header = Some(body.to_vec()),
            b"IDAT" => data.push(body),
            b"IEND" => break,
            _ => {}
        }
        at += 12 + length;
    }
    let header = header.ok_or("the file carries no header chunk")?;
    if data.is_empty() {
        return Err("the file carries no image data".into());
    }
    Ok((header, data))
}

fn word(file: &[u8], at: usize) -> Result<[u8; 4], String> {
    file.get(at..at + 4)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| "the file ends inside a chunk".to_string())
}

/// `fcTL`: the whole picture, standing `ms`, put over what was there.
/// A delay past what sixteen bits of milliseconds hold is said in
/// hundredths.
fn frame_control(sequence: u32, header: &[u8], ms: u32) -> Vec<u8> {
    let (numerator, denominator) = match u16::try_from(ms) {
        Ok(ms) => (ms, 1000u16),
        Err(_) => (u16::try_from(ms / 10).unwrap_or(u16::MAX), 100u16),
    };
    let mut body = Vec::with_capacity(26);
    body.extend_from_slice(&sequence.to_be_bytes());
    // Width and height, as the header has them.
    body.extend_from_slice(&header[0..8]);
    body.extend_from_slice(&[0; 8]);
    body.extend_from_slice(&numerator.to_be_bytes());
    body.extend_from_slice(&denominator.to_be_bytes());
    // Dispose: none. Blend: source — each frame replaces the picture.
    body.extend_from_slice(&[0, 0]);
    body
}

#[cfg(test)]
mod tests {
    use super::{Frame, animate, image_data, parts};
    use crate::png::{decode, rgba};

    fn chunks(file: &[u8]) -> Vec<(String, Vec<u8>)> {
        let mut out = Vec::new();
        let mut at = 8;
        while at < file.len() {
            let length = u32::from_be_bytes(file[at..at + 4].try_into().expect("length")) as usize;
            let kind = String::from_utf8_lossy(&file[at + 4..at + 8]).into_owned();
            out.push((kind, file[at + 8..at + 8 + length].to_vec()));
            at += 12 + length;
        }
        out
    }

    #[test]
    fn frames_go_in_whole_and_numbered_in_one_sequence() {
        let red = rgba(3, 2, |_, _| [255, 0, 0, 255]);
        let blue = rgba(3, 2, |_, _| [0, 0, 255, 255]);
        let film = animate(&[
            Frame { file: &red, ms: 40 },
            Frame {
                file: &blue,
                ms: 70_000,
            },
            Frame {
                file: &red,
                ms: 1600,
            },
        ])
        .expect("animates");
        let kinds: Vec<String> = chunks(&film).into_iter().map(|(kind, _)| kind).collect();
        assert_eq!(
            kinds,
            [
                "IHDR", "acTL", "fcTL", "IDAT", "fcTL", "fdAT", "fcTL", "fdAT", "IEND"
            ]
        );
        let all = chunks(&film);
        assert_eq!(all[1].1[..4], 3u32.to_be_bytes());
        // fcTL 0, fcTL 1, fdAT 2, fcTL 3, fdAT 4.
        let numbers: Vec<u32> = all
            .iter()
            .filter(|(kind, _)| kind == "fcTL" || kind == "fdAT")
            .map(|(_, body)| u32::from_be_bytes(body[..4].try_into().expect("number")))
            .collect();
        assert_eq!(numbers, [0, 1, 2, 3, 4]);
        // 70 s stands as 7000 hundredths.
        assert_eq!(all[4].1[20..24], [0x1b, 0x58, 0, 100]);
        assert_eq!(all[5].1[4..], image_data(&blue).expect("data")[0][..]);
        // A reader that knows no animation sees the first frame.
        assert_eq!(
            decode(&film).expect("decodes").pixel(2, 1),
            [255, 0, 0, 255]
        );
    }

    #[test]
    fn a_frame_of_another_shape_is_refused() {
        let small = rgba(3, 2, |_, _| [0; 4]);
        let large = rgba(4, 2, |_, _| [0; 4]);
        let frames = [
            Frame {
                file: &small,
                ms: 40,
            },
            Frame {
                file: &large,
                ms: 40,
            },
        ];
        assert!(animate(&frames).is_err());
        assert!(parts(b"not a png").is_err());
    }
}
