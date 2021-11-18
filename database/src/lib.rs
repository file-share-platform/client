#[macro_use]
extern crate diesel;

#[cfg(not(tarpaulin_include))]
#[doc(hidden)]
pub mod schema;
pub mod models;

use diesel::prelude::*;
pub use diesel::SqliteConnection;
use ws_com_framework::File;

pub use crate::models::Share;

pub fn establish_connection() -> Result<SqliteConnection, ConnectionError> {
    let database_url = "database.db";
    Ok(SqliteConnection::establish(database_url)?)
}

pub fn insert_share(conn: &SqliteConnection, share: &File) -> Result<(), diesel::result::Error> {
    use schema::shares;

    let share: Share = share.into();

    diesel::insert_into(shares::table)
        .values(share)
        .execute(conn)?;

    Ok(())
}

pub fn find_share_by_id(conn: &SqliteConnection, search_id: &str) -> Result<Option<File>, diesel::result::Error> {
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