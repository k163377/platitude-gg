//! Pictures a person has put against the authors they read.
//!
//! Nothing here talks to anybody. A picture is one the person named on
//! their own disk, copied in beside the settings; there is no service to
//! ask and none is going to be added (CLAUDE.md 絶対制約: the only network
//! this application has is git's). What everyone else gets from a gravatar
//! service, this gets from a file dialog.
//!
//! The key is the **address**, lowercased and read through `.mailmap`
//! (`parse::log`), never the name: names are what people change, what two
//! people share, and what a repository spells three ways. The store keeps
//! the name too, but only so a list of assignments can be read by a human —
//! it is never matched on.
//!
//! The index lives in `settings.toml` and the images in `avatars/` beside
//! it, because both are what a person decided rather than what a session
//! left behind (`settings` module). An image is named for a hash of its own
//! bytes, which buys three things: replacing a picture changes the file
//! name, so an image cache keyed by path cannot hand back the old one; the
//! same picture assigned to two people is stored once; and no part of an
//! address ever reaches a file name, so nothing has to be escaped, kept
//! under a path limit, or checked against the names Windows reserves.

use std::path::{Path, PathBuf};

/// Directory the images sit in, beside `settings.toml`.
pub const DIR_NAME: &str = "avatars";

/// Extensions the picker offers. **What a file is gets decided by what
/// is in it** (`picture`), not by this — a picture saved under the wrong
/// name still works. This is only so that somebody browsing for their
/// own picture is not shown every file they own.
pub const EXTENSIONS: [&str; 3] = ["png", "jpg", "jpeg"];

/// Largest file read. Nothing this size is ever *kept* — everything is
/// rewritten small on the way in — so this bounds the reading and
/// decoding done while a person waits, and nothing else. In particular
/// it bounds no part of the picture itself: a flat PNG decodes to about
/// a thousand times its own bytes, which is what `picture::MAX_PIXELS`
/// is for.
pub const MAX_BYTES: u64 = 10 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum AvatarError {
    #[error("the file is larger than {}MB", MAX_BYTES / (1024 * 1024))]
    TooLarge,
    #[error(transparent)]
    Picture(#[from] crate::picture::PictureError),
    #[error("could not read {}: {source}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("could not write {}: {source}", path.display())]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// No configuration directory, so nothing can be kept. A screenshot run
    /// and a machine with no `APPDATA` both land here.
    #[error("there is nowhere to keep avatars")]
    NoStore,
}

/// The key a picture is filed under.
///
/// One function so the log parser, the details pane and the store cannot
/// disagree about what counts as the same person. git hands addresses back
/// exactly as each commit spelled them — mailmap matches without regard to
/// case but does not rewrite what it did not map (measured) — so a person
/// who shouted their address into one commit would otherwise file under a
/// second key.
pub fn key(email: &str) -> String {
    email.trim().to_lowercase()
}

/// One assignment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignment {
    /// The address, already through [`key`].
    pub email: String,
    /// What to call this person in a list. Display only.
    pub name: String,
    /// File name within the avatars directory.
    pub file: String,
}

/// Every assignment, in the order a list should show them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Avatars {
    entries: Vec<Assignment>,
}

impl Avatars {
    /// Reads assignments off a parsed `[[avatar]]` array. Entries missing
    /// either half of the mapping are dropped rather than kept as a hole:
    /// an assignment with no address matches nobody, and one with no file
    /// draws nothing.
    pub fn from_values(values: &[toml::Value]) -> Self {
        let mut entries: Vec<Assignment> = Vec::new();
        for value in values {
            let Some(table) = value.as_table() else {
                continue;
            };
            let email = table
                .get("email")
                .and_then(toml::Value::as_str)
                .map(key)
                .unwrap_or_default();
            let file = table
                .get("file")
                .and_then(toml::Value::as_str)
                .unwrap_or_default()
                .to_string();
            if email.is_empty() || !is_safe_file_name(&file) {
                continue;
            }
            let name = table
                .get("name")
                .and_then(toml::Value::as_str)
                .unwrap_or_default()
                .to_string();
            // Last writer wins for a repeated address, the same way a
            // repeated key in a table would.
            entries.retain(|e| e.email != email);
            entries.push(Assignment { email, name, file });
        }
        entries.sort_by(|a, b| a.email.cmp(&b.email));
        Self { entries }
    }

