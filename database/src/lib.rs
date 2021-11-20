#[macro_use]
extern crate diesel;

#[macro_use]
extern crate diesel_migrations;

pub mod models;
#[cfg(not(tarpaulin_include))]
#[doc(hidden)]
pub mod schema;

use std::error::Error;

use diesel::prelude::*;
pub use diesel::SqliteConnection;
use diesel_migrations::embed_migrations;
use ws_com_framework::File;

pub use crate::models::Share;

embed_migrations!("./migrations/");

pub fn establish_connection() -> Result<SqliteConnection, Box<dyn Error>> {
    let database_url = "database.db";
    let conn = SqliteConnection::establish(database_url)?;
    embedded_migrations::run(&conn)?;
    Ok(conn)
}

pub fn insert_share(conn: &SqliteConnection, share: &File) -> Result<(), diesel::result::Error> {
    use schema::shares;

    let share: Share = share.into();

    diesel::insert_into(shares::table)
        .values(share)
        .execute(conn)?;

    Ok(())
}

pub fn find_share_by_id(
    conn: &SqliteConnection,
    search_id: &[u8],
) -> Result<Option<File>, diesel::result::Error> {
    use schema::shares::dsl::*;
    let mut f = shares
        .filter(id.eq(search_id))
        .limit(1)
        .load::<Share>(conn)?;

    if f.is_empty() {
        Ok(None)
    } else {
        Ok(Some(f.remove(0).into()))
    }
}
