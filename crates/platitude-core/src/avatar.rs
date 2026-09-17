//! Pictures a person has put against the authors they read.
//!
//! Nothing here talks to anybody. A picture is one the person named on
//! their own disk, copied in beside the settings; there is no service to
//! ask and none is going to be added (CLAUDE.md 絶対制約: the only network
//! this application has is git's). What everyone else gets from a gravatar
//! service, this gets from a file dialog.
//!
//! The key is the **address**, lowercased and read through `.mailmap`
//! (`parse::log`): names are what people change, what two people share,
//! and what a repository spells three ways. The store keeps the name
//! too, but only so a list of assignments can be read by a human — the
//! address is what matching runs on.
//!
//! The index lives in `settings.toml` and the images in `avatars/` beside
//! it, because both are what a person decided (`settings` module). An
//! image is named for a hash of its own
//! bytes, which buys three things: replacing a picture changes the file
//! name, so an image cache keyed by path cannot hand back the old one; the
//! same picture assigned to two people is stored once; and no part of an
//! address ever reaches a file name, so nothing has to be escaped, kept
//! under a path limit, or checked against the names Windows reserves.

use std::path::{Path, PathBuf};

/// Directory the images sit in, beside `settings.toml`.
pub const DIR_NAME: &str = "avatars";

/// Extensions the picker offers. **What a file is gets decided by what
/// is in it** (`picture`) — a picture saved under the wrong name still
/// works. This is only so that somebody browsing for their own picture
/// sees only pictures.
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

/// A refusal as the screen is written from it (app-ui.md「Rust に文言を
/// 置かない」): which one it is, the numbers its sentence takes, and
/// whoever outside wrote a line of their own.
///
/// **The sentences above are the log's.** They reach a screen through
/// this instead, where the words are `qsTr`'d like every other word in
/// the application — six of the seven are this end's own writing and had
/// no business being in Rust at all. The two that wrap an `io::Error` are
/// the exception in the other direction: the operating system's words
/// are carried across untouched, under a frame the UI writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvatarRefusal {
    pub kind: &'static str,
    /// The numbers the sentence takes, in the order it takes them.
    pub facts: Vec<String>,
    /// What the operating system said, where the failure is one it made.
    /// Empty for the rest, which is how the screen tells the two apart.
    pub said: String,
}

impl AvatarError {
    /// This refusal as the screen reads it.
    #[must_use]
    pub fn refusal(&self) -> AvatarRefusal {
        let plain = |kind: &'static str| AvatarRefusal {
            kind,
            facts: Vec::new(),
            said: String::new(),
        };
        match self {
            Self::TooLarge => AvatarRefusal {
                kind: "too-large",
                facts: vec![(MAX_BYTES / (1024 * 1024)).to_string()],
                said: String::new(),
            },
            Self::Picture(crate::picture::PictureError::Unreadable) => plain("unreadable"),
            Self::Picture(crate::picture::PictureError::TooManyPixels { width, height }) => {
                AvatarRefusal {
                    kind: "too-many-pixels",
                    facts: vec![
                        format!("{width}x{height}"),
                        (crate::picture::MAX_PIXELS / 1_000_000).to_string(),
                    ],
                    said: String::new(),
                }
            }
            Self::Picture(crate::picture::PictureError::Unstorable) => plain("unstorable"),
            Self::Read { source, .. } => AvatarRefusal {
                kind: "read",
                facts: Vec::new(),
                said: source.to_string(),
            },
            Self::Write { source, .. } => AvatarRefusal {
                kind: "write",
                facts: Vec::new(),
                said: source.to_string(),
            },
            Self::NoStore => plain("no-store"),
        }
    }
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
    /// Reads assignments off a parsed `[[avatar]]` array. Entries
    /// missing either half of the mapping are dropped: an assignment
    /// with no address matches nobody, and one with no file draws
    /// nothing.
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
    /// nothing with no way to say why. Rewriting is what keeps a
    /// photograph from costing tens of megabytes of memory for as
    /// long as the application is up.
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

    /// Deletes an image no assignment points at any more. Failure here
    /// is silent: the assignment is already gone, which is what was
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
#[path = "avatar_tests.rs"]
mod tests;
