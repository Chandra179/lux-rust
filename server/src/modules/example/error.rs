//! Domain errors of the example module. HTTP mapping lives in
//! `crate::error`.

#[derive(Debug, thiserror::Error)]
pub enum ExampleError {
    #[error("create example: name {0:?} is reserved")]
    ReservedName(String),
    #[error("create example")]
    Store(#[from] sqlx::Error),
}
