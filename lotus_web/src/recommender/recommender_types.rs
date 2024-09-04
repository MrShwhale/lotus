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

#[derive(Clone, Debug)]
pub struct RecommenderOptions {
    /// Minimum number of votes to consider a users opinion
    /// Lowering this increases the amount of memory and time needed to get recs
    /// It also clutters similarity weights with users who have read few articles
    /// However, users with less than this many votes will not be able to get recs
    min_votes: u16,
    /// Number of similar users to consider when recommending an article
    /// Setting this higher gives a wider variety of opinions, but makes
    /// suggestions more susceptible to popularity bias
    users_to_consider: u32,
    /// Locations of the output files
    outputs: OutputFiles,
}

impl RecommenderOptions {
    /// Create an options instance with the default options
    pub fn new() -> RecommenderOptions {
        RecommenderOptions {
            min_votes: 10,
            users_to_consider: 30,
            outputs: OutputFiles::new(),
        }
    }

    pub fn with_articles_file(mut self, new_articles_file: String) -> RecommenderOptions {
        self.outputs.article_output = new_articles_file;
        self
    }

    pub fn get_articles_file(&self) -> &String {
        &self.outputs.article_output
    }

    pub fn with_min_votes(mut self, new_min_votes: u16) -> RecommenderOptions {
        self.min_votes = new_min_votes;
        self
    }

    pub fn get_min_votes(&self) -> u16 {
        self.min_votes
    }

    pub fn with_tags_file(mut self, new_tags_file: String) -> RecommenderOptions {
        self.outputs.tags_output = new_tags_file;
        self
    }

    pub fn get_tags_file(&self) -> &String {
        &self.outputs.tags_output
    }

    pub fn with_users_to_consider(mut self, new_users_to_consider: u32) -> RecommenderOptions {
        self.users_to_consider = new_users_to_consider;
        self
    }

    pub fn get_users_to_consider(&self) -> u32 {
        self.users_to_consider
    }

    pub fn with_users_file(mut self, new_users_file: String) -> RecommenderOptions {
        self.outputs.users_output = new_users_file;
        self
    }

    pub fn get_users_file(&self) -> &String {
        &self.outputs.users_output
    }

    pub fn with_votes_file(mut self, new_votes_file: String) -> RecommenderOptions {
        self.outputs.votes_output = new_votes_file;
        self
    }

    pub fn get_votes_file(&self) -> &String {
        &self.outputs.votes_output
    }
}

impl Default for RecommenderOptions {
    fn default() -> Self {
        Self::new()
    }
}
