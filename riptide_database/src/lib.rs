//! Database abstraction for the riptide client-side application

#![warn(
    missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unstable_features,
    unused_import_braces,
    unused_qualifications,
    deprecated
)]

#[macro_use]
extern crate diesel;

pub mod models;
#[cfg(not(tarpaulin_include))]
#[doc(hidden)]
#[allow(missing_docs)]
pub mod schema;

use std::time::UNIX_EPOCH;

use diesel::prelude::*;
pub use diesel::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use ws_com_framework::FileId;

pub use crate::models::Share;

/// migration to initalise the database
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

/// Create a new connection pool to the database
pub fn establish_connection(
    database_url: &str,
) -> Result<SqliteConnection, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let mut conn = SqliteConnection::establish(database_url)?;
    conn.exclusive_transaction(move |conn| conn.run_pending_migrations(MIGRATIONS).map(|_| ()))?;
    Ok(conn)
}

/// Insert a new share into the database
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

/// Attempt to find a share, searching by its ID and filter by username
pub fn get_share(
    conn: &mut SqliteConnection,
    search_id: &FileId,
    username: &str,
) -> Result<Option<Share>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    use schema::shares::dsl::*;
    let mut f = shares
        .filter(file_id.eq(*search_id as i64))
        .filter(user_name.eq(username))
        .load::<Share>(conn)?;

    if f.is_empty() {
        Ok(None)
    } else {
        Ok(Some(f.remove(0)))
    }
}

/// Attempt to find a share only by a given ID
pub fn get_share_by_id(
    conn: &mut SqliteConnection,
    search_id: &FileId,
) -> Result<Option<Share>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    use schema::shares::dsl::*;
    let mut f = shares
        .filter(file_id.eq(*search_id as i64))
        .load::<Share>(conn)?;

    if f.is_empty() {
        Ok(None)
    } else {
        Ok(Some(f.remove(0)))
    }
}

/// Attempt to get all shares currently in the database
pub fn get_shares(
    conn: &mut SqliteConnection,
    username: &str,
) -> Result<Vec<Share>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    use schema::shares::dsl::*;
    let f = shares.filter(user_name.eq(username)).load::<Share>(conn)?;

    Ok(f)
}

/// Attempt to remove a share from the database
pub fn remove_share(
    conn: &mut SqliteConnection,
    id: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    use schema::shares::dsl::*;
    diesel::delete(shares.filter(file_id.eq(id as i64))).execute(conn)?;

    Ok(())
}

/// Attempt to remove all shares from the database
pub fn remove_all_shares(
    conn: &mut SqliteConnection,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    use schema::shares::dsl::*;
    diesel::delete(shares).execute(conn)?;

    Ok(())
}

/// Collect all expired shares from the database
pub fn remove_expired_shares(
    conn: &mut SqliteConnection,
) -> Result<Vec<Share>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    use schema::shares::dsl::*;

    let now = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_secs() as i64;

    let f: Vec<Share> = diesel::delete(shares.filter(exp.lt(now)))
        .returning(shares::all_columns())
        .get_results::<Share>(conn)?;

    Ok(f)
}
