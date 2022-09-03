//! Models for the database schema

#![allow(unused_qualifications)]

use super::schema::*;
use diesel::Insertable;

/// A unique share representing a file

#[derive(Queryable, Insertable, Debug)]
#[diesel(table_name = shares)]
pub struct Share {
    /// Unique ID identifying the file
    pub file_id: i64,
    /// The time that the file will expire
    pub exp: i64,
    /// The time that the share was created
    pub crt: i64,
    /// The size of the file
    pub file_size: i64,
    /// The user who shared the file
    pub user_name: String,
    /// The name of the file
    pub file_name: String,
}
