// #![warn(
//     missing_docs,
//     missing_debug_implementations,
//     missing_copy_implementations,
//     trivial_casts,
//     trivial_numeric_casts,
//     unsafe_code,
//     unstable_features,
//     unused_import_braces,
//     unused_qualifications,
//     deprecated
// )]

#[macro_use]
extern crate diesel;

pub mod models;
#[cfg(not(tarpaulin_include))]
#[doc(hidden)]
pub mod schema;

use diesel::prelude::*;
pub use diesel::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use ws_com_framework::FileId;

pub use crate::models::Share;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub fn establish_connection(
    database_url: &str,
) -> Result<SqliteConnection, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let mut conn = SqliteConnection::establish(database_url)?;
    conn.exclusive_transaction(move |conn| {
        conn.run_pending_migrations(MIGRATIONS).map(|_| ())
    })?;
    Ok(conn)
}

pub fn insert_share(
    conn: &mut SqliteConnection,
    share: &Share,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    use schema::shares;

    diesel::insert_into(shares::table)
        .values(share)
        .execute(conn)?;

    Ok(())
}

pub fn find_share_by_id(
    conn: &mut SqliteConnection,
    search_id: &FileId,
) -> Result<Option<Share>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    use schema::shares::dsl::*;
    let mut f = shares
        .filter(file_id.eq(*search_id as i32))
        .load::<Share>(conn)?;

    if f.is_empty() {
        Ok(None)
    } else {
        Ok(Some(f.remove(0)))
    }
}
