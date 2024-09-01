use lotus::{ARTICLE_OUTPUT, TAGS_OUTPUT, USERS_OUTPUT, VOTES_OUTPUT};
use polars::prelude::*;
use std::{fmt::Debug, io};

pub enum RecommenderError {
    Polars(PolarsError),
    File(io::Error),
    Bounds,
}

impl Debug for RecommenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Polars(err) => format!("Polars: {:?}", err),
            Self::File(err) => format!("File: {:?}", err),
            Self::Bounds => String::from("OOB somewhere"),
        };

        write!(f, "{}", message)
    }
}

impl From<PolarsError> for RecommenderError {
    fn from(value: PolarsError) -> Self {
        RecommenderError::Polars(value)
    }
}

impl From<io::Error> for RecommenderError {
    fn from(value: io::Error) -> Self {
        RecommenderError::File(value)
    }
}

// CONS choose between multiple floating point types
// CONS nonstatic lifetime
// CONS properly encapsulate with getters
#[derive(Clone)]
pub struct RecommenderOptions {
    /// Location of the parquet file that contains article information
    pub articles_file: &'static str,
    /// Minimum number of votes to consider a users opinion
    /// Lowering this increases the amount of memory and time needed to get recs
    /// It also clutters similarity weights with users who have read few articles
    /// However, users with less than this many votes will not be able to get recs
    pub min_votes: u16,
    /// Location of the parquet file that contains tag information
    pub tags_file: &'static str,
    /// Number of similar users to consider when recommending an article
    /// Setting this higher gives a wider variety of opinions, but makes
    /// suggestions more susceptible to popularity bias
    pub users_to_consider: u32,
    /// Location of the parquet file that contains user information
    pub users_file: &'static str,
    /// Location of the parquet file that contains vote information
    pub votes_file: &'static str,
}

impl RecommenderOptions {
    /// Create an options instance with the default options
    pub const fn new() -> RecommenderOptions {
        RecommenderOptions {
            articles_file: ARTICLE_OUTPUT,
            min_votes: 10,
            tags_file: TAGS_OUTPUT,
            users_to_consider: 30,
            users_file: USERS_OUTPUT,
            votes_file: VOTES_OUTPUT,
        }
    }

    pub fn with_articles_file(mut self, new_articles_file: &'static str) -> RecommenderOptions {
        self.articles_file = new_articles_file;
        self
    }

    pub fn with_min_votes(mut self, new_min_votes: u16) -> RecommenderOptions {
        self.min_votes = new_min_votes;
        self
    }

    pub fn with_tags_file(mut self, new_tags_file: &'static str) -> RecommenderOptions {
        self.tags_file = new_tags_file;
        self
    }

    pub fn with_users_to_consider(mut self, new_users_to_consider: u32) -> RecommenderOptions {
        self.users_to_consider = new_users_to_consider;
        self
    }

    pub fn with_users_file(mut self, new_users_file: &'static str) -> RecommenderOptions {
        self.users_file = new_users_file;
        self
    }

    pub fn with_votes_file(mut self, new_votes_file: &'static str) -> RecommenderOptions {
        self.votes_file = new_votes_file;
        self
    }
}

impl Default for RecommenderOptions {
    fn default() -> Self {
        Self::new()
    }
}
