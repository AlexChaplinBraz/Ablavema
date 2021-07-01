use crate::package::Package;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, Deserialize, PartialEq, Serialize)]
pub enum SortBy {
    NameAscending,
    NameDescending,
    DateAscending,
    DateDescending,
    VersionAscending,
    VersionDescending,
}

impl SortBy {
    pub const ALL: [SortBy; 6] = [
        SortBy::NameAscending,
        SortBy::NameDescending,
        SortBy::DateAscending,
        SortBy::DateDescending,
        SortBy::VersionAscending,
        SortBy::VersionDescending,
    ];

    pub fn get_ordering(&self, a: &Package, b: &Package) -> std::cmp::Ordering {
        match self {
            SortBy::NameAscending => Ord::cmp(&a.name, &b.name),
            SortBy::NameDescending => Ord::cmp(&a.name, &b.name).reverse(),
            SortBy::DateAscending => Ord::cmp(&a.date, &b.date),
            SortBy::DateDescending => Ord::cmp(&a.date, &b.date).reverse(),
            SortBy::VersionAscending => Ord::cmp(&a.version, &b.version),
            SortBy::VersionDescending => Ord::cmp(&a.version, &b.version).reverse(),
        }
    }
}

impl Default for SortBy {
    fn default() -> Self {
        Self::VersionDescending
    }
}

impl std::fmt::Display for SortBy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SortBy::NameAscending => " Name [A]",
                SortBy::NameDescending => " Name [D]",
                SortBy::DateAscending => " Date [A]",
                SortBy::DateDescending => " Date [D]",
                SortBy::VersionAscending => " Version [A]",
                SortBy::VersionDescending => " Version [D]",
            }
        )
    }
}
