//! What the remotes last advertised under `refs/tags/`, in the shape the
//! refs joins read it back out of.

use super::*;

/// What the remotes last said they carry under `refs/tags/`: for each tag
/// name, every commit some remote has it on and what is known about it
/// there.
///
/// Two commits under one name means the remotes disagree, which reads on
/// screen like a tag that drifted from the one here — the name standing
/// on more than one row.
///
/// One sorted run, not a map per name: nearly every name has one
/// reading, and a `BTreeMap` node per name is an order of magnitude more
/// heap than the flat run (ci/baseline/code-costs-windows-x64.md
/// §メモリの形).
///
/// Public because the sidebar snapshot carries a pointer to it
/// ([`crate::session::RefsSnapshot::remote_tags`]) rather than a second
/// copy of every name. What is public is the reading
/// ([`Self::carriers_against`]); the run itself is this module's.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct RemoteTagIndex {
    /// Sorted by name, then by commit; never mutated after `build`.
    entries: Vec<RemoteTagEntry>,
}

/// One tag name standing on one commit, as the remotes told it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoteTagEntry {
    pub(crate) name: crate::Name,
    pub(crate) commit: Oid,
    /// Sorted; more than one when several remotes agree on the commit.
    /// Inline while there is one (nearly always) — a `Vec` would be an
    /// allocation per tag.
    pub(crate) remotes: smallvec::SmallVec<[Carrier; 1]>,
}

/// One remote's reading of one tag.
///
/// `annotated` is per remote: two remotes can carry one name on one
/// commit, one as a tag object and one pointing straight at the commit,
/// and a flag OR-ed for the pair could not be undone when one remote's
/// readings are taken back out (fetched on its own, or unconfigured).
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
    /// Collects readings into the sorted run: one entry per
    /// `(name, commit)`, with the remotes carrying it gathered on it.
    ///
    /// The only constructor; public so a snapshot can be built on nothing
    /// but this index (`models::nav::testkit`).
    pub fn build(readings: impl Iterator<Item = (crate::Name, Oid, bool, crate::Name)>) -> Self {
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

    /// Every reading held, in the shape [`Self::build`] takes them back —
    /// what lets the index be the only copy: replacing one remote's
    /// readings rebuilds from this instead of keeping a per-remote copy of
    /// every name beside it (ci/baseline/code-costs-windows-x64.md
    /// §メモリの形).
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

    /// The remotes carrying this name, each said once and in name order,
    /// with whether it stands somewhere other than where `against` has it
    /// — what a tag's row opens under itself
    /// (デザイン規約 §左メニューの所作).
    ///
    /// `against` is the remote this repository's tag rows act on, not the
    /// local tag: that is one opinion among the readings, and as the
    /// reference it would put the warning on whichever remote disagrees
    /// with a local tag that may itself be the odd one out. Where
    /// `against` does not carry the name, nobody stands apart.
    pub fn carriers_against(&self, name: &str, against: &str) -> Vec<(&str, bool)> {
        let start = self.entries.partition_point(|e| e.name.as_str() < name);
        let run = || {
            self.entries[start..]
                .iter()
                .take_while(|e| e.name.as_str() == name)
        };
        let reference = run()
            .find(|e| e.remotes.iter().any(|c| c.remote.as_str() == against))
            .map(|e| e.commit);
        let mut out: Vec<(&str, bool)> = run()
            .flat_map(|e| {
                let apart = reference.is_some_and(|here| here != e.commit);
                e.remotes.iter().map(move |c| (c.remote.as_str(), apart))
            })
            .collect();
        out.sort_unstable();
        out.dedup_by(|a, b| a.0 == b.0);
        out
    }

    /// Whether the graph's cloud belongs on this repository's copy of the
    /// name: some remote carries it, and every remote that does has it on
    /// the commit it is on here (デザイン規約 §ref の種別). A drift stands
    /// the name on two rows, and neither of them wears the badge.
    ///
    /// One walk of the run: this is asked for every tag on every refs read
    /// (`RefJoins`).
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

    /// The names in order, each with every reading of it.
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

    /// Entries held — one per `(name, commit)`.
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

    /// Pins the per-remote `annotated` (`Carrier`): with one flag per
    /// entry, dropping the annotated side would leave the lightweight one
    /// still calling itself annotated.
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
