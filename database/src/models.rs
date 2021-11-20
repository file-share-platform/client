use super::schema::*;
use chrono::NaiveDateTime;
use diesel::Insertable;
use ws_com_framework::File;

#[derive(Queryable, Insertable)]
#[table_name = "shares"]
pub struct Share {
    pub id: Vec<u8>,
    pub user: String,
    pub exp: NaiveDateTime,
    pub crt: NaiveDateTime,
    pub name: String,
    pub size: i32,
    pub ext: String,
}

impl From<&File> for Share {
    fn from(f: &File) -> Self {
        Share {
            id: f.id.to_vec(),
            user: f.user.clone(),
            exp: f.exp.naive_utc(),
            crt: f.crt.naive_utc(),
            name: f.name.clone(),
            size: f.size as i32,
            ext: f.ext.clone(),
        }
    }
}

impl From<Share> for File {
    fn from(_: Share) -> Self {
        todo!()
    }
}
