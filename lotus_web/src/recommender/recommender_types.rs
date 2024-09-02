use lotus::OutputFiles;
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
    /// Minimum number of votes to consider a users opinion
    /// Lowering this increases the amount of memory and time needed to get recs
    /// It also clutters similarity weights with users who have read few articles
    /// However, users with less than this many votes will not be able to get recs
    pub min_votes: u16,
    /// Number of similar users to consider when recommending an article
    /// Setting this higher gives a wider variety of opinions, but makes
    /// suggestions more susceptible to popularity bias
    pub users_to_consider: u32,
    /// Locations of the output files
    pub outputs: OutputFiles,
}

impl RecommenderOptions {
    /// Create an options instance with the default options
    pub const fn new() -> RecommenderOptions {
        RecommenderOptions {
            min_votes: 10,
            users_to_consider: 30,
            outputs: OutputFiles::new(),
        }
    }

    pub fn with_articles_file(mut self, new_articles_file: &'static str) -> RecommenderOptions {
        self.outputs.article_output = new_articles_file;
        self
    }

    pub fn with_min_votes(mut self, new_min_votes: u16) -> RecommenderOptions {
        self.min_votes = new_min_votes;
        self
    }

    pub fn with_tags_file(mut self, new_tags_file: &'static str) -> RecommenderOptions {
        self.outputs.tags_output = new_tags_file;
        self
    }

    pub fn with_users_to_consider(mut self, new_users_to_consider: u32) -> RecommenderOptions {
        self.users_to_consider = new_users_to_consider;
        self
    }

    pub fn with_users_file(mut self, new_users_file: &'static str) -> RecommenderOptions {
        self.outputs.users_output = new_users_file;
        self
    }

    pub fn with_votes_file(mut self, new_votes_file: &'static str) -> RecommenderOptions {
        self.outputs.votes_output = new_votes_file;
        self
    }
}

impl Default for RecommenderOptions {
    fn default() -> Self {
        Self::new()
    }
}
