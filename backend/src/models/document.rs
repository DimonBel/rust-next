use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::documents;

#[derive(Queryable, Serialize, Deserialize, Clone)]
pub struct Document {
    pub id: Option<i32>,
    pub filename: String,
    pub path: String,
    pub summary: Option<String>,
    pub keywords: Option<String>,
    pub entities: Option<String>,
    pub topics: Option<String>,
    pub uploaded_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[diesel(table_name = documents)]
pub struct NewDocument<'a> {
    pub filename: &'a str,
    pub path: &'a str,
    pub summary: Option<&'a str>,
    pub keywords: Option<&'a str>,
    pub entities: Option<&'a str>,
    pub topics: Option<&'a str>,
}
