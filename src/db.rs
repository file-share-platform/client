//! Contains all database functionality

use crate::error::Error;
use chrono::prelude::*;
use mobc::{Connection, Pool};
use mobc_postgres::{tokio_postgres, PgConnectionManager};
use std::fs;
use std::str::FromStr;
use std::time::Duration;
use tokio_postgres::{Config, NoTls, Row};
use ws_com_framework::File;

/// Maximum number of open db connections
const DB_POOL_MAX_OPEN: u64 = 32;

/// Maximum number of idle db connections
const DB_POOL_MAX_IDLE: u64 = 8;

/// How long we will wait for a db connection before timing out.
const DB_POOL_TIMEOUT_SECONDS: u64 = 15;

/// The location of the initalisation file for the db.
const INIT_SQL: &str = "./config/db.sql";

pub type DBCon = Connection<PgConnectionManager<NoTls>>;
pub type DBPool = Pool<PgConnectionManager<NoTls>>;

/// A value can be pulled from the databse if it has this trait implemented.
pub trait FromDataBase: Sized {
    type Error: Send + std::fmt::Debug + Into<Error>;
    fn from_database(data: &Row) -> Result<Self, Self::Error>;
}

impl FromDataBase for File {
    type Error = Error;
    fn from_database(data: &Row) -> Result<Self, Self::Error> {
        let id: uuid::Uuid = data.try_get::<usize, uuid::Uuid>(1).unwrap();
        let created_at: DateTime<Utc> = data.try_get::<usize, DateTime<Utc>>(2).unwrap(); //Test timestamp
        let expires: DateTime<Utc> = data.try_get::<usize, DateTime<Utc>>(3).unwrap(); //Test timestamp
        let usr: String = data.try_get::<usize, String>(4).unwrap();
        let website: bool = data.try_get::<usize, bool>(5).unwrap();
        let wget: bool = data.try_get::<usize, bool>(6).unwrap();
        let file_name: String = data.try_get::<usize, String>(7).unwrap();
        let size: i64 = data.try_get::<usize, i64>(8).unwrap();
        let file_type: String = data.try_get::<usize, String>(9).unwrap();

        let f = File::new(
            id,
            created_at,
            expires,
            usr,
            website,
            wget,
            file_name,
            size as usize,
            file_type,
            0,
        );
        Ok(f)
    }
}

pub fn create_pool() -> Result<DBPool, mobc::Error<tokio_postgres::Error>> {
    let config = Config::from_str("postgres://postgres@127.0.0.1:7877/postgres")?; //TODO load this from config file

    let manager = PgConnectionManager::new(config, NoTls);
    Ok(Pool::builder()
        .max_open(DB_POOL_MAX_OPEN)
        .max_idle(DB_POOL_MAX_IDLE)
        .get_timeout(Some(Duration::from_secs(DB_POOL_TIMEOUT_SECONDS)))
        .build(manager))
}

pub async fn get_db_con(pool: &DBPool) -> Result<DBCon, Error> {
    pool.get().await.map_err(Error::DBPool)
}

pub async fn init_db(pool: &DBPool) -> Result<(), Error> {
    let init_file = fs::read_to_string(INIT_SQL)?;
    let conn = get_db_con(pool).await?;
    conn.batch_execute(&init_file)
        .await
        .map_err(Error::DBInit)?;
    Ok(())
}

pub enum Search {
    Id(usize),
    uuid(uuid::Uuid),
}

impl Search {
    fn get_search_term(self) -> String {
        match self {
            Search::Id(i) => format!("{} = {}", "id", i),
            Search::uuid(s) => format!("{} = '{}'", "uuid", s),
        }
    }

    pub async fn find(self, db_pool: &DBPool) -> Result<Option<File>, Error> {
        let mut s = search_database(db_pool, self).await?;
        if s.is_empty() {
            return Ok(None);
        }
        Ok(Some(s.remove(0)))
    }
}

async fn search_database<'a>(db_pool: &DBPool, search: Search) -> Result<Vec<File>, Error> {
    let conn = get_db_con(db_pool).await?;

    let rows = conn
        .query(
            format!(
                "
                SELECT * from shares
                WHERE {}
                ORDER BY created_at DESC
            ",
                search.get_search_term()
            )
            .as_str(),
            &[],
        )
        .await
        .map_err(Error::DBQuery)?;

    rows.iter().map(|r| File::from_database(r)).collect()
}
