//! The QML modules as Qt resources that carry a modification time.
//!
//! **Qt keeps a compiled QML file on disk only when its source has a
//! modification time** — a resource without one is compiled at every
//! start and never saved (`CompilationUnit::saveToDisk`: "Missing time
//! stamp for source file"), which is every file of `platitude.ui` and
//! `platitude.auto`, each start. `include_bytes_qml!` writes the time as
//! 0, so the tree is written here, in the same layout (rcc format 3, one
//! tree per file — `qtbase/src/tools/rcc`), with the time filled in.
//!
//! **The time is the file's content, hashed.** Qt reuses a compiled file
//! when the time it was compiled against equals the source's (with the Qt
//! build and the checksums of the types it depends on — `Unit::verifyHeader`,
//! `QQmlTypeData::done`), and the cache names its entry after the path alone
//! (`qrc:/qt/qml/platitude/ui/Main.qml` in every build). A time that moves
//! with the content alone is what makes a changed file compile again and an
//! unchanged one load as it was, whichever build or tree wrote the entry.
//!
//! **Only a start that was told where to keep them carries times**
//! (`QML_DISK_CACHE_PATH` — the runs the harness starts are given the
//! build's own `target/<profile>/qmlcache`, `xtask::verify::child`). Every
//! other start writes the times as 0, as `include_bytes_qml!` does: it
//! compiles what it loads and keeps nothing, the shipped app's own
//! behaviour, so nothing is left in a person's cache directory by a build
//! nobody asked to cache.
//!
//! **The untimed tree is built at compile time** ([`untimed`], a static
//! in the image's read-only data, where `include_bytes_qml!`'s arrays
//! stood): a tree built at start would be the whole of both modules again
//! on the heap, for the life of the process. Only a start that keeps its
//! compiled QML builds its timed copy there ([`tree`]).

/// Where the modules stand in the resource tree: `qrc:/qt/qml`, the import
/// path the engine is given, then the module's own directory.
pub const PREFIX: &str = "qt/qml/platitude";

/// Embeds a file of this directory at `qrc:/qt/qml/platitude/<path>`.
/// The path is read relative to the file the macro is written in, like
/// `include_bytes!` — main.rs, beside `ui/` and `auto/`.
macro_rules! embed {
    ($path:literal) => {{
        const BYTES: &[u8] = include_bytes!($path);
        const LEN: usize = $crate::qrc::tree_len($crate::qrc::PREFIX, $path, BYTES.len());
        static UNTIMED: [u8; LEN] = $crate::qrc::untimed($crate::qrc::PREFIX, $path, BYTES);
        $crate::qrc::register($path, &UNTIMED, BYTES)
    }};
}
pub(crate) use embed;

/// Registers the resource `qrc:/<PREFIX>/<path>`: `untimed` as it is, or a
/// timed tree of `bytes` for a start that keeps its compiled QML.
///
/// Qt reads the tree where it is handed and keeps no copy
/// (`QResource::registerResource`), so the tree lives as long as the
/// process does.
pub fn register(path: &str, untimed: &'static [u8], bytes: &[u8]) {
    let tree: &'static [u8] = if kept() {
        Box::leak(tree(PREFIX, path, bytes, true).into_boxed_slice())
    } else {
        untimed
    };
    if !qtbridge::qresource::register_bytes(tree) {
        tracing::error!(path, "Qt refused the resource");
    }
}

/// Whether this start was told where to keep its compiled QML.
fn kept() -> bool {
    static KEPT: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *KEPT.get_or_init(|| std::env::var_os("QML_DISK_CACHE_PATH").is_some_and(|dir| !dir.is_empty()))
}

