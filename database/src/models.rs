use super::schema::*;
use diesel::Insertable;

#[derive(Queryable, Insertable)]
#[table_name = "shares"]
pub struct Share {
    pub file_id: i32,
    pub exp: i64,
    pub crt: i64,
    pub file_size: i64,
    pub user_name: String,
    pub file_name: String,
}
