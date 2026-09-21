//! The people a commit message credits beside its author, as QML draws
//! them: a name, an address and the identicon the name picks.

use platitude_core::details::CoAuthor;
use qtbridge::qtbridge_type_lib::QVariantMap;

use super::graph::avatar_code;
use super::wire::{Fields, Listed, Record, field};

/// One `Co-authored-by` trailer, ready to draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mate {
    pub name: String,
    /// Empty when the trailer had none.
    pub email: String,
    /// The identicon code off the name — the same one the graph rows
    /// draw for an author (`avatar_code`), so one person wears one face.
    pub face: i32,
}

/// Everyone a message credits, in the order the message lists them.
pub type Mates = Listed<Mate>;

pub fn mates_of(mates: &[CoAuthor]) -> Mates {
    Listed::new(
        mates
            .iter()
            .map(|m| Mate {
                name: m.name.clone(),
                email: m.email.clone(),
                face: avatar_code(&m.name),
            })
            .collect(),
    )
}

impl Record for Mate {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("name", &self.name)
            .put("email", &self.email)
            .put("face", &self.face)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            name: field(map, "name")?,
            email: field(map, "email")?,
            face: field(map, "face")?,
        })
    }
}

impl platitude_core::mem::Footprint for Mate {
    fn heap_bytes(&self) -> usize {
        self.name.heap_bytes() + self.email.heap_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qtbridge::qtbridge_type_lib::QVariant;

    #[test]
    fn a_mate_carries_name_address_and_the_face_off_the_name() {
        let mates = mates_of(&[
            CoAuthor {
                name: "Claude Opus 5".into(),
                email: "noreply@anthropic.com".into(),
            },
            CoAuthor {
                name: "Nameless".into(),
                email: String::new(),
            },
        ]);
        assert_eq!(mates.len(), 2);
        assert_eq!(mates[0].name, "Claude Opus 5");
        assert_eq!(mates[0].email, "noreply@anthropic.com");
        assert_eq!(
            mates[0].face,
            avatar_code("Claude Opus 5"),
            "the face comes off the name, the way the graph rows' do"
        );
        assert_eq!(mates[1].email, "");
        assert_eq!(Mates::try_from(&QVariant::from(&mates)), Ok(mates));
    }

    #[test]
    fn nobody_credited_is_an_empty_list() {
        assert_eq!(mates_of(&[]), Mates::default());
    }
}