/// The rcc tree of one file: header, the file's bytes, the names on the
/// way to it, then one node per name. `timed` is whether the file node
/// carries its time ([`kept`]); without one Qt keeps nothing.
fn tree(prefix: &str, path: &str, bytes: &[u8], timed: bool) -> Vec<u8> {
    let names: Vec<&str> = prefix
        .split('/')
        .chain(path.split('/'))
        .filter(|name| !name.is_empty())
        .collect();
    let mut out = Vec::with_capacity(bytes.len() + 64 + 32 * names.len());
    out.extend_from_slice(b"qres");
    push_u32(&mut out, FORMAT);
    // Where the tree, the data and the names start, and the flags —
    // written once they are known.
    out.extend_from_slice(&[0; 16]);

    let data_at = out.len();
    push_u32(&mut out, len_u32(bytes.len()));
    out.extend_from_slice(bytes);

    let names_at = out.len();
    let mut name_offsets = Vec::with_capacity(names.len());
    for name in &names {
        name_offsets.push(len_u32(out.len() - names_at));
        let units: Vec<u16> = name.encode_utf16().collect();
        push_u16(&mut out, u16::try_from(units.len()).unwrap_or(u16::MAX));
        push_u32(&mut out, qt_hash(&units));
        for unit in units {
            out.extend_from_slice(&unit.to_be_bytes());
        }
    }

    let tree_at = out.len();
    // Node 0 is the root and node k the k-th name: every directory holds
    // exactly the next node, and the last node is the file.
    push_node(&mut out, 0, DIRECTORY, [1, 1], 0);
    let (file, directories) = match name_offsets.split_last() {
        Some((file, directories)) => (*file, directories),
        None => (0, &[][..]),
    };
    for (at, offset) in directories.iter().enumerate() {
        push_node(&mut out, *offset, DIRECTORY, [1, len_u32(at + 2)], 0);
    }
    // A file node carries territory and language where a directory carries
    // its children: C, any territory — what rcc writes for an unqualified
    // file.
    let time = if timed { stamp(bytes) } else { 0 };
    push_node(&mut out, file, 0, [LANGUAGE_C, 0], time);

    for (at, value) in [(8, tree_at), (12, data_at), (16, names_at), (20, 0)] {
        out[at..at + 4].copy_from_slice(&len_u32(value).to_be_bytes());
    }
    out
}

/// The length of [`untimed`]'s tree for a file of `len` bytes.
pub const fn tree_len(prefix: &str, path: &str, len: usize) -> usize {
    let (outer, outer_units) = segments(prefix.as_bytes());
    let (inner, inner_units) = segments(path.as_bytes());
    let names = outer + inner;
    HEAD + 4 + len + 6 * names + 2 * (outer_units + inner_units) + NODE * (1 + names)
}

/// The tree [`tree`] builds with no time on it, built where it is called —
/// a static's initializer ([`embed`]). The same bytes, a test holds it to;
/// the file's bytes are copied once and whole, and a name outside ASCII
/// stops the build.
pub const fn untimed<const N: usize>(prefix: &str, path: &str, bytes: &[u8]) -> [u8; N] {
    let mut out = [0u8; N];
    let (magic, rest) = out.split_at_mut(4);
    magic.copy_from_slice(b"qres");
    put_u32(rest, 0, FORMAT);
    let data_at = HEAD;
    put_u32(&mut out, data_at, u32_of(bytes.len()));
    let (_, rest) = out.split_at_mut(data_at + 4);
    let (data, _) = rest.split_at_mut(bytes.len());
    data.copy_from_slice(bytes);
    let names_at = data_at + 4 + bytes.len();
    let (outer, outer_units) = segments(prefix.as_bytes());
    let (inner, inner_units) = segments(path.as_bytes());
    let names = outer + inner;
    let tree_at = names_at + 6 * names + 2 * (outer_units + inner_units);
    put_node(&mut out, tree_at, [0, DIRECTORY as u32, 1, 1]);
    let mut written = 0;
    let mut offset = 0;
    let mut which = 0;
    while which < 2 {
        let name = if which == 0 {
            prefix.as_bytes()
        } else {
            path.as_bytes()
        };
        let mut start = 0;
        while start < name.len() {
            let mut end = start;
            while end < name.len() && name[end] != b'/' {
                assert!(name[end] < 0x80, "a resource name outside ASCII");
                end += 1;
            }
            if end > start {
                let at = names_at + offset;
                let units = end - start;
                put_u16(&mut out, at, u16_of(units));
                put_u32(&mut out, at + 2, qt_hash_ascii(name, start, end));
                let mut unit = 0;
                while unit < units {
                    out[at + 6 + 2 * unit + 1] = name[start + unit];
                    unit += 1;
                }
                written += 1;
                // Every directory holds exactly the next node, and the last
                // node is the file: C, any territory, its data at the start.
                let words = if written == names {
                    [u32_of(offset), 0, LANGUAGE_C, 0]
                } else {
                    [u32_of(offset), DIRECTORY as u32, 1, u32_of(written + 1)]
                };
                put_node(&mut out, tree_at + NODE * written, words);
                offset += 6 + 2 * units;
            }
            start = end + 1;
        }
        which += 1;
    }
    put_u32(&mut out, 8, u32_of(tree_at));
    put_u32(&mut out, 12, u32_of(data_at));
    put_u32(&mut out, 16, u32_of(names_at));
    out
}

