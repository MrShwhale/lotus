use regex::Regex;
use reqwest::blocking;
use scraper::{Html, Selector};
use std::{
    cell::UnsafeCell,
    collections::HashSet,
    fmt::{Debug, Display},
    mem,
    sync::{
        mpsc::{self, Receiver, RecvError, SendError, Sender},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

const TAG_PREFIX: &str = "https://scp-wiki.wikidot.com/system:page-tags/tag/";
// const TAG_TYPES: [&str; 4] = ["scp", "tale", "hub", "goi-format"];
const TAG_TYPES: [&str; 1] = ["hub"];

const WIKI_PREFIX: &str = "https://scp-wiki.net/";

type ID = u64;

enum ThreadResponse {
    VoteRequest(usize),
    ArticleRequest(usize),
    EndRequest,
    Alright,
}

/// Holds information about the errors which can happen while web scraping
/// CONS make this more robust and specific with context-based information
pub enum ScrapeError {
    RegexError,
    WebError,
    RuntimeError,
    MessagingError,
    ThreadError,
}

impl From<reqwest::Error> for ScrapeError {
    fn from(_: reqwest::Error) -> Self {
        ScrapeError::WebError
    }
}

impl From<std::io::Error> for ScrapeError {
    fn from(_: std::io::Error) -> Self {
        ScrapeError::RuntimeError
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

impl Display for ScrapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self)
    }
}

impl Debug for ScrapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            ScrapeError::RegexError => "There was an error in the regex.",
            ScrapeError::WebError => "There was an error in a web request.",
            ScrapeError::RuntimeError => "There was an error in setting up the runtime.",
            ScrapeError::MessagingError => {
                "There was an error in sending a message between threads."
            }
            ScrapeError::ThreadError => "There was an error in one of the threads.",
        };

        write!(f, "{}", message)
    }
}

/// Holds basic information about an article on the wiki
#[derive(Debug)]
struct Article {
    /// The name of the article, user-facing
    name: String,
    /// The url suffix at which the page can be found
    url: String,
    /// The internal page id
    page_id: u64,
    /// The indices of this article's tags in the taglist
    tags: Vec<u16>,
    /// The votes (by user)
    votes: Vec<(i8, ID)>,
}

/// Holds basic information about a user on the wiki
#[derive(Debug)]
struct User {
    /// The name of the user, user-facing
    name: String,
    /// The url suffix at which the user's page can be found
    url: String,
    /// The internal user id
    user_id: ID,
}

/// Holds all information that is recorded during a scrape
pub struct ScrapeInfo {
    articles: Vec<Article>,
    users: HashSet<User>,
    tags: Vec<String>,
}

/// Used to scrape the SCP wiki for votes, tags, and users,
/// and stores that data
pub struct Scraper {
    /// Settings to use for this Scraper
    settings: ScraperSettings,
    /// If the scraper is currently scraping
    running: bool,
    /// Number of requests which have been sent
    num_requests_sent: u32,
}

pub struct ScraperSettings {
    /// String to use when accessing the SCP api
    api_token: String,
    /// Level of detail to log
    log_level: u8,
    /// How to display logs (bitmap)
    log_method: u8,
    /// Maximum number of requests to send. Negative to disable.
    max_requests: i32,
    /// Maximum number of requests to send each second
    requests_per_second: u64,
    /// Maximum number of additional system threads to create
    max_threads: usize,
}

impl Scraper {
    pub fn new() -> Scraper {
        Scraper {
            // api token found in https://github.com/scp-data/scp_crawler
            // It is not a real api token, just meant to placehold
            settings: ScraperSettings {
                api_token: String::from("123456"),
                log_level: 1,
                log_method: 3,
                max_requests: -1,
                requests_per_second: 8,
                max_threads: 32,
            },
            running: false,
            num_requests_sent: 0,
        }
    }

    /// Scrapes the full SCP wiki and records the information in a format
    /// which the rest of this program can use.
    pub fn scrape(&mut self) -> Result<ScrapeInfo, ScrapeError> {
        // Requests are only saved per-scrape
        self.num_requests_sent = 0;

        // Get the list of articles to be scraped on the wiki
        let scrape_list = self.add_all_pages()?;

        // Actually scrape each article
        let scraped_info = self.scrape_pages(scrape_list)?;

        Ok(scraped_info)
    }

