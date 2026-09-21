//! Where a walk over a file list lands: the row, and the file it names.

use qtbridge::qtbridge_type_lib::QVariantMap;

use super::wire::{Fields, Optional, Record, field};

/// The file row a step of the arrows lands on: which row of the list it
/// is (what the view is asked to bring on screen), the bucket the file
/// sits in — empty for a commit's changed files, which sit in none — and
/// the path it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Landing {
    pub row: i32,
    pub bucket: String,
    pub path: String,
}

/// A landing, or nowhere left to go — which is how the arrows stop at
/// the ends, and how a walk goes past a bucket holding no files.
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
    use qtbridge::qtbridge_type_lib::QVariant;

    #[test]
    fn a_landing_round_trips_and_nowhere_is_nothing() {
        let landed = Landed::some(Landing {
            row: 3,
            bucket: "staged".into(),
            path: "src/日本語.txt".into(),
        });
        assert_eq!(
            Landed::try_from(&QVariant::from(&landed)),
            Ok(landed.clone())
        );
        assert_eq!(landed.as_ref().map(|l| l.row), Some(3));
        assert!(!QVariant::from(&Landed::none()).is_valid());
    }
}
