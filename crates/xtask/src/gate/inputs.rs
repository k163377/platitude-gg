//! Step input identities exclude Markdown, including inside directory inputs.

use std::collections::BTreeMap;

pub(super) fn from_listing(listing: &[u8]) -> BTreeMap<String, String> {
    let mut files = BTreeMap::new();
    let mut directories: BTreeMap<String, String> = BTreeMap::new();
    for entry in String::from_utf8_lossy(listing).split('\0') {
        let Some((head, path)) = entry.split_once('\t') else {
            continue;
        };
        let mut fields = head.split(' ');
        let (Some(mode), Some(kind), Some(id)) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        if kind == "tree" || super::graph::is_markdown(path) {
            continue;
        }
        files.insert(path.to_string(), id.to_string());
        let mut parent = path;
        while let Some((dir, _)) = parent.rsplit_once('/') {
            // Retain modes, names and gitlinks as a Git tree would.
            let contents = directories.entry(dir.to_string()).or_default();
            contents.push_str(&format!("{mode} {kind} {id}\t{path}\0"));
            parent = dir;
        }
    }
    for (dir, contents) in directories {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in contents.bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x1000_0000_01b3);
        }
        files.insert(dir, format!("files:{hash:016x}"));
    }
    files
}

#[cfg(test)]
mod tests {
    use super::from_listing;

    #[test]
    fn directory_inputs_keep_names_modes_and_gitlinks_but_not_markdown() {
        let base = "100644 blob a\tsrc/code.rs\0";
        let ids = from_listing(base.as_bytes());
        let docs = format!("040000 tree changed\tsrc\0{base}100644 blob b\tsrc/README.MD\0");
        assert_eq!(ids, from_listing(docs.as_bytes()));
        for moved in [
            "100755 blob a\tsrc/code.rs\0",
            "100644 blob b\tsrc/code.rs\0",
            "100644 blob a\tsrc/renamed.rs\0",
            "160000 commit a\tsrc/code.rs\0",
        ] {
            assert_ne!(ids["src"], from_listing(moved.as_bytes())["src"]);
        }
        assert!(from_listing(b"100644 blob a\tdocs/only.md\0").is_empty());
    }
}