    /// Scrapes the links to the given articles, overwriting existing data in each element
    fn scrape_pages(&self, scrape_list: Vec<Article>) -> Result<ScrapeInfo, ScrapeError> {
        // Spawn some number of threads
        // This will cap the number of concurrent web requests sent out at once
        // Each thread will get a mutable reference to the article it needs to fill in
        // Once finished, it will send a message to the main thread, which will pass it a new
        // article to review

        let mut scraped_info = ScrapeInfo {
            articles: scrape_list,
            users: HashSet::new(),
            tags: Vec::new(),
        };

        let mut next_article = 0;
        let num_articles = scraped_info.articles.len();
        let num_threads = self.settings.max_threads;
        let wait_time = Duration::from_millis(1000 / self.settings.requests_per_second);

        // Create the message passing situation
        let (main_tx, main_rx): (Sender<ThreadResponse>, Receiver<ThreadResponse>) =
            mpsc::channel();
        let (thread_txs, thread_rxs): (Vec<_>, Vec<_>) = (0..num_threads)
            .map(|_| {
                let thread_channel: (Sender<ThreadResponse>, Receiver<ThreadResponse>) =
                    mpsc::channel();
                thread_channel
            })
            .unzip();

        // Create an unsafe reference here
        let articles_arc = Arc::new(&scraped_info.articles);

        // Create the threads
        thread::scope(|scope| {
            thread_rxs.into_iter().enumerate().for_each(|(id, thread_rx)| {
                let articles_copy = articles_arc.clone();
                let main_tx = main_tx.clone();
                scope.spawn(move || {
                    // CONS not doing this with blocking since there is probably a better way asyncronously
                    // but we all know how that went
                    let client = blocking::Client::new();
                    loop {
                        // Tell main which thread needs an article
                        main_tx
                            .send(ThreadResponse::ArticleRequest(id))
                            .expect("The reciever should never be deallocated.");

                        // Wait for a response
                        let article_index = match thread_rx
                            .recv()
                            .expect("The Sender should never be disconnected.")
                        {
                            ThreadResponse::ArticleRequest(article_id) => article_id,
                            ThreadResponse::EndRequest => {
                                main_tx
                                    .send(ThreadResponse::Alright)
                                    .expect("The reciever should never be deallocated.");
                                break;
                            }
                            // Main will only send the above responses
                            _ => unreachable!(),
                        };

                        // Gets a reference to the url string of the selected article
                        let url: &String = unsafe {
                            let unsafe_articles = articles_copy
                                .get(article_index)
                                .expect("This should never be OOB");
                            let raw_article = unsafe_articles as *const Article;
                            &(*raw_article).url
                        };

                        // Make the article request
                        // TODO add limited retry support

                        println!("THREAD {}: URL TO SCRAPE: {:?}", id, url);
                        // Parse the article
                        // Update the article
                        // Make the vote request
                        // Update the article
                    }
                });
            });

            // TODO add error checking below this point
            while next_article < num_articles {
                // Wait for a perm request
                let response = main_rx.recv().unwrap();
                match response {
                    ThreadResponse::VoteRequest(id) => thread_txs
                        .get(id)
                        .expect("ID should never be OOB.")
                        .send(ThreadResponse::Alright)
                        .unwrap(),
                    ThreadResponse::ArticleRequest(id) => thread_txs
                        .get(id)
                        .expect("ID should never be OOB.")
                        .send(ThreadResponse::ArticleRequest(next_article))
                        .unwrap(),
                    // Threads can only send the above requests
                    _ => unreachable!(),
                };

                next_article += 1;

                // Wait until the next web request can be sent according to settings
                thread::sleep(wait_time);
            }

            // Ask threads to stop
            for thread_tx in thread_txs.iter() {
                thread_tx.send(ThreadResponse::EndRequest).unwrap();
            }

            // Wait for all to stop
            let mut num_alive = num_threads;
            while num_alive > 0 {
                main_rx.recv().unwrap();
                num_alive -= 1;
            }
        });

        Ok(scraped_info)
    }

    /// Adds all pages on the wiki to the list of pages to scrape.
    /// Pages are determined to be "on" the wiki if they have one of the major tag types (things
    /// like "tale" or "scp"). Since articles must (I think) have exactly one of these, it is
    /// reasonable to use this to discover pages.
    /// # Return
    /// The articles in the returned vector are only the names and links to the articles
    /// and do not have id, vote, or tag information.
    fn add_all_pages(&mut self) -> Result<Vec<Article>, ScrapeError> {
        let mut tag_url;
        let mut articles = Vec::new();
        let client = blocking::Client::new();
        for tag in TAG_TYPES.iter() {
            tag_url = String::from(TAG_PREFIX);
            tag_url.push_str(tag);
            let mut pages = self.extract_links_from_syspage(&client, &tag_url)?;
            articles.append(&mut pages);
        }

        Ok(articles)
    }

    /// Adds all articles on a system page to a Vec then returns it.
    /// Does not add directly to the Vec to make multithreading easy.
    /// This is blocking since this should be run before starting the real
    /// scraper and should have a very limited number of requests.
    fn extract_links_from_syspage(
        &mut self,
        client: &blocking::Client,
        url: &String,
    ) -> Result<Vec<Article>, ScrapeError> {
        self.num_requests_sent += 1;
        let response = client.get(url).send()?;
        let document = Html::parse_document(response.text()?.as_str());
        let page_item = Selector::parse(r#"div[class="pages-list-item"]"#)
            .expect("hardcoded selector, shouldn't fail");
        let page_elements = document.select(&page_item);

        let name_pattern =
            Regex::new(r#"<a href="(.+)">(.+)+</a>"#).expect("hardcoded regex, shouldn't fail");

        let mut pages = Vec::new();
        for page in page_elements {
            let element_html = page.inner_html();

            // TODO this looks really not great, could be some way to fix it
            let captures = match name_pattern.captures(element_html.as_str()) {
                Some(cap) => cap,
                None => return Err(ScrapeError::RegexError),
            };

            let url = match captures.get(1) {
                Some(url) => String::from(
                    url.as_str()
                        .get(1..)
                        .expect("Valid links should always have more than 1 char."),
                ),
                None => return Err(ScrapeError::RegexError),
            };

            let name = match captures.get(2) {
                Some(name) => String::from(name.as_str()),
                None => return Err(ScrapeError::RegexError),
            };

            pages.push(Article {
                name,
                url,
                page_id: 0,
                tags: Vec::new(),
                votes: Vec::new(),
            });
        }

        Ok(pages)
    }

    /// Checks how many pages would have to be scraped in a real scrape.
    /// Still sends as many requests as there are major tag types.
    pub fn dry_scrape(&mut self) -> Result<(), ScrapeError> {
        let articles = self.add_all_pages()?;

        println!(
            "Successfully came up with {} pages to scrape.",
            articles.len()
        );

        Ok(())
    }

    pub fn schedule_scrape(&mut self) -> Result<(), ScrapeError> {
        Ok(())
    }
}
