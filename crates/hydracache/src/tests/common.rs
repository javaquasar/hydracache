use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct User {
    pub(crate) id: u64,
    pub(crate) name: String,
}

#[derive(Debug)]
pub(crate) struct LoaderError;

impl fmt::Display for LoaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("loader failed")
    }
}

impl Error for LoaderError {}

pub(crate) fn user(id: u64) -> User {
    User {
        id,
        name: format!("user-{id}"),
    }
}
