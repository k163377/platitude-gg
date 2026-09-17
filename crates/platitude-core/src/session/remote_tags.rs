//! What the remotes last advertised under `refs/tags/`, in the shape the
//! refs joins read it back out of.

use super::*;

/// What the remotes last said they carry under `refs/tags/`: for each tag
/// name, every commit some remote has it on and what is known about it
/// there.
///
/// Two commits under one name means the remotes disagree, which reads on
/// screen exactly like a tag that drifted from the one here — the name
/// standing on more than one row.
///
/// **One sorted run.** A tag standing on two
/// commits is rare, so nearly every name has exactly one reading — and a
/// `BTreeMap` per name allocates a whole eleven-slot node to hold that one
/// — a node per name, which on a repository with tens of thousands of
/// remote tags is an order of magnitude more than the same readings take
/// flat, and a large share of the process's entire Rust heap
/// (ci/baseline/code-costs-windows-x64.md §メモリの形). The operations are
/// the ones the two joins need — is this name out there, and walk the
/// names in order — and both are as good on a sorted run as on a tree.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct RemoteTagIndex {
    /// Sorted by name, then by commit. Built once per merge and read many
    /// times, so it is sorted on the way in and never mutated after.
    entries: Vec<RemoteTagEntry>,
}

/// One tag name standing on one commit, as the remotes told it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoteTagEntry {
    pub(crate) name: crate::Name,
    pub(crate) commit: Oid,
    /// Sorted, and more than one when several remotes agree on the commit.
    ///
    /// **Inline while there is one**, which is nearly always: a `Vec` would
    /// be an allocation per tag to hold a single remote's name, and a
    /// repository with 45,901 of them pays that 45,901 times.
    pub(crate) remotes: smallvec::SmallVec<[Carrier; 1]>,
}

/// One remote's reading of one tag.
///
/// **What that remote advertised, kept per remote.** Two remotes can
/// carry the same name on the same commit with one of them holding a tag
/// object and the other pointing straight at the commit, and a single
/// flag for the pair could only be the two OR-ed together. That is lossy
/// in exactly the direction this index has to survive: readings are taken
/// out of it again when a remote is fetched on its own or stops being
/// configured, and a fold cannot be undone.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Carrier {
    pub(crate) remote: crate::Name,
    pub(crate) annotated: bool,
}

impl RemoteTagEntry {
    /// Whether any remote holds this as a tag object.
    pub(crate) fn annotated(&self) -> bool {
        self.remotes.iter().any(|c| c.annotated)
    }
}

impl RemoteTagIndex {
    /// Collects readings into the sorted run. Each `(name, commit)` is one
    /// entry however many remotes carry it, and their names gather on it.
    pub(crate) fn build(
        readings: impl Iterator<Item = (crate::Name, Oid, bool, crate::Name)>,
    ) -> Self {
        let mut entries: Vec<RemoteTagEntry> = Vec::new();
        for (name, commit, annotated, remote) in readings {
            entries.push(RemoteTagEntry {
                name,
                commit,
                remotes: smallvec::smallvec![Carrier { remote, annotated }],
            });
        }
        entries.sort_by(|a, b| a.name.cmp(&b.name).then(a.commit.cmp(&b.commit)));
        let mut folded: Vec<RemoteTagEntry> = Vec::with_capacity(entries.len());
        for entry in entries {
            match folded.last_mut() {
                Some(last) if last.name == entry.name && last.commit == entry.commit => {
                    last.remotes.extend(entry.remotes);
                }
                _ => folded.push(entry),
            }
        }
        for entry in &mut folded {
            entry.remotes.sort();
            entry.remotes.dedup_by(|a, b| a.remote == b.remote);
        }
        folded.shrink_to_fit();
        Self { entries: folded }
    }

