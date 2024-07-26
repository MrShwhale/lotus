use crate::lotus_core::User;
use std::fmt::Debug;
use std::sync::mpsc::{RecvError, SendError};

pub enum ThreadResponse {
    ArticleRequest(usize),
    EndRequest,
    Alright,
    UserInfo(User),
}

/// Holds information about the errors which can happen while web scraping
/// CONS make this more robust and specific with context-based information
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
