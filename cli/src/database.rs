//! Contains all database functionality

use chrono::prelude::*;
use mobc::{Connection, Pool};
use mobc_postgres::{tokio_postgres, PgConnectionManager};
use std::fs;
use std::str::FromStr;
use std::time::Duration;
use tokio_postgres::{Config, NoTls, Row};
use crate::error::Error;
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
        let id: String = data.try_get::<usize, String>(1).unwrap();
        let crt: DateTime<Utc> = data.try_get::<usize, DateTime<Utc>>(2).unwrap(); //Test timestamp
        let exp: DateTime<Utc> = data.try_get::<usize, DateTime<Utc>>(3).unwrap(); //Test timestamp
        let user: String = data.try_get::<usize, String>(4).unwrap();
        let website: bool = data.try_get::<usize, bool>(5).unwrap();
        let wget: bool = data.try_get::<usize, bool>(6).unwrap();
        let name: String = data.try_get::<usize, String>(7).unwrap();
        let size: usize = data.try_get::<usize, i64>(8).unwrap() as usize;
        let ext: String = data.try_get::<usize, String>(9).unwrap();

        Ok(File {
            id,
            crt,
            exp,
            user,
            website,
            wget,
            name,
            size,
            ext,
        })
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

pub enum Search {
    #[allow(dead_code)]
    Id(usize),
    PublicId(String),
}

impl Search {
    fn get_search_term(self) -> String {
        match self {
            Search::Id(i) => format!("{} = {}", "id", i),
            Search::PublicId(s) => format!("{} = '{}'", "public_id", s),
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

pub async fn add_share(db_pool: &DBPool, body: File) -> Result<File, Error> {
    let conn = get_db_con(db_pool).await?;
    let row = conn
        .query_one(
            "
            INSERT INTO shares (public_id, expires, usr, website, wget, name, size, file_type)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *;
        ",
            &[&body.id, &body.exp, &body.user, &body.website, &body.wget, &body.name, &(body.size as i64), &body.ext],
        )
        .await
        .map_err(Error::DBQuery)?;

    File::from_database(&row)
}


pub async fn delete_share(db_pool: &DBPool, id: &usize) -> Result<u64, Error> {
    let conn = get_db_con(db_pool).await?;
    conn.execute(
        "
            DELETE FROM shares
            WHERE public_id = $1
        ",
        &[&(*id as i64)],
    )
    .await
    .map_err(Error::DBQuery)
}