/// How many names `/` divides `name` into, empty ones aside, and their
/// length together.
const fn segments(name: &[u8]) -> (usize, usize) {
    let (mut count, mut units, mut at, mut run) = (0, 0, 0, 0);
    while at <= name.len() {
        if at == name.len() || name[at] == b'/' {
            if run > 0 {
                count += 1;
                units += run;
            }
            run = 0;
        } else {
            run += 1;
        }
        at += 1;
    }
    (count, units)
}

/// One node of an untimed tree: the offset of its name, its flags, its two
/// words; the time stays zero.
const fn put_node(out: &mut [u8], at: usize, [name, flags, first, second]: [u32; 4]) {
    put_u32(out, at, name);
    put_u16(out, at + 4, u16_of(flags as usize));
    put_u32(out, at + 6, first);
    put_u32(out, at + 10, second);
}

const fn put_u32(out: &mut [u8], at: usize, value: u32) {
    let bytes = value.to_be_bytes();
    let mut i = 0;
    while i < 4 {
        out[at + i] = bytes[i];
        i += 1;
    }
}

const fn put_u16(out: &mut [u8], at: usize, value: u16) {
    let bytes = value.to_be_bytes();
    out[at] = bytes[0];
    out[at + 1] = bytes[1];
}

const fn u32_of(value: usize) -> u32 {
    assert!(
        value <= u32::MAX as usize,
        "a resource past what four bytes hold"
    );
    value as u32
}

const fn u16_of(value: usize) -> u16 {
    assert!(
        value <= u16::MAX as usize,
        "a resource name past what two bytes hold"
    );
    value as u16
}

/// [`qt_hash`] of an ASCII name, `name[start..end]` — each byte is its own
/// UTF-16 unit.
const fn qt_hash_ascii(name: &[u8], start: usize, end: usize) -> u32 {
    let mut hash: u32 = 0;
    let mut at = start;
    while at < end {
        hash = (hash << 4).wrapping_add(name[at] as u32);
        hash ^= (hash & 0xf000_0000) >> 23;
        hash &= 0x0fff_ffff;
        at += 1;
    }
    hash
}

/// The magic, the format, the offsets of the tree, the data and the
/// names, and the flags word.
const HEAD: usize = 24;
/// A node: the name's offset, the flags, two words and the time.
const NODE: usize = 22;

/// The rcc format whose nodes carry a modification time (2 and later).
const FORMAT: u32 = 3;
const DIRECTORY: u16 = 2;
/// A file node's territory (high half) and language (low half) words as one
/// `u32`: `QLocale::C` is 1.
const LANGUAGE_C: u32 = 1;

/// One tree node: the offset of its name, its flags, two words that are
/// a directory's child count and first child or a file's locale and data
/// offset, and its time in milliseconds since the epoch.
fn push_node(out: &mut Vec<u8>, name: u32, flags: u16, words: [u32; 2], time: u64) {
    push_u32(out, name);
    push_u16(out, flags);
    push_u32(out, words[0]);
    push_u32(out, words[1]);
    out.extend_from_slice(&time.to_be_bytes());
}

