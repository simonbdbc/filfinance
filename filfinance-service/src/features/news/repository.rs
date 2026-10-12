//! Repository news expose par la facade de feature.

// Le backend news reste SQLite dans cette passe, mais le reste de l'application depend du nom
// metier `NewsStore` plutot que de l'adapter concret `sqlite::SqliteNewsStore`.
pub(crate) type NewsStore = crate::features::news::sqlite::SqliteNewsStore;
