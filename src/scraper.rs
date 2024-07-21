use regex::Regex;
use reqwest::blocking;
use scraper::{Html, Selector};
use std::{
    collections::HashSet,
    fmt::{Debug, Display},
    sync::{
        mpsc::{self, Receiver, RecvError, SendError, Sender},
        Arc,
    },
    thread,
    time::Duration,
};

use parking_lot::{Mutex, RwLock};

const TAG_PREFIX: &str = "https://scp-wiki.wikidot.com/system:page-tags/tag/";
// const TAG_TYPES: [&str; 4] = ["scp", "tale", "hub", "goi-format"];
const TAG_TYPES: [&str; 1] = ["hub"];

const WIKI_PREFIX: &str = "https://scp-wiki.net/";

// api token found in https://github.com/scp-data/scp_crawler
// It is not a real api token, just meant to placehold
const API_TOKEN: &str = "123456";

type ID = u64;

enum ThreadResponse {
    VoteRequest(usize),
    ArticleRequest(usize),
    EndRequest,
    Alright,
    UserInfo(User),
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
pub struct Article {
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
#[derive(Debug, Hash)]
pub struct User {
    /// The name of the user, user-facing
    name: String,
    /// The url suffix at which the user's page can be found
    url: String,
    /// The internal user id
    user_id: ID,
}

impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        self.user_id == other.user_id
    }
}

impl Eq for User {}

/// Holds all information that is recorded during a scrape
pub struct ScrapeInfo {
    articles: Vec<Article>,
    users: HashSet<User>,
    // CONS replacing with something faster
    tags: Vec<String>,
}

/// Used to scrape the SCP wiki for votes, tags, and users,
/// and stores that data
pub struct Scraper {
    /// Settings to use for this Scraper
    settings: ScraperSettings,
    /// If the scraper is currently scraping
    running: bool,
}

pub struct ScraperSettings {
    /// Level of detail to log
    log_level: u8,
    /// How to display logs (bitmap)
    log_method: u8,
    /// Maximum number of requests to send. Negative to disable.
    max_requests: i32,
    /// Maximum number of requests to send each second
    requests_per_second: f64,
    /// Maximum number of additional system threads to create
    max_threads: usize,
}

impl Scraper {
    pub fn new() -> Scraper {
        Scraper {
            settings: ScraperSettings {
                log_level: 1,
                log_method: 3,
                max_requests: -1,
                requests_per_second: 0.125,
                max_threads: 32,
            },
            running: false,
        }
    }

    /// Scrapes the full SCP wiki and records the information in a format
    /// which the rest of this program can use.
    pub fn scrape(&mut self) -> Result<ScrapeInfo, ScrapeError> {
        // Get the list of articles to be scraped on the wiki
        println!("Getting all the list of pages...");
        let scrape_list = self.add_all_pages()?;

        // Scrape the tags
        println!("Getting the list of tags...");
        let tag_group = self.add_all_tags()?;

        // Actually scrape each article
        println!("Scraping the pages...");
        let scraped_info = self.scrape_pages(scrape_list, tag_group)?;

        Ok(scraped_info)
    }

    /// Scrapes the links to the given articles, overwriting existing data in each element
    fn scrape_pages(
        &self,
        mut articles: Vec<Mutex<Article>>,
        mut tags: Vec<String>,
    ) -> Result<ScrapeInfo, ScrapeError> {
        let mut next_article = 0;
        let num_articles = articles.len();
        let num_threads = self.settings.max_threads;
        let wait_time = Duration::from_millis((1000.0 / self.settings.requests_per_second) as u64);

        let mut users = HashSet::new();

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

        // Create a shared reference here
        // Terrible things will happen to it later
        let articles_arc = Arc::new(&mut articles);
        let tags_arc = Arc::new(&mut tags);

        // Create the threads
        println!("Creating the threads...");
        thread::scope(|scope| {
            thread_rxs
                .into_iter()
                .enumerate()
                .for_each(|(id, thread_rx)| {
                    let articles_copy = articles_arc.clone();
                    let tags_copy = tags_arc.clone();
                    let main_tx = main_tx.clone();
                    scope.spawn(move || {
                        let client = blocking::Client::new();
                        let page_id_pattern =
                        Regex::new(r#"WIKIREQUEST.info.siteId = (/d+);"#).expect("hardcoded regex, shouldn't fail");
                        let user_pattern = Regex::new(r#"userInfo\((\d+)\); return false;\\\"  ><img class=\\\"small\\\" src=\\\"https:\\\/\\\/www\.wikidot\.com\\\/avatar\.php\?userid=(?:\d+)&amp;amp;size=small&amp;amp;timestamp=(?:\d+)\\\" alt=\\\"(?:[^\\]+)\\\" style=\\\"background-image:url\(https:\\\/\\\/www\.wikidot\.com\\\/userkarma\.php\?u=(?:\d+)\)\\\"\\\/><\\\/a><a href=\\\"http:\\\/\\\/www\.wikidot\.com\\\/user:info\\\/([^\\]+)\\\" onclick=\\\"WIKIDOT\.page\.listeners\.userInfo\((?:\d+)\); return false;\\\" >([^<]+)<\\/a><\\/span>\\n        <span style=\\"color:#777\\">\\n(?: +)(.)"#).unwrap();
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

                            // Gets a reference to the selected article
                            let mut article = articles_copy
                                .get(article_index)
                                .expect("This should never be OOB.")
                                .lock();
                            let url = &article.url;

                            // Make the article request
                            // TODO add limited retry support
                            // TODO error checking
                            // println!("THREAD {}: {}", id, url);
                            println!("url sent: {}", url);
                            let document_text = client
                                .get(String::from(WIKI_PREFIX) + url.as_str())
                                .send()
                                .unwrap()
                                .text()
                                .unwrap();
                            println!("url_response: {}", url);

                            // Get the required values
                            let page_id_captures = page_id_pattern.captures(document_text.as_str()).unwrap();
                            let page_id = page_id_captures.get(1).unwrap().as_str();
                            let page_id: u64 = page_id.parse().unwrap();

                            let document = Html::parse_document(document_text.as_str());
                            let selector =
                            Selector::parse(r#"div.page-tags a"#).expect("Hardcoded selector should not fail.");
                            let tags: Vec<_> = document
                                .select(&selector)
                                .map(|a| {
                                    let tag_string = a.inner_html();
                                    tags_copy
                                        .iter()
                                        .enumerate()
                                        .find(|(_, tag)| **tag == tag_string)
                                        .expect("All tags should be known by this point.")
                                        .0
                                        .try_into()
                                        .expect("There should never be more tags than a u16.")
                                })
                                .collect();

                            // Update the article
                            article.tags = tags;
                            article.page_id = page_id;

                            // Make the vote request
                            // TODO add error handling
                            let mut headers = reqwest::header::HeaderMap::new();
                            headers.insert(
                                "Content-Type",
                                "application/x-www-form-urlencoded; charset=UTF-8"
                                    .parse()
                                    .unwrap(),
                            );
                            headers.insert("user-agent", "SCP_SCRAPER/1.0".parse().unwrap());
                            headers.insert(
                                "Cookie",
                                format!("wikidot_token7={}", API_TOKEN).parse().unwrap(),
                            );

                            let data = format!(
                            "pageId={}&moduleName=pagerate%2FWhoRatedPageModule&wikidot_token7={}",
                            page_id, API_TOKEN
                        );

                            let request = client
                                .request(
                                    reqwest::Method::POST,
                                    "https://scp-wiki.wikidot.com/ajax-module-connector.php",
                                )
                                .headers(headers)
                                .body(data)
                                .send()
                                .unwrap();

                            let text = request.text().unwrap();

                            // Update the article
                            // TODO add error handling
                            for reg_match in user_pattern.captures_iter(text.as_str()) {
                                // Tell the main thread about this user
                                // CONS remove if CPU bound and not mem bound
                                let user_id = reg_match.get(1).unwrap().as_str().parse().unwrap();
                                let url = String::from(reg_match.get(2).unwrap().as_str());
                                let name = String::from(reg_match.get(3).unwrap().as_str());
                                main_tx
                                    .send(ThreadResponse::UserInfo(User {
                                        user_id,
                                        url,
                                        name,
                                    }))
                                    .unwrap();

                                let vote = match reg_match.get(4).unwrap().as_str() {
                                    "+" => 1,
                                    "-" => -1,
                                    _ => unreachable!()
                                };

                                article.votes.push((vote, user_id));
                            }
                            println!("{:?}", article);
                        }
                    });
                });
        });

        // TODO add error checking below this point
        println!("Actually scraping the pages...");
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
                ThreadResponse::UserInfo(user) => {
                    users.insert(user);
                }
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

        let scraped_info = ScrapeInfo {
            articles: articles
                .into_iter()
                .map(|mutex| mutex.into_inner())
                .collect(),
            users,
            tags,
        };

        Ok(scraped_info)
    }

    /// Adds all tags on the wiki to the collection of tags.
    /// This avoids having to build the taglist manually from the pages, which saves a lot of
    /// complexity when multithreading
    fn add_all_tags(&self) -> Result<Vec<String>, ScrapeError> {
        let mut tag_collection = Vec::new();
        let tag_url = "https://scp-wiki.wikidot.com/system:page-tags";
        let client = blocking::Client::new();

        let response = client.get(tag_url).send()?;
        let document = Html::parse_document(response.text()?.as_str());
        let page_item = Selector::parse(r#".tag"#).expect("Hardcoded selector shouldn't fail.");
        let page_elements = document.select(&page_item);

        for page in page_elements {
            let element_html = page.inner_html();

            tag_collection.push(element_html);
        }

        Ok(tag_collection)
    }

    /// Adds all pages on the wiki to the list of pages to scrape.
    /// Pages are determined to be "on" the wiki if they have one of the major tag types (things
    /// like "tale" or "scp"). Since articles must (I think) have exactly one of these, it is
    /// reasonable to use this to discover pages.
    /// # Return
    /// The articles in the returned vector are only the names and links to the articles
    /// and do not have id, vote, or tag information.
    fn add_all_pages(&self) -> Result<Vec<Mutex<Article>>, ScrapeError> {
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
        &self,
        client: &blocking::Client,
        url: &String,
    ) -> Result<Vec<Mutex<Article>>, ScrapeError> {
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

            pages.push(Mutex::new(Article {
                name,
                url,
                page_id: 0,
                tags: Vec::new(),
                votes: Vec::new(),
            }));
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
