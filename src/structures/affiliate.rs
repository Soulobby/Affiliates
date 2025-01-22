#[derive(sqlx::FromRow, Clone, Eq, PartialEq, Hash)]
pub struct Affiliate {
    pub user_id: i64,
}