/// A nonzero time that moves with `bytes` alone: FNV-1a, folded into the
/// hundred years from 2001 (Qt reads a zero as "no time").
fn stamp(bytes: &[u8]) -> u64 {
    const JAN_2001_MS: u64 = 978_307_200_000;
    const CENTURY_MS: u64 = 3_155_760_000_000;
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    JAN_2001_MS + hash % CENTURY_MS
}

/// Qt's hash of a resource name (`qt_hash` over UTF-16 units), which the
/// lookup compares before the name itself.
fn qt_hash(units: &[u16]) -> u32 {
    let mut hash: u32 = 0;
    for unit in units {
        hash = (hash << 4).wrapping_add(u32::from(*unit));
        hash ^= (hash & 0xf000_0000) >> 23;
        hash &= 0x0fff_ffff;
    }
    hash
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

/// A length or offset as the tree stores it. The largest file embedded is
/// a few hundred kilobytes, far inside what four bytes hold.
fn len_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The node `index` of a tree, as Qt reads one (`QResourceRoot`): 22
    /// bytes a node from the offset the header names.
    fn node(tree: &[u8], index: usize) -> &[u8] {
        let at = u32::from_be_bytes(tree[8..12].try_into().unwrap()) as usize + 22 * index;
        &tree[at..at + 22]
    }

    fn time(node: &[u8]) -> u64 {
        u64::from_be_bytes(node[14..22].try_into().unwrap())
    }

    #[test]
    fn the_file_carries_a_time_and_the_directories_none() {
        let built = tree("qt/qml/platitude", "ui/Main.qml", b"Item {}\n", true);
        // root, qt, qml, platitude, ui, then the file
        for index in 0..5 {
            assert_eq!(time(node(&built, index)), 0, "directory node {index}");
            assert_eq!(
                u16::from_be_bytes(node(&built, index)[4..6].try_into().unwrap()),
                DIRECTORY
            );
        }
        let file = node(&built, 5);
        assert_eq!(
            u16::from_be_bytes(file[4..6].try_into().unwrap()),
            0,
            "a plain file"
        );
        assert_eq!(time(file), stamp(b"Item {}\n"));
        assert_ne!(time(file), 0, "Qt reads a zero as no time at all");
    }

    /// A start nobody told where to keep its compiled QML writes the same
    /// tree with no time on it, so Qt keeps nothing — and the tree is the
    /// same byte for byte otherwise.
    #[test]
    fn a_start_not_told_where_to_keep_them_carries_no_time() {
        let untimed = tree("qt/qml/platitude", "ui/Main.qml", b"Item {}\n", false);
        assert_eq!(time(node(&untimed, 5)), 0);
        let mut timed = tree("qt/qml/platitude", "ui/Main.qml", b"Item {}\n", true);
        let at = u32::from_be_bytes(timed[8..12].try_into().unwrap()) as usize + 22 * 5 + 14;
        timed[at..at + 8].copy_from_slice(&[0; 8]);
        assert_eq!(untimed, timed, "the time is the only difference");
    }

    #[test]
    fn the_names_and_the_bytes_are_where_the_nodes_say() {
        let bytes = b"import QtQuick\nItem {}\n";
        let built = tree("qt/qml/platitude", "auto/qmldir", bytes, true);
        let names_at = u32::from_be_bytes(built[16..20].try_into().unwrap()) as usize;
        let mut walked = Vec::new();
        for index in 1..=5 {
            let entry = node(&built, index);
            let name_at = names_at + u32::from_be_bytes(entry[0..4].try_into().unwrap()) as usize;
            let units =
                u16::from_be_bytes(built[name_at..name_at + 2].try_into().unwrap()) as usize;
            let hash = u32::from_be_bytes(built[name_at + 2..name_at + 6].try_into().unwrap());
            let name: Vec<u16> = built[name_at + 6..name_at + 6 + 2 * units]
                .chunks(2)
                .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
                .collect();
            assert_eq!(hash, qt_hash(&name));
            walked.push(String::from_utf16(&name).unwrap());
            if index < 5 {
                // Each directory holds one child: the next node.
                assert_eq!(u32::from_be_bytes(entry[6..10].try_into().unwrap()), 1);
                assert_eq!(
                    u32::from_be_bytes(entry[10..14].try_into().unwrap()) as usize,
                    index + 1
                );
            }
        }
        assert_eq!(walked, ["qt", "qml", "platitude", "auto", "qmldir"]);
        let data_at = u32::from_be_bytes(built[12..16].try_into().unwrap()) as usize;
        let length = u32::from_be_bytes(built[data_at..data_at + 4].try_into().unwrap()) as usize;
        assert_eq!(&built[data_at + 4..data_at + 4 + length], bytes);
    }

    #[test]
    fn the_time_moves_with_the_content_alone() {
        assert_eq!(stamp(b"Item {}"), stamp(b"Item {}"));
        assert_ne!(stamp(b"Item {}"), stamp(b"Item { }"));
        assert_eq!(
            time(node(&tree("a", "b.qml", b"x", true), 2)),
            time(node(&tree("qt/qml/platitude", "ui/b.qml", b"x", true), 5)),
            "where a file stands is not part of its time"
        );
    }

    /// The tree a static holds is the one a start would have built with no
    /// time on it, byte for byte — a nested path, a prefix with empty
    /// names in it, and a file of a few hundred kilobytes.
    #[test]
    fn the_tree_built_at_compile_time_is_the_untimed_one() {
        const SMALL: &[u8] = b"import QtQuick\nItem {}\n";
        const SMALL_LEN: usize = tree_len(PREFIX, "ui/Main.qml", SMALL.len());
        static SMALL_TREE: [u8; SMALL_LEN] = untimed(PREFIX, "ui/Main.qml", SMALL);
        assert_eq!(
            &SMALL_TREE[..],
            &tree(PREFIX, "ui/Main.qml", SMALL, false)[..]
        );

        const EDGES_LEN: usize = tree_len("/qt//qml/", "auto/qmldir", SMALL.len());
        static EDGES_TREE: [u8; EDGES_LEN] = untimed("/qt//qml/", "auto/qmldir", SMALL);
        assert_eq!(
            &EDGES_TREE[..],
            &tree("/qt//qml/", "auto/qmldir", SMALL, false)[..]
        );

        const LARGE: &[u8] = &[b'x'; 300_000];
        const LARGE_LEN: usize = tree_len(PREFIX, "ui/RepoPage.qml", LARGE.len());
        static LARGE_TREE: [u8; LARGE_LEN] = untimed(PREFIX, "ui/RepoPage.qml", LARGE);
        assert_eq!(
            &LARGE_TREE[..],
            &tree(PREFIX, "ui/RepoPage.qml", LARGE, false)[..]
        );
    }

    #[test]
    fn names_hash_as_qt_hashes_them() {
        // Worked out apart from this code, from `qt_hash` (qhash.cpp):
        // (0x71 << 4) + 0x74 is all inside 28 bits; the two longer ones fold
        // the high nibble back. A wrong hash is a resource Qt cannot find —
        // every verb's app loading nothing — so the runs say it too.
        let hash = |name: &str| qt_hash(&name.encode_utf16().collect::<Vec<_>>());
        assert_eq!(hash("qt"), 0x784);
        assert_eq!(hash("platitude"), 0x08b0_55e5);
        assert_eq!(hash("WindowSettingsActs.qml"), 0x0222_ea7c);
        // And the compile-time reading of the same names.
        for name in ["qt", "platitude", "WindowSettingsActs.qml"] {
            assert_eq!(
                qt_hash_ascii(name.as_bytes(), 0, name.len()),
                hash(name),
                "{name}"
            );
        }
    }
}
