#[macro_use]
extern crate diesel;

#[macro_use]
extern crate diesel_migrations;

pub mod models;
#[cfg(not(tarpaulin_include))]
#[doc(hidden)]
pub mod schema;

use diesel::prelude::*;
pub use diesel::SqliteConnection;
use diesel_migrations::embed_migrations;
use ws_com_framework::FileId;

pub use crate::models::Share;

embed_migrations!("./migrations/");

pub fn establish_connection(
    database_url: &str,
) -> Result<SqliteConnection, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let conn = SqliteConnection::establish(database_url)?;
    embedded_migrations::run(&conn)?;
    Ok(conn)
}

pub fn insert_share(
    conn: &SqliteConnection,
    share: &Share,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    use schema::shares;

    diesel::insert_into(shares::table)
        .values(share)
        .execute(conn)?;

    Ok(())
}

pub fn find_share_by_id(
    conn: &SqliteConnection,
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
