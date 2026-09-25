//! Pictures a person has put against the authors they read.
//!
//! A picture is a local file the person picked, copied in beside the
//! settings; there is no avatar service and none is to be added (CLAUDE.md
//! 絶対制約: the only network is git's).
//!
//! The key is the address, lowercased and read through `.mailmap`
//! (`parse::log`) — names change, are shared, and are spelled several ways.
//!
//! The index lives in `settings.toml` and the images in `avatars/` beside
//! it — both are what a person decided (`settings` module). An image is
//! named for a hash of its own bytes: replacing a picture changes the file
//! name (so a path-keyed image cache cannot hand back the old one), a shared
//! picture is stored once, and no part of an address reaches a file name.

use std::path::{Path, PathBuf};

/// Directory the images sit in, beside `settings.toml`.
pub const DIR_NAME: &str = "avatars";

/// Extensions the file dialog filters on — the dialog's only; the format
/// is decided by the content (`picture`).
pub const EXTENSIONS: [&str; 3] = ["png", "jpg", "jpeg"];

/// Largest file read. It bounds the reading while a person waits, not the
/// decoded size (`picture::MAX_PIXELS` does).
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
    /// No configuration directory (a screenshot run, or no `APPDATA`).
    #[error("there is nowhere to keep avatars")]
    NoStore,
}

/// A refusal as the screen is written from it; the `#[error]` sentences
/// above are the log's only (rules-refs/app-ui.md「アバターの拒否は種別 + 数で渡す」).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvatarRefusal {
    pub kind: &'static str,
    /// The numbers the sentence takes, in the order it takes them.
    pub facts: Vec<String>,
    /// What the operating system said (`read` / `write`). Empty otherwise,
    /// which is how the screen tells the two apart.
    pub said: String,
}

impl AvatarError {
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

/// The key a picture is filed under — the one fold the log parser, the
/// details pane and the store share. git returns addresses as each commit
/// spelled them (mailmap matches case-insensitively but does not rewrite
/// what it did not map), so without it one person files under two keys.
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
    /// Reads assignments off a parsed `[[avatar]]` array. Entries with no
    /// address or no safe file name are dropped.
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
            // Last writer wins for a repeated address.
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

    /// Drops assignments whose image is no longer on disk; kept, they would
    /// be settings rows that draw nothing and look merely slow.
    pub fn forget_missing(&mut self, dir: &Path) -> usize {
        let before = self.entries.len();
        self.entries.retain(|e| dir.join(&e.file).is_file());
        before - self.entries.len()
    }

    /// Files a picture under an address, copying it in rewritten small
    /// (`picture`). Returns the name of the file now holding it.
    ///
    /// A copy, not the path: the picked file will be moved or deleted.
    /// Rewriting keeps a full-size photograph out of memory for as long as
    /// the application is up.
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
        // Named for the rewritten bytes, so one photograph picked in two
        // formats still shares one file.
        let bytes = crate::picture::normalize(&bytes)?;
        let file = format!("{}.png", content_name(&bytes));
        let target = dir.join(&file);
        std::fs::create_dir_all(dir).map_err(|source_err| AvatarError::Write {
            path: dir.to_path_buf(),
            source: source_err,
        })?;
        // Written even when the name exists: a half-written one from a
        // previous run gets replaced.
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

    /// Deletes an image no assignment points at any more. Failure is only
    /// logged: the assignment is already gone, which is what was asked for.
    fn sweep(&self, dir: &Path, file: &str) {
        if self.entries.iter().any(|e| e.file == file) {
            return;
        }
        if let Err(error) = std::fs::remove_file(dir.join(file)) {
            tracing::debug!(file, %error, "unreferenced avatar image not removed");
        }
    }
}

/// FNV-1a of the content plus its length; the length keeps two pictures
/// from colliding on the 64-bit hash alone.
fn content_name(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}{:08x}", bytes.len() as u32)
}

/// Whether a name out of the settings file may be joined onto the avatars
/// directory — a hand-edited `..` would otherwise read an arbitrary path.
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
