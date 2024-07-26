use const_format::formatcp;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

// Output location constants
pub const OUTPUT_DIR: &str = "./output";

pub const ARTICLE_OUTPUT: &str = formatcp!("{}/articles.parquet", OUTPUT_DIR);

pub const TAGS_OUTPUT: &str = formatcp!("{}/tags.parquet", OUTPUT_DIR);

pub const USERS_OUTPUT: &str = formatcp!("{}/users.parquet", OUTPUT_DIR);

pub const VOTES_OUTPUT: &str = formatcp!("{}/votes.parquet", OUTPUT_DIR);

// The types used to represent different things on the wiki
// CONS typedef for u64 as ID here

// Holds basic information about an article on the wiki
// CONS changing public fields?
#[derive(Debug, Hash, Serialize, Deserialize)]
pub struct Article {
    /// The name of the article, user-facing
    pub name: String,
    /// The internal page id
    pub page_id: u64,
    /// The indices of this article's tags in the taglist
    pub tags: Vec<u16>,
    /// The url suffix at which the page can be found
    pub url: String,
    /// The vote results and the userids of those who gave them
    pub votes: Vec<(i8, u64)>,
}

/// Holds basic information about a user on the wiki
/// CONS changing public fields?
#[derive(Debug, Hash, Serialize, Deserialize)]
pub struct User {
    /// The name of the user, user-facing
    pub name: String,
    /// The url suffix at which the user's page can be found
    pub url: String,
    /// The internal user id
    pub user_id: u64,
}

impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        self.user_id == other.user_id
    }
}

impl Eq for User {}
