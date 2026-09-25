//! The tree a test stands its files in, and the one place every one of
//! them stands.
//!
//! One entry under the system temp for the whole suite: a pid-named
//! entry flat under temp is a name no later run spells again, so nothing
//! ever removes it. What a killed run leaves under [`BASE`] is taken by
//! the day-old sweep ([`crate::verify::sweep_yesterdays_runs`]).

/// The one directory the tests' trees stand under. Named among
/// [`crate::verify`]'s run bases, which is what sweeps it, and spelled
/// again in `tests/gate/support.rs` (another crate), held to this one by
/// a test below.
pub(crate) const BASE: &str = "pgg-tests";

/// A tree of one test's own, taken away when it goes out of scope —
/// including when the test panics, which a removal at the end of a test
/// body never covers.
///
/// The name is [`crate::verify::claim_dir`]'s: under parallel tests only
/// an exclusive `create_dir` tells a directory this call made from one
/// already standing. It spells nothing a shell reads (no thread id, whose
/// `Debug` form has parentheses), because some of these paths are written
/// into wrapper scripts (`linux::container`).
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

/// The trap: a yard nobody binds is dropped at the end of its own
/// statement, and the tree is gone before the assertion — so
/// `Yard::new(…).join(…)` is a bug, and every caller binds.
#[cfg(test)]
impl std::ops::Deref for Yard {
    type Target = std::path::Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// For `std::fs`'s generic calls, which deref coercion does not reach.
#[cfg(test)]
impl AsRef<std::path::Path> for Yard {
    fn as_ref(&self) -> &std::path::Path {
        &self.0
    }
}

/// One attempt, and nothing said: Windows refuses to remove a tree a live
/// process is still in, and one test's tree is that by design —
/// `verify::look`'s `a_child_the_diagnostic_leaves_behind…` needs its
/// child, which inherited a stdout file in this tree, alive at the
/// assert. That tree waits for the day-old sweep ([`BASE`]). Retrying
/// would not reach it: the reparented child leaves no pid to wait on, and
/// its file appearing is not the process gone.
#[cfg(test)]
impl Drop for Yard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::BASE;

    /// A rename here that left `tests/gate/support.rs`'s spelling behind
    /// would put the gate suite's sandboxes under a directory
    /// `verify::ownership::RUN_BASES` does not name, which nothing ever
    /// sweeps. The value is compared and not the line: renaming the
    /// constant over there is nobody's mistake.
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
