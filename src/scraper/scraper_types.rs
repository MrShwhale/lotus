use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::mpsc::{RecvError, SendError};

pub enum ThreadResponse {
    ArticleRequest(usize),
    EndRequest,
    Alright,
    UserInfo(User),
}

// Holds information about the errors which can happen while web scraping
// CONS make this more robust and specific with context-based information
pub enum ScrapeError {
    RegexError,
    WebError(reqwest::Error),
    WritingError,
    MessagingError,
    // ThreadError,
}

impl From<reqwest::Error> for ScrapeError {
    fn from(err: reqwest::Error) -> Self {
        ScrapeError::WebError(err)
    }
}

impl From<std::io::Error> for ScrapeError {
    fn from(_: std::io::Error) -> Self {
        ScrapeError::WritingError
    }
}

impl From<RecvError> for ScrapeError {
    fn from(_: RecvError) -> Self {
        ScrapeError::MessagingError
    }
}

impl From<SendError<ThreadResponse>> for ScrapeError {
    fn from(_: SendError<ThreadResponse>) -> Self {
        ScrapeError::MessagingError
    }
}

impl Debug for ScrapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            ScrapeError::RegexError => String::from("There was an error in the regex."),
            ScrapeError::WebError(err) => format!("{:?}", err),
            ScrapeError::WritingError => String::from("There was an error in writing to a file."),
            ScrapeError::MessagingError => {
                String::from("There was an error in sending a message between threads.")
            } //ScrapeError::ThreadError => String::from("There was an error in one of the threads."),
        };

        write!(f, "{}", message)
    }
}

/// Holds basic information about an article on the wiki
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
    /// The votes' values paired with the user id that cast the vote
    pub votes: Vec<(i8, u64)>,
}

/// Holds basic information about a user on the wiki
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
