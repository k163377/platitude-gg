//! The worktrees whose `git worktree remove` is under way, process-wide
//! (デザイン規約 §消す操作は先に画面から消す): every listing leaves them out,
//! so a session opened while git is still deleting the folder — a tab
//! selected again, or one stood in another of the repository's worktrees —
//! does not draw it back.
//!
//! The process's, not a session's: the session that asked can be closed
//! with its write let run on (`RepoSession::close`), and the next one may
//! stand in another worktree of the repository, which has a write order of
//! its own (`session::write_order`) — so the write's landing tells it
//! nothing. Each removal remembers who left it out, and has them read
//! again once git has answered: a refused worktree is listed again there
//! at once, not at the next paced read.
//!
//! Keyed by where the worktree is ([`same_path_key`] of git's spelling),
//! which no two repositories share.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};

use super::{relock, same_path_key};
use crate::worktrees::WorktreeEntry;

/// One that lists worktrees and can be asked to list them again — a
/// session; a trait so the registry is tested without one.
pub(super) trait Reader: Send + Sync {
    fn read_again(self: Arc<Self>);
}

impl Reader for super::RepoSession {
    fn read_again(self: Arc<Self>) {
        super::RepoSession::read_again(&self);
    }
}

struct UnderWay {
    /// Which removal this is: two of one worktree end apart.
    id: u64,
    key: String,
    /// The session that asked, which reads behind its own write.
    writer: Weak<dyn Reader>,
    /// The ones that left it out of a listing.
    readers: Vec<Weak<dyn Reader>>,
}

/// One entry per removal under way: a path twice is two writes asked for it.
static UNDER_WAY: std::sync::Mutex<Vec<UnderWay>> = std::sync::Mutex::new(Vec::new());
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

/// One removal under way, from the acceptance of its write until git has
/// answered for it. Given back by dropping, however the write ends — one
/// the queue turned away included.
pub(super) struct Removal {
    id: u64,
}

impl Removal {
    pub(super) fn begin(path: &str, writer: &Arc<impl Reader + 'static>) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let writer: Weak<dyn Reader> = Arc::downgrade(writer) as Weak<dyn Reader>;
        relock(&UNDER_WAY).push(UnderWay {
            id,
            key: same_path_key(path),
            writer,
            readers: Vec::new(),
        });
        Self { id }
    }
}

impl Drop for Removal {
    fn drop(&mut self) {
        let told = {
            let mut under_way = relock(&UNDER_WAY);
            let Some(at) = under_way.iter().position(|removal| removal.id == self.id) else {
                return;
            };
            let ended = under_way.swap_remove(at);
            // Another removal of the same worktree still out: it is still
            // left out, and its own end tells them.
            if let Some(other) = under_way.iter_mut().find(|other| other.key == ended.key) {
                for reader in ended.readers {
                    note_reader(&mut other.readers, reader);
                }
                return;
            }
            ended
                .readers
                .iter()
                .filter(|reader| !std::ptr::addr_eq(reader.as_ptr(), ended.writer.as_ptr()))
                .filter_map(Weak::upgrade)
                .collect::<Vec<_>>()
        };
        // Outside the lock: a read asked for lists again, through `left_out`.
        for reader in told {
            reader.read_again();
        }
    }
}

/// The listing less the worktrees being removed, `reader` written down as
/// one to tell when each of those removals is over.
pub(super) fn left_out(
    mut worktrees: Vec<WorktreeEntry>,
    reader: &Arc<impl Reader + 'static>,
) -> Vec<WorktreeEntry> {
    let mut under_way = relock(&UNDER_WAY);
    if under_way.is_empty() {
        return worktrees;
    }
    let reader: Weak<dyn Reader> = Arc::downgrade(reader) as Weak<dyn Reader>;
    worktrees.retain(|worktree| {
        let key = same_path_key(&worktree.path);
        let mut out = false;
        for removal in under_way.iter_mut().filter(|removal| removal.key == key) {
            out = true;
            note_reader(&mut removal.readers, Weak::clone(&reader));
        }
        !out
    });
    worktrees
}

/// Writes `reader` down once: it is told once however often it left the
/// worktree out.
fn note_reader(readers: &mut Vec<Weak<dyn Reader>>, reader: Weak<dyn Reader>) {
    if !readers
        .iter()
        .any(|known| std::ptr::addr_eq(known.as_ptr(), reader.as_ptr()))
    {
        readers.push(reader);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Counts the reads it is asked for.
    #[derive(Default)]
    struct Counting(AtomicU64);

    impl Reader for Counting {
        fn read_again(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl Counting {
        fn asked(&self) -> u64 {
            self.0.load(Ordering::SeqCst)
        }
    }

    fn entry(path: &str) -> WorktreeEntry {
        WorktreeEntry {
            path: path.to_string(),
            branch: None,
            head_hex: Some("1".repeat(40)),
            bare: false,
            detached: true,
            locked: false,
            lock_reason: String::new(),
            prunable: false,
            prune_reason: String::new(),
            main: false,
            held: None,
        }
    }

    fn paths(worktrees: &[WorktreeEntry]) -> Vec<&str> {
        worktrees.iter().map(|w| w.path.as_str()).collect()
    }

    /// Left out by however git spells the path, and back once the write is
    /// over; whoever left it out is told then, the writer excepted.
    #[test]
    fn a_worktree_is_left_out_while_its_removal_is_under_way() {
        let (writer, reader) = (Arc::new(Counting::default()), Arc::new(Counting::default()));
        let listing = || {
            vec![
                entry("C:/removals-test/repo"),
                entry("C:/removals-test/going"),
            ]
        };
        let removal = Removal::begin("c:\\Removals-Test\\going", &writer);
        assert_eq!(
            paths(&left_out(listing(), &reader)),
            ["C:/removals-test/repo"]
        );
        assert_eq!(paths(&left_out(listing(), &writer)).len(), 1);
        // Twice left out, told once.
        left_out(listing(), &reader);
        drop(removal);
        assert_eq!((reader.asked(), writer.asked()), (1, 0));
        assert_eq!(
            paths(&left_out(listing(), &reader)),
            ["C:/removals-test/repo", "C:/removals-test/going"]
        );
    }

    /// Two writes asked for one worktree: it stays out, and nobody is told,
    /// until both are over.
    #[test]
    fn a_second_removal_of_the_same_worktree_keeps_it_out_until_both_end() {
        let (writer, reader) = (Arc::new(Counting::default()), Arc::new(Counting::default()));
        let listing = || vec![entry("C:/removals-test/twice")];
        let first = Removal::begin("C:/removals-test/twice", &writer);
        let second = Removal::begin("C:/removals-test/twice", &writer);
        assert!(left_out(listing(), &reader).is_empty());
        drop(first);
        assert_eq!(reader.asked(), 0);
        assert!(left_out(listing(), &reader).is_empty());
        drop(second);
        assert_eq!(reader.asked(), 1);
        assert_eq!(
            paths(&left_out(listing(), &reader)),
            ["C:/removals-test/twice"]
        );
    }

    /// A worktree nobody listed meanwhile tells nobody.
    #[test]
    fn a_removal_no_listing_met_tells_nobody() {
        let (writer, reader) = (Arc::new(Counting::default()), Arc::new(Counting::default()));
        let removal = Removal::begin("C:/removals-test/unmet", &writer);
        left_out(vec![entry("C:/removals-test/other")], &reader);
        drop(removal);
        assert_eq!(reader.asked(), 0);
    }
}