    pub fn to_values(&self) -> Vec<toml::Value> {
        self.entries
            .iter()
            .map(|entry| {
                let mut table = toml::Table::new();
                table.insert("email".into(), toml::Value::String(entry.email.clone()));
                if !entry.name.is_empty() {
                    table.insert("name".into(), toml::Value::String(entry.name.clone()));
                }
                table.insert("file".into(), toml::Value::String(entry.file.clone()));
                toml::Value::Table(table)
            })
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn list(&self) -> &[Assignment] {
        &self.entries
    }

    /// The file name assigned to an address, if any.
    pub fn file_of(&self, email: &str) -> Option<&str> {
        let key = key(email);
        self.entries
            .iter()
            .find(|e| e.email == key)
            .map(|e| e.file.as_str())
    }

    /// Drops assignments whose image is no longer on disk. A person who
    /// emptied the directory by hand has said what they meant; carrying the
    /// entry forward would leave a row in the settings list that draws
    /// nothing and cannot be told apart from one that is merely slow.
    pub fn forget_missing(&mut self, dir: &Path) -> usize {
        let before = self.entries.len();
        self.entries.retain(|e| dir.join(&e.file).is_file());
        before - self.entries.len()
    }

    /// Files a picture under an address, rewriting it small on the way in
    /// (`picture`). Returns the name of the file now holding it.
    ///
    /// Keeping a copy of our own is what makes this durable: the picture a
    /// person picked out of their downloads folder will be moved or
    /// deleted, and a store that only remembered the path would then draw
    /// nothing with no way to say why. Rewriting rather than copying is
    /// what keeps a photograph from costing tens of megabytes of memory
    /// for as long as the application is up.
    pub fn assign(
        &mut self,
        dir: &Path,
        email: &str,
        name: &str,
        source: &Path,
    ) -> Result<String, AvatarError> {
        let size = std::fs::metadata(source)
            .map_err(|source_err| AvatarError::Read {
                path: source.to_path_buf(),
                source: source_err,
            })?
            .len();
        if size > MAX_BYTES {
            return Err(AvatarError::TooLarge);
        }
        let bytes = std::fs::read(source).map_err(|source_err| AvatarError::Read {
            path: source.to_path_buf(),
            source: source_err,
        })?;
        // What gets stored is the rewritten picture, so the name is a hash
        // of that and not of what was picked: two people who found the
        // same photograph in different formats still share one file.
        let bytes = crate::picture::normalize(&bytes)?;
        let file = format!("{}.png", content_name(&bytes));
        let target = dir.join(&file);
        std::fs::create_dir_all(dir).map_err(|source_err| AvatarError::Write {
            path: dir.to_path_buf(),
            source: source_err,
        })?;
        // Written even when a file of that name is already there: same
        // name means same bytes, and a half-written one from a previous
        // run is worth replacing.
        std::fs::write(&target, &bytes).map_err(|source_err| AvatarError::Write {
            path: target.clone(),
            source: source_err,
        })?;

        let email = key(email);
        let previous = self.file_of(&email).map(str::to_string);
        self.entries.retain(|e| e.email != email);
        self.entries.push(Assignment {
            email,
            name: name.trim().to_string(),
            file: file.clone(),
        });
        self.entries.sort_by(|a, b| a.email.cmp(&b.email));
        if let Some(old) = previous {
            self.sweep(dir, &old);
        }
        Ok(file)
    }

    /// Takes the picture off an address. The person's own file is not
    /// touched — only the copy this kept.
    pub fn remove(&mut self, dir: &Path, email: &str) -> bool {
        let email = key(email);
        let Some(position) = self.entries.iter().position(|e| e.email == email) else {
            return false;
        };
        let gone = self.entries.remove(position);
        self.sweep(dir, &gone.file);
        true
    }

    /// Deletes an image no assignment points at any more. Failing to is not
    /// worth reporting: the assignment is already gone, which is what was
    /// asked for, and a file left behind costs a few kilobytes until the
    /// same picture is assigned again.
    fn sweep(&self, dir: &Path, file: &str) {
        if self.entries.iter().any(|e| e.file == file) {
            return;
        }
        if let Err(error) = std::fs::remove_file(dir.join(file)) {
            tracing::debug!(file, %error, "unreferenced avatar image not removed");
        }
    }
}

/// A file name built only from the bytes it will hold: 16 hex digits of
/// FNV-1a over the content, plus the length, which is what keeps two
/// different pictures from colliding on a 64-bit hash alone.
fn content_name(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}{:08x}", bytes.len() as u32)
}

