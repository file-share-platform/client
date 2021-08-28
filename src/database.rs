use rocket_sync_db_pools::rusqlite::{self, params};
use rocket_sync_db_pools::database;
use crate::structs::Share;

#[database("sqlite_shares")]
pub struct SharesDbConn(rusqlite::Connection);

#[derive(Debug)]
pub enum DatabaseError {
    SqlError(String),
    DoesNotExist
}

impl From<rusqlite::Error> for DatabaseError {
    fn from(error: rusqlite::Error) -> DatabaseError {
        DatabaseError::SqlError(error.to_string())
    }
}

impl std::convert::From<DatabaseError> for (rocket::http::Status, std::string::String) {
    fn from(err: DatabaseError) -> (rocket::http::Status, std::string::String) {
        (rocket::http::Status::new(500), err.to_string()) //TODO
    }
}

///Setup the database. Creates the table(s) required if they do not already exist in the database.db file.
pub async fn setup(conn: &SharesDbConn) -> Result<(), DatabaseError> {
    conn.run(|c| {
        c.execute("CREATE TABLE IF NOT EXISTS shares (
            id INTEGER PRIMARY KEY,
            uuid TEXT NOT NULL,
            usr TEXT NOT NULL,
            exp BIGINT NOT NULL,
            website BOOLEAN NOT NULL,
            wget BOOLEAN NOT NULL,
            name TEXT NOT NULL,
            crt BIGINT INT NOT NULL,
            size BIGINT NOT NULL,
            file_type TEXT NOT NULL
        );", [])
    }).await?;
    Ok(())
}

///Attempts to add a new share to the database.
pub async fn add_to_database(conn: &SharesDbConn, data: Share) -> Result<(), DatabaseError> {
    conn.run(move |c| {
        c.execute("
            INSERT INTO shares (uuid, exp, crt, usr, wget, website, name, size, file_type)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ", params![data.uuid(), data.exp(), data.crt(), data.usr(), data.restrict_wget(), data.restrict_website(), data.name(), data.size(), data.file_type()])
    }).await?;
    Ok(())
}

///Specifies how to search for items in the database, with two options.
pub enum Search {
    #[allow(dead_code)]
    Id(i64),
    Uuid(String),
}

impl Search {
    ///Based on which variant of the enum you are using, generates the search term required to interface with the sqlite database.
    // Note, if adding to this function in the future, ensure to add '' around strings.
    fn get_search_term(self) -> String {
        match self {
            Search::Id(s) => format!("{} = {}", "id", s),
            Search::Uuid(s) => format!("{} = '{}'", "uuid", s),
        }
    }
    ///Run a search, returns the first result it finds in the database, or a DatabaseError if something goes wrong.
    pub async fn find_share<T: FromDatabase<rusqlite::Error> + 'static + Send + Clone>(self, conn: &SharesDbConn) -> Result<T, DatabaseError> {
        let search_result: Vec<T> = search_database(conn, self).await?;
        if search_result.is_empty() {
            return Err(DatabaseError::DoesNotExist);
        }
        Ok(search_result[0].clone()) //Assume first result is correct, user will use search::id() variant if exactness is important.
    }
}

///Implementing this trait means that the struct can be parsed from a database row, or return an error.
pub trait FromDatabase<E>: Sized 
    where E: Send + std::fmt::Debug + Into<rocket_sync_db_pools::rusqlite::Error> 
{
    fn from_database(data: &rocket_sync_db_pools::rusqlite::Row<'_> ) -> Result<Self, E>;
}

///This is a non-public function, utilised by Search.find_share(). It will search a database, matching against criteria. It returns a vec of possible elements which may match the query.
async fn search_database<T: FromDatabase<rusqlite::Error> + 'static + Send>(conn: &SharesDbConn, search: Search) -> Result<Vec<T>, DatabaseError> {
    let result = conn.run(move |c| {
        c.prepare(&format!("Select * FROM shares WHERE {};", search.get_search_term()))
        .and_then(|mut res: rusqlite::Statement| -> Result<Vec<T>, rusqlite::Error> {
            res.query_map([], |row| {
                T::from_database(row)
            }).unwrap().collect()
        })
    }).await?;
    Ok(result)
}


impl From<DatabaseError> for String {
    fn from(err: DatabaseError) -> String {
        return match err {
            DatabaseError::DoesNotExist => "not found in database".to_string(),
            DatabaseError::SqlError(s) => format!("an sql error occured when interfacing with the database: {}", s),
        } 
    }
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DatabaseError::DoesNotExist => f.write_str("not found in database"),
            DatabaseError::SqlError(s) => f.write_str(&format!("an sql error occured when interfacing with the database: {}", s)),
        } 
    }
}