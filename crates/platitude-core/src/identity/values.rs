//! The name and the address a write hands git.

/// The `user.name` and `user.email` a write asks for, each trimmed as git
/// would trim it — so the stored configuration equals what commits show,
/// and a write's first pass, its second and the read-back it is compared
/// with all go by one spelling (`read::parse_config` trims what git holds
/// the same way).
///
/// An empty half is kept as empty: what it means is the writer's —
/// [`super::set_identity`] refuses an empty name and writes an empty
/// address, [`super::set_local_identity`] takes an emptied key out.
///
/// What a read found is [`super::Identity`], where a key nobody wrote is
/// `None`; this is what a write asks for, and is never read back into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityValues {
    name: String,
    email: String,
}

impl IdentityValues {
    pub fn new(name: &str, email: &str) -> Self {
        Self {
            name: name.trim().to_string(),
            email: email.trim().to_string(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    /// The name as a read reports it once written: an empty one is no
    /// value there, whether the write emptied the key or took it out
    /// (`read::parse_config`).
    pub fn name_read(&self) -> Option<&str> {
        Some(self.name.as_str()).filter(|name| !name.is_empty())
    }

    /// The address, the same way.
    pub fn email_read(&self) -> Option<&str> {
        Some(self.email.as_str()).filter(|email| !email.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::IdentityValues;

    /// Both halves lose the blanks around them and keep the ones inside; a
    /// half that was only blanks is empty, not refused.
    #[test]
    fn both_halves_are_trimmed_and_an_empty_one_is_kept() {
        let values = IdentityValues::new("  Ada Lovelace\t", " ");
        assert_eq!(values.name(), "Ada Lovelace");
        assert_eq!(values.email(), "");
        assert_eq!(values.name_read(), Some("Ada Lovelace"));
        assert_eq!(values.email_read(), None, "an empty half reads as none");
    }
}