/// Whether a name out of the settings file may be joined onto the avatars
/// directory. Everything this writes is hex and a known extension, so
/// anything else came from an edited file — and `..` in a stored name is
/// how a settings file turns into a way to read an arbitrary path.
fn is_safe_file_name(file: &str) -> bool {
    !file.is_empty()
        && Path::new(file).file_name().is_some_and(|n| n == file)
        && file
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
        && !file.starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    /// A real picture on disk. The store decodes what it is handed now,
    /// so a few bytes standing in for one no longer reaches the far side.
    fn picture(dir: &Path, name: &str, tint: u8) -> PathBuf {
        let png = crate::picture::png_of(24, 16, |x, _| [tint, x as u8, 40, 255]);
        file_of(dir, name, &png)
    }

    fn file_of(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, bytes).expect("write file");
        path
    }

    #[test]
    fn the_key_is_the_address_folded_to_one_spelling() {
        assert_eq!(key("  Ada@Example.COM "), "ada@example.com");
        assert_eq!(key(""), "");
    }

    #[test]
    fn assigning_copies_the_picture_in() {
        let scratch = scratch();
        let source = picture(scratch.path(), "cat.png", 200);
        let store_dir = scratch.path().join("avatars");
        let mut avatars = Avatars::default();

        let file = avatars
            .assign(&store_dir, "Ada@Example.com", "Ada", &source)
            .expect("assign");
        assert!(store_dir.join(&file).is_file(), "the copy is on disk");
        assert_eq!(avatars.file_of("ada@example.com"), Some(file.as_str()));
        // The address folded, the name kept as it was given.
        assert_eq!(avatars.list()[0].email, "ada@example.com");
        assert_eq!(avatars.list()[0].name, "Ada");

        // The picture a person picked can go away without taking the
        // avatar with it.
        std::fs::remove_file(&source).expect("remove source");
        assert!(store_dir.join(&file).is_file());
    }

    #[test]
    fn replacing_a_picture_changes_the_file_name_and_sweeps_the_old_one() {
        let scratch = scratch();
        let store_dir = scratch.path().join("avatars");
        let mut avatars = Avatars::default();
        let first = picture(scratch.path(), "a.png", 10);
        let second = picture(scratch.path(), "b.png", 220);

        let old = avatars
            .assign(&store_dir, "ada@example.com", "Ada", &first)
            .expect("assign");
        let new = avatars
            .assign(&store_dir, "ada@example.com", "Ada", &second)
            .expect("reassign");

        assert_ne!(old, new, "a different picture is a different name");
        assert!(!store_dir.join(&old).exists(), "the old copy is swept");
        assert!(store_dir.join(&new).is_file());
        assert_eq!(avatars.list().len(), 1, "one address, one row");
    }

    #[test]
    fn the_same_picture_for_two_people_is_stored_once() {
        let scratch = scratch();
        let store_dir = scratch.path().join("avatars");
        let mut avatars = Avatars::default();
        let source = picture(scratch.path(), "shared.png", 90);

        let a = avatars
            .assign(&store_dir, "ada@example.com", "Ada", &source)
            .expect("assign");
        let b = avatars
            .assign(&store_dir, "bob@example.com", "Bob", &source)
            .expect("assign");
        assert_eq!(a, b);

        // And taking it off one leaves the other drawable.
        assert!(avatars.remove(&store_dir, "ada@example.com"));
        assert!(store_dir.join(&b).is_file(), "the file is still spoken for");
        assert!(avatars.remove(&store_dir, "bob@example.com"));
        assert!(!store_dir.join(&b).exists(), "now nothing points at it");
    }

    #[test]
    fn something_that_is_not_a_picture_is_refused_before_anything_is_written() {
        let scratch = scratch();
        let store_dir = scratch.path().join("avatars");
        let mut avatars = Avatars::default();
        // Named like a picture, which is exactly why the name is not
        // what decides.
        let source = file_of(scratch.path(), "cat.png", b"hello");
        assert!(matches!(
            avatars.assign(&store_dir, "ada@example.com", "Ada", &source),
            Err(AvatarError::Picture(
                crate::picture::PictureError::Unreadable
            ))
        ));
        assert!(!store_dir.exists(), "nothing was created");
        assert!(avatars.is_empty());
    }

    #[test]
    fn a_file_past_the_ceiling_is_refused_without_being_read() {
        let scratch = scratch();
        let store_dir = scratch.path().join("avatars");
        let mut avatars = Avatars::default();
        let big = vec![0u8; (MAX_BYTES + 1) as usize];
        let source = file_of(scratch.path(), "huge.png", &big);
        assert!(matches!(
            avatars.assign(&store_dir, "ada@example.com", "Ada", &source),
            Err(AvatarError::TooLarge)
        ));
    }

    #[test]
    fn what_is_stored_is_the_small_square_rather_than_what_was_picked() {
        let scratch = scratch();
        let store_dir = scratch.path().join("avatars");
        let mut avatars = Avatars::default();
        // Wide, and far bigger than anything ever drawn.
        let wide = crate::picture::png_of(1200, 800, |x, y| {
            [(x % 251) as u8, (y % 253) as u8, 30, 255]
        });
        let source = file_of(scratch.path(), "wide.png", &wide);

        let file = avatars
            .assign(&store_dir, "ada@example.com", "Ada", &source)
            .expect("assign");
        assert!(file.ends_with(".png"), "always stored as png: {file}");
        let stored = std::fs::read(store_dir.join(&file)).expect("read stored");
        assert!(
            stored.len() < wide.len(),
            "smaller than what was picked: {} vs {}",
            stored.len(),
            wide.len()
        );
        // The header says it plainly: IHDR carries the two sides.
        let side = u32::from_be_bytes([stored[16], stored[17], stored[18], stored[19]]);
        let other = u32::from_be_bytes([stored[20], stored[21], stored[22], stored[23]]);
        assert_eq!((side, other), (crate::picture::SIDE, crate::picture::SIDE));
    }

    #[test]
    fn a_file_name_out_of_the_settings_file_cannot_leave_the_directory() {
        let values = vec![
            toml::Value::Table({
                let mut t = toml::Table::new();
                t.insert("email".into(), toml::Value::String("a@b.c".into()));
                t.insert(
                    "file".into(),
                    toml::Value::String("../../../etc/passwd".into()),
                );
                t
            }),
            toml::Value::Table({
                let mut t = toml::Table::new();
                t.insert("email".into(), toml::Value::String("d@e.f".into()));
                t.insert("file".into(), toml::Value::String("ab12cd34.png".into()));
                t
            }),
        ];
        let avatars = Avatars::from_values(&values);
        assert_eq!(avatars.list().len(), 1);
        assert_eq!(avatars.list()[0].email, "d@e.f");
    }

    #[test]
    fn an_assignment_whose_image_is_gone_is_forgotten() {
        let scratch = scratch();
        let store_dir = scratch.path().join("avatars");
        let mut avatars = Avatars::default();
        let source = picture(scratch.path(), "a.png", 70);
        let file = avatars
            .assign(&store_dir, "ada@example.com", "Ada", &source)
            .expect("assign");

        assert_eq!(avatars.forget_missing(&store_dir), 0);
        std::fs::remove_file(store_dir.join(&file)).expect("remove copy");
        assert_eq!(avatars.forget_missing(&store_dir), 1);
        assert!(avatars.is_empty());
    }

    #[test]
    fn a_round_trip_through_the_table_keeps_every_assignment() {
        let scratch = scratch();
        let store_dir = scratch.path().join("avatars");
        let mut avatars = Avatars::default();
        let a = picture(scratch.path(), "a.png", 30);
        let b = picture(scratch.path(), "b.jpg", 180);
        avatars
            .assign(&store_dir, "zoe@example.com", "Zoe", &a)
            .expect("assign");
        avatars
            .assign(&store_dir, "ada@example.com", "Ada", &b)
            .expect("assign");

        let back = Avatars::from_values(&avatars.to_values());
        assert_eq!(back, avatars);
        // Sorted by address, so the file a person opens reads in a fixed
        // order rather than in the order they happened to assign.
        assert_eq!(back.list()[0].email, "ada@example.com");
    }
}
