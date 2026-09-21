//! The tree a test stands its files in, and the one place every one of
//! them stands.
//!
//! **One entry under the system temp for the whole suite.** A name flat
//! under the temp directory, carrying the pid so that parallel tests do
//! not meet, is a name the next run never spells again — so nothing
//! ever removed one, and ten thousand of them collected (measured: a
//! quarter of everything in the Windows temp directory). Under one base
//! the suite costs that directory a single entry whatever it leaves,
//! and what a killed run leaves is bounded by the same day-old sweep
//! the run directories answer to
//! ([`crate::verify::sweep_yesterdays_runs`]).

/// The one directory the tests' trees stand under. Named among
/// [`crate::verify`]'s run bases, which is what sweeps it, and spelled
/// again in `tests/gate/support.rs` — an integration binary is a crate
/// of its own and cannot read this. That second spelling is held to
/// this one by a test below, because a base the sweep does not know is
/// a base nothing ever takes away.
pub(crate) const BASE: &str = "pgg-tests";

/// A tree of one test's own, taken away when it goes out of scope —
/// **including when the test panics**, which is the half that a removal
/// written at the end of a test body never covers, and the half a
/// failing suite leaves most of.
///
/// The name is [`crate::verify::claim_dir`]'s: the suite runs in
/// parallel by rule (CLAUDE.md Rust 規約), and an exclusive `create_dir`
/// is the only thing that tells a directory this call made from one
/// that was already standing there. It spells nothing a shell reads —
/// no thread id, whose `Debug` form has parentheses in it — because
/// some of these paths are written into wrapper scripts
/// (`linux::container`).
#[cfg(test)]
pub(crate) struct Yard(std::path::PathBuf);

#[cfg(test)]
impl Yard {
    #[must_use]
    pub(crate) fn new(what: &str) -> Self {
        let base = std::env::temp_dir().join(BASE);
        Self(crate::verify::claim_dir(&base, what).expect("a tree of this test's own"))
    }
}

/// **The guard stands where the path stood**, so a test says what it
/// always said (`dir.join(…)`, `&dir`) and the removal is the only
/// thing added to it. The trap runs the other way: a yard nobody binds
/// is dropped at the end of its own statement, and the tree is gone
/// before the assertion — so `Yard::new(…).join(…)` is a bug, and every
/// caller binds.
#[cfg(test)]
impl std::ops::Deref for Yard {
    type Target = std::path::Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// For the calls that take the path by value — `std::fs`'s own, which
/// are generic and so out of the coercion's reach.
#[cfg(test)]
impl AsRef<std::path::Path> for Yard {
    fn as_ref(&self) -> &std::path::Path {
        &self.0
    }
}

/// **One attempt, and nothing said.** A tree a live process is still in
/// is one Windows refuses to remove, and that is the same question
/// `verify::ownership::give_back_claimed` asks outright before it takes
/// a run's roots: a directory something is still writing into is
/// nobody's to remove.
///
/// **One test's tree is exactly that, and correctly so.**
/// `verify::look`'s `a_child_the_diagnostic_leaves_behind…` is about a
/// child that outlives the diagnostic holding the stdout it inherited —
/// a file that stands in this tree — and the child has to still be
/// there when the test asserts, or there is nothing to assert. So that
/// one tree stands until the day-old sweep takes it
/// ([`crate::verify::sweep_yesterdays_runs`]), which is what [`BASE`] is
/// for.
///
/// **Retrying would not reach it.** The child is reparented — `start /b`
/// and `( … ) &` leave no pid behind — and the file it writes appearing
/// is not the process having gone, so a removal that waited for either
/// would be a flake in place of a leaving nobody sees.
#[cfg(test)]
impl Drop for Yard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::BASE;

    /// **The gate suite's sandboxes stand under [`BASE`] too**, and the
    /// only thing that can say so is this: an integration binary is a
    /// crate of its own, so `tests/gate/support.rs` spells the name
    /// rather than reading it. A rename here that left that spelling
    /// behind would put its roots under a directory
    /// `verify::ownership::RUN_BASES` does not name — and the ones it
    /// leaves are the ones whose removal a live git refused, so nothing
    /// would ever take them away again.
    ///
    /// The value and not the line: what has to agree is the name of the
    /// directory, and renaming the constant that holds it over there is
    /// nobody's mistake.
    #[test]
    fn the_gate_suite_spells_the_same_base() {
        let support =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/gate/support.rs");
        let text = std::fs::read_to_string(&support).expect("the gate suite's support file");
        assert!(
            text.contains(&format!("\"{BASE}\"")),
            "{} stands its sandboxes somewhere {BASE} does not cover",
            support.display()
        );
    }
}
