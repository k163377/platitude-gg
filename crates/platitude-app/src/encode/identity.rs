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
}
