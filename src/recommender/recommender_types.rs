use polars::prelude::*;
use std::{fmt::Debug, io};
use const_format::formatcp;

// Output location constants
// TODO make these not copy/pasted from the other file
pub const OUTPUT_DIR: &str = "./output";

pub const ARTICLE_OUTPUT: &str = formatcp!("{}/articles.parquet", OUTPUT_DIR);
pub const TAGS_OUTPUT: &str = formatcp!("{}/tags.parquet", OUTPUT_DIR);
pub const USERS_OUTPUT: &str = formatcp!("{}/users.parquet", OUTPUT_DIR);
pub const VOTES_OUTPUT: &str = formatcp!("{}/votes.parquet", OUTPUT_DIR);

pub enum RecommenderError {
    PolarsError(PolarsError),
    FileError(io::Error),
    OOBError,
}

impl Debug for RecommenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::PolarsError(err) => format!("Polars: {:?}", err),
            Self::FileError(err) => format!("File: {:?}", err),
            Self::OOBError => String::from("OOB somewhere"),
        };

        write!(f, "{}", message)
    }
}

impl From<PolarsError> for RecommenderError {
    fn from(value: PolarsError) -> Self {
        RecommenderError::PolarsError(value)
    }
}

impl From<io::Error> for RecommenderError {
    fn from(value: io::Error) -> Self {
        RecommenderError::FileError(value)
    }
}

// CONS choose between multiple floating point types?
// CONS nonstatic lifetime
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

pub static DEFAULT_REC_OPTIONS: RecommenderOptions = RecommenderOptions {
    articles_file: ARTICLE_OUTPUT,
    min_votes: 10,
    tags_file: TAGS_OUTPUT,
    users_to_consider: 30,
    users_file: USERS_OUTPUT,
    votes_file: VOTES_OUTPUT,
};