    /// Every reading held, in the shape [`Self::build`] takes them back.
    ///
    /// **This is what makes the index the only copy.** Keeping the
    /// per-remote answers beside it, so one remote's could be replaced on
    /// its own, is a second copy of every name — megabytes of the memory
    /// budget (ci/baseline/code-costs-windows-x64.md §メモリの形) — for
    /// data already here. Taking them out again costs one pass.
    pub(crate) fn readings(
        &self,
    ) -> impl Iterator<Item = (crate::Name, Oid, bool, crate::Name)> + '_ {
        self.entries.iter().flat_map(|e| {
            e.remotes
                .iter()
                .map(move |c| (e.name.clone(), e.commit, c.annotated, c.remote.clone()))
        })
    }

    /// Whether any remote carries this name at all.
    pub(crate) fn carries(&self, name: &str) -> bool {
        self.entries
            .binary_search_by(|e| e.name.as_str().cmp(name))
            .is_ok()
    }

    /// Whether the graph's cloud belongs on this repository's copy of the
    /// name: some remote carries it, and every remote that does has it on
    /// the commit it is on here (デザイン規約 §ref の種別). A drift stands
    /// the name on two rows, and neither of them wears the badge.
    ///
    /// **One search.** This is answered for every tag on every refs
    /// read, and `JetBrains/kotlin` brings 45,901 of them
    /// (`RefJoins`) — a second walk of the same run costs the whole
    /// of that again for an answer this one already
    /// has.
    pub(crate) fn agrees_at(&self, name: &str, here: Oid) -> bool {
        let start = self.entries.partition_point(|e| e.name.as_str() < name);
        let mut carried = false;
        for entry in self.entries[start..]
            .iter()
            .take_while(|e| e.name.as_str() == name)
        {
            if entry.commit != here {
                return false;
            }
            carried = true;
        }
        carried
    }

    /// The names in order, each with every reading of it. One name is one
    /// run of the sorted entries.
    pub(crate) fn names(&self) -> impl Iterator<Item = (&str, &[RemoteTagEntry])> {
        let mut rest = self.entries.as_slice();
        std::iter::from_fn(move || {
            let (first, _) = rest.split_first()?;
            let name = first.name.as_str();
            let end = rest.partition_point(|e| e.name == name);
            let (run, tail) = rest.split_at(end);
            rest = tail;
            Some((name, run))
        })
    }

    /// Readings held, across every name.
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }
}

impl crate::mem::Footprint for RemoteTagIndex {
    fn heap_bytes(&self) -> usize {
        self.entries.heap_bytes()
    }
}

impl crate::mem::Footprint for RemoteTagEntry {
    fn heap_bytes(&self) -> usize {
        self.name.heap_bytes() + self.remotes.heap_bytes()
    }
}

impl crate::mem::Footprint for Carrier {
    fn heap_bytes(&self) -> usize {
        self.remote.heap_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(byte: u8) -> Oid {
        Oid::from_hex_str(&format!("{byte:02x}").repeat(20)).expect("valid sha")
    }

    /// Taking one remote's readings back out leaves the others exactly as
    /// they were told — including the one thing a folded flag would lose.
    ///
    /// Two remotes can carry a name on the same commit with one holding a
    /// tag object and the other pointing straight at it (measured: only
    /// the annotated side advertises the `^{}` line). The index is the
    /// only copy of the readings, so a fetch of one remote rebuilds from
    /// what it hands back — and if `annotated` were one flag per entry it
    /// could only be the two OR-ed, and dropping the annotated side would
    /// leave the lightweight one still calling itself annotated.
    #[test]
    fn a_remotes_readings_come_back_out_the_way_they_went_in() {
        let commit = oid(7);
        let both = RemoteTagIndex::build(
            [
                (
                    crate::Name::from("v1"),
                    commit,
                    true,
                    crate::Name::from("up"),
                ),
                (
                    crate::Name::from("v1"),
                    commit,
                    false,
                    crate::Name::from("mirror"),
                ),
            ]
            .into_iter(),
        );
        assert_eq!(both.len(), 1, "one name on one commit is one entry");
        assert!(
            both.entries[0].annotated(),
            "one of the two holds an object"
        );

        // `up` stops being configured: rebuild from what the index hands
        // back, minus its readings.
        let without_up = RemoteTagIndex::build(
            both.readings()
                .filter(|(_, _, _, remote)| remote.as_str() != "up"),
        );
        assert_eq!(without_up.len(), 1);
        assert!(
            !without_up.entries[0].annotated(),
            "what is left is the lightweight reading, and says so"
        );

        let without_mirror = RemoteTagIndex::build(
            both.readings()
                .filter(|(_, _, _, remote)| remote.as_str() != "mirror"),
        );
        assert!(without_mirror.entries[0].annotated());
    }
}
