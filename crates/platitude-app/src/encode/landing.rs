//! Where a walk over a file list lands: the row, and the file it names.

use qtbridge::qtbridge_type_lib::QVariantMap;

use super::wire::{Fields, Optional, Record, field};

/// The file row a step of the arrows lands on. `bucket` is empty for a
/// commit's changed files, which sit in none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Landing {
    pub row: i32,
    pub bucket: String,
    pub path: String,
}

/// A landing, or nowhere left to go — how the arrows stop at the ends
/// and pass a bucket holding no files.
pub type Landed = Optional<Landing>;

impl Record for Landing {
    fn to_map(&self) -> QVariantMap {
        Fields::new()
            .put("row", &self.row)
            .put("bucket", &self.bucket)
            .put("path", &self.path)
            .done()
    }

    fn from_map(map: &QVariantMap) -> Result<Self, ()> {
        Ok(Self {
            row: field(map, "row")?,
            bucket: field(map, "bucket")?,
            path: field(map, "path")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qtbridge::QVariantConvertible;

    #[test]
    fn a_landing_round_trips_and_nowhere_is_nothing() {
        let landed = Landed::some(Landing {
            row: 3,
            bucket: "staged".into(),
            path: "src/日本語.txt".into(),
        });
        let back = <Landed as QVariantConvertible>::try_from_qvariant(&landed.to_qvariant());
        assert_eq!(back.map(|b| (*b).clone()), Ok((*landed).clone()));
        assert_eq!(landed.as_ref().map(|l| l.row), Some(3));
        assert!(!Landed::none().to_qvariant().is_valid());
    }
}
