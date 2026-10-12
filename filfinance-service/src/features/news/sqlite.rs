//! Couche de persistence SQLite et bootstrap du schema applicatif.

// `SqlitePool` represente le pool de connexions partage par les sous-modules SQLite.
use sqlx::SqlitePool;

use crate::errors::AppError;

// `blacklist` reserve le CRUD des regles de rejet et les audits associes.
mod blacklist;
// `cleanup` regroupe les purges ciblees de donnees connues comme invalides.
mod cleanup;
// `glossary` reserve le CRUD du glossaire traduction/TTS.
mod glossary;
// `queries` reserve les lectures et paginations SQLite de la feature news.
mod queries;
// `retention` reserve la purge periodique des anciennes news persistantes.
mod retention;
// `schema` reserve l'ouverture du pool et le bootstrap idempotent des tables SQLite.
mod schema;
// `statements` centralise les projections SQL partagees par les lectures et reparations.
mod statements;
// `writes` reserve les insertions, upserts et reparations SQLite.
mod writes;

// Le type de rapport reste disponible via `news::sqlite` pour ne pas changer les imports existants.
#[allow(unused_imports)]
pub use cleanup::BrowserSourceCleanupReport;
#[allow(unused_imports)]
pub use retention::NewsRetentionPruneReport;

/// Enveloppe le pool SQLite et centralise les acces au stockage.
// `#[derive(Clone)]` permet de recopier legerement un handle de stockage vers plusieurs handlers/workers.
#[derive(Clone)]
pub struct SqliteNewsStore {
    // Le pool SQLx est garde prive pour imposer un point d'acces unique via les methodes du repository.
    pool: SqlitePool,
}

impl SqliteNewsStore {
    /// Verifie rapidement que le pool SQLite repond sans exposer l'URL de base.
    pub(crate) async fn health_check(&self) -> Result<(), AppError> {
        sqlx::query("SELECT 1").fetch_one(&self.pool).await?;
        Ok(())
    }

    /// Expose un snapshot non sensible du pool pour l'observabilite admin.
    pub(crate) fn pool_snapshot(&self) -> (u32, usize) {
        (self.pool.size(), self.pool.num_idle())
    }
}
