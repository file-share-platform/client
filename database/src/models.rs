use chrono::NaiveDateTime;
use diesel::Insertable;
use super::schema::*;
use ws_com_framework::File;

#[derive(Queryable, Insertable)]
#[table_name = "shares"]
pub struct Share {
    pub id: String,
    pub user: String,
    pub exp: NaiveDateTime,
    pub crt: NaiveDateTime,
    pub name: String,
    pub size: i32,
    pub ext: String,
}

//XXX this type conversion is temporary, until ws-com-framework can be updated
impl From<&File> for Share {
    fn from(f: &File) -> Self {
        Share {
            id: f.id.clone(),
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