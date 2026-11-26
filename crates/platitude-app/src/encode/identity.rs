//! A person as a typed identity names them: the name and the address, cut
//! once by core (`trailers::split_identity`).

use qtbridge::qtbridge_type_lib::QVariantMap;

use super::wire::{Fields, Record, field};

/// `Name <address>` in its two halves; either is empty where the text
/// gives none.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Identity {
    pub name: String,
    pub email: String,
}

impl Identity {
    /// The halves of what was typed (`trailers::split_identity`).
    pub fn typed(text: &str) -> Self {
        let (name, email) = platitude_core::trailers::split_identity(text);
        Self { name, email }
    }
}

/// Whether `email` and `other` are two people as faces go: the address
/// tells them apart, folded as it keys a face (`avatar::key`), each as
/// `.mailmap` spells it — the rule the author card shows a committer by.
/// An empty address is one as recorded (`Name <>`); a caller with one not
/// read yet does not ask.
pub fn two_people(email: &str, other: &str) -> bool {
    platitude_core::avatar::key(email) != platitude_core::avatar::key(other)
}

/// Whether `--reset-author` would change who HEAD is by: its author as git
/// recorded it, `(name, address)`, is not the identity git would record now
/// — any difference in either, as git writes what it is given, with no
/// fold. An empty name is no HEAD read: git refuses an empty `user.name`.
pub fn author_replaced(head: (&str, &str), identity: (&str, &str)) -> bool {
    !head.0.is_empty() && head != identity
}

impl Record for Identity {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("name", &self.name)
            .put("email", &self.email)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Option<Self> {
        Some(Self {
            name: field(map, "name")?,
            email: field(map, "email")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::One;
    use qtbridge::QVariantConvertible;

    #[test]
    fn a_typed_identity_comes_back_in_its_two_halves() {
        let who = One::new(Identity::typed("Ada Lovelace <ada@example.com>"));
        assert_eq!(who.name, "Ada Lovelace");
        assert_eq!(who.email, "ada@example.com");
        let back = <One<Identity> as QVariantConvertible>::try_from_qvariant(&who.to_qvariant());
        assert_eq!(back.map(|b| (*b).clone()), Some((*who).clone()));
    }

    #[test]
    fn two_addresses_are_two_people_as_their_keys_differ() {
        assert!(two_people("yuki.tanaka@example.com", "demo@example.com"));
        assert!(!two_people("demo@example.com", "demo@example.com"));
        assert!(
            !two_people(" Demo@Example.com", "demo@example.com"),
            "folded as a face's key"
        );
        assert!(
            two_people("", "demo@example.com"),
            "an address recorded empty"
        );
        assert!(!two_people("", ""));
    }

    #[test]
    fn reset_author_replaces_any_difference_as_recorded() {
        let me = ("Demo User", "demo@example.com");
        assert!(!author_replaced(me, me));
        assert!(author_replaced(("Yuki Tanaka", "yuki@example.com"), me));
        assert!(
            author_replaced(("Demo", "demo@example.com"), me),
            "the name alone"
        );
        assert!(
            author_replaced(("Demo User", "Demo@Example.com"), me),
            "no fold: git records the spelling it is given"
        );
        assert!(!author_replaced(("", ""), me), "no HEAD read");
    }
}
