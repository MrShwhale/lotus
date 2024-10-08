use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::mpsc::{RecvError, SendError};

#[derive(Clone)]
pub struct RawPointerWrapper {
    pub raw: *mut Article,
}

/// Thanks to <https://cryptical.xyz/rust/unsafe> for this idea/code.
/// I worship at your altar, Özgün Özerk.
impl RawPointerWrapper {
    /// Get a mutable reference to the pointer
    /// # Safety
    /// The caller must be sure that this is not being referenced by multiple things, especially
    /// across threads
    pub unsafe fn get_mut_ptr(&self) -> &mut Article {
        &mut *self.raw
    }
}

unsafe impl Send for RawPointerWrapper {}

pub enum ThreadResponse {
    /// A message to a thread for an article to be scraped
    ArticleResponse(RawPointerWrapper),
    /// A message from a thread that it is ready for an article
    ArticleRequest(usize),
    /// Request to a thread to stop scraping things
    EndRequest,
    /// Response to EndRequest
    Alright,
    /// Request from thread to add a User to the set of users
    UserInfo(User),
}

/// Holds information about the errors which can happen while web scraping
pub enum ScrapeError {
    RegexError,
    WebError(reqwest::Error),
    WritingError,
    MessagingError,
    ThreadError,
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
            }
            ScrapeError::ThreadError => String::from("There was an error in one of the threads."),
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

unsafe impl Send for Article {}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn total_inequality() {
        let user_1 = User {
            name: String::from("Shark Lover"),
            url: String::from("shark-lover"),
            user_id: 1,
        };

        let user_2 = User {
            name: String::from("Whale Lover"),
            url: String::from("shark-lover"),
            user_id: 2,
        };

        assert!(user_1 == user_2);
    }

    #[test]
    fn total_equality() {
        let user_1 = User {
            name: String::from("Shark Lover"),
            url: String::from("shark-lover"),
            user_id: 1,
        };

        let user_2 = User {
            name: String::from("Shark Lover"),
            url: String::from("shark-lover"),
            user_id: 1,
        };

        assert!(user_1 == user_2);
    }

    #[test]
    #[should_panic]
    fn partial_inequality() {
        let user_1 = User {
            name: String::from("Shark Lover"),
            url: String::from("shark-lover"),
            user_id: 1,
        };

        let user_2 = User {
            name: String::from("Shark Lover"),
            url: String::from("shark-lover"),
            user_id: 2,
        };

        assert!(user_1 == user_2);
    }

    #[test]
    fn partial_equality() {
        let user_1 = User {
            name: String::from("Shark Lover"),
            url: String::from("shark-lover"),
            user_id: 1,
        };

        let user_2 = User {
            name: String::from("Whale Lover"),
            url: String::from("shark-lover"),
            user_id: 1,
        };

        assert!(user_1 == user_2);
    }
}
