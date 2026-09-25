use super::*;

// A flat list answered by value: `data()` computes the field, where
// `QListModel::get` would lend a stored `NavItem` and force every row to
// exist (rules-refs/app-ui.md「サイドバーの行は射影する」). Rows hang off
// the root; `QAbstractItemModel` is the base that lets a row be answered.
impl QAbstractItemModel for NavSectionModel {
    fn index(&self, row: i32, column: i32, parent: &QModelIndex) -> QModelIndex {
        let out_of_list = parent.is_valid()
            || column != 0
            || usize::try_from(row).is_ok_and(|row| row >= self.shown_rows());
        if out_of_list || row < 0 {
            return QModelIndex::default();
        }
        self.create_index(row, column, 0)
    }

    fn parent(&self, _child: &QModelIndex) -> QModelIndex {
        QModelIndex::default()
    }

    fn row_count(&self, parent: &QModelIndex) -> i32 {
        if parent.is_valid() {
            0
        } else {
            self.shown_rows() as i32
        }
    }

    fn column_count(&self, parent: &QModelIndex) -> i32 {
        if parent.is_valid() { 0 } else { 1 }
    }

    fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let (Some(role), Some(row)) = (
            Role::of(role),
            usize::try_from(index.row())
                .ok()
                .and_then(|at| self.row_at(at)),
        ) else {
            return QVariant::default();
        };
        self.field(row, role).variant()
    }

    /// The names QML resolves a role by, spelled by `Role` and held to the
    /// item's derived table by the test in `role.rs`.
    fn role_names(&self) -> QHash<i32, QByteArray> {
        let mut names = QHash::default();
        for (number, role) in Role::ALL.iter().enumerate() {
            names.insert(&(number as i32), &QByteArray::from(role.spelling()));
        }
        names
    }
}
