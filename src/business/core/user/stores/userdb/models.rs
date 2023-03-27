use serde::Serialize;

#[derive(sqlx::FromRow, Serialize)]
pub struct User {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub role: String,
}
