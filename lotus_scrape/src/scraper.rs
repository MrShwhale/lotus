mod scrape_writer;
mod scraper_types;

use crate::SCRAPER_HEADING;
use const_format::formatcp;
use http::HeaderMap;
use lazy_static::lazy_static;
use lotus::OutputFiles;
use regex::Regex;
use reqwest::blocking::{self, Client, Response};
use scraper::{Html, Selector};
use scraper_types::*;
use std::{
    collections::HashMap,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread::{self, Scope},
    time::Duration,
};

/// Number of times to try a url before giving up. In reality, urls may be tried more than this
/// in rare circumstances.
const MAX_RETRIES: u8 = 7;

const WIKI_PREFIX: &str = "https://scp-wiki.wikidot.com/";
const TAG_PREFIX: &str = formatcp!("{}system:page-tags/tag/", WIKI_PREFIX);

const WIKIDOT_TOKEN: &str = "123456";

const LISTPAGES: [&str; 13] = [
    "/scp-series",
    "/scp-series-2",
    "/scp-series-3",
    "/scp-series-4",
    "/scp-series-5",
    "/scp-series-6",
    "/scp-series-7",
    "/scp-series-8",
    "/scp-series-9",
    "/scp-series-10",
    "/archived-scps",
    "/scp-ex",
    "/joke-scps",
];

lazy_static! {
    pub static ref PAGE_ID_PATTERN: Regex =
    Regex::new(r#"WIKIREQUEST.info.pageId = (\d+);"#).expect("hardcoded regex, shouldn't fail");
    pub static ref USER_PATTERN: Regex = Regex::new(r#"userInfo\((\d+)\); return false;\\\"  ><img class=\\\"small\\\" src=\\\"https:\\\/\\\/www\.wikidot\.com\\\/avatar\.php\?userid=(?:\d+)&amp;amp;size=small&amp;amp;timestamp=(?:\d+)\\\" alt=\\\"(?:[^\\]+)\\\" style=\\\"background-image:url\(https:\\\/\\\/www\.wikidot\.com\\\/userkarma\.php\?u=(?:\d+)\)\\\"\\\/><\\\/a><a href=\\\"http:\\\/\\\/www\.wikidot\.com\\\/user:info\\\/([^\\]+)\\\" onclick=\\\"WIKIDOT\.page\.listeners\.userInfo\((?:\d+)\); return false;\\\" >([^<]+)<\\/a><\\/span>\\n        <span style=\\"color:#777\\">\\n(?: +)(.)"#).expect("Hardcoded regex should be valid");
}

/// Holds all information that is recorded during a scrape
struct ScrapeInfo {
    articles: Vec<Article>,
    users: HashMap<u64, User>,
    tags: Vec<String>,
}

/// Used to scrape the SCP wiki for votes, tags, and users, and stores that data
pub struct Scraper {
    /// Maximum number of requests to send at once. Also the number of additional system threads to create
    max_concurrent_requests: u8,
    /// Delay between requests in milliseconds. This may have fluctuations due to
    /// calculation time and retrying requests.
    download_delay: u64,
}

impl Scraper {
    pub fn new() -> Scraper {
        Scraper {
            max_concurrent_requests: 8,
            download_delay: 0,
        }
    }

    pub fn new_with_options(max_concurrent_requests: u8, download_delay: u64) -> Scraper {
        assert!(
            max_concurrent_requests != 0,
            "max_concurrent_requests must be more than 0!"
        );

        Scraper {
            max_concurrent_requests,
            download_delay: (download_delay * max_concurrent_requests as u64),
        }
    }

    /// Scrapes the SCP wiki and records the information in a format which the rest of this program can use.
    pub fn scrape(
        self,
        article_limit: usize,
        tag_pages: Vec<&str>,
        outputs: OutputFiles,
    ) -> Result<(), ScrapeError> {
        eprintln!("{}Getting page list", SCRAPER_HEADING);
        let mut scrape_list = self.create_page_list(tag_pages)?;

        // Limit the number of pages (debugging, mostly)
        scrape_list.truncate(article_limit);

        eprintln!("{}Getting the list of tags", SCRAPER_HEADING);
        let tag_group = self.scrape_all_tags()?;

        eprintln!("{}Scraping the pages", SCRAPER_HEADING);
        let scraped_info = self.scrape_pages(scrape_list, tag_group)?;

        scrape_writer::record_info(scraped_info, outputs)?;

        Ok(())
    }

    /// Adds all pages on the wiki to the list of pages to scrape.
    /// Pages are determined to be "on" the wiki if they have one of the major tag types (things
    /// like "tale" or "scp"). Since the target articles must have exactly one of these, it is
    /// reasonable to use this to discover pages.
    fn create_page_list(&self, tag_types: Vec<&str>) -> Result<Vec<Article>, ScrapeError> {
        let mut tag_url;
        let mut articles = Vec::new();
        let client = blocking::Client::new();

        let page_item = Selector::parse(r#"div[class="pages-list-item"]"#)
            .expect("Hardcoded selector shouldn't fail");
        let name_pattern = Regex::new(r#"<a href="(?<url>.+)">(?<name>.+)+</a>"#)
            .expect("Hardcoded regex shouldn't fail");

        let name_map = self.listpages_scrape(&client, LISTPAGES.to_vec())?;

        for tag in tag_types.iter() {
            tag_url = String::from(TAG_PREFIX);
            tag_url.push_str(tag);
            let mut pages = self.extract_links_from_syspage(
                &client,
                &tag_url,
                &page_item,
                &name_pattern,
                &name_map,
            )?;
            articles.append(&mut pages);

            // Avoid throttling, even at 0 delay (otherwise this is too fast).
            thread::sleep(Duration::from_millis(self.download_delay + 100));
        }

        Ok(articles)
    }

    /// Scrapes the links to the given articles, overwriting existing data in each element
    fn scrape_pages(
        &self,
        mut articles: Vec<Article>,
        mut tags: Vec<String>,
    ) -> Result<ScrapeInfo, ScrapeError> {
        let num_articles = articles.len();
        let num_threads = self.max_concurrent_requests;

        let mut users = HashMap::new();

        // Create the message passing mechanism
        let (main_tx, main_rx) = mpsc::channel();
        let (thread_txs, thread_rxs): (Vec<_>, Vec<_>) =
            (0..num_threads).map(|_| mpsc::channel()).unzip();

        let tags_arc = Arc::new(&mut tags);

        // Create the threads
        eprintln!("{}Creating the threads...", SCRAPER_HEADING);
        lazy_static::initialize(&PAGE_ID_PATTERN);
        lazy_static::initialize(&USER_PATTERN);
        thread::scope(|scope| {
            for (id, thread_rx) in thread_rxs.into_iter().enumerate() {
                let tags_copy = tags_arc.clone();
                let main_tx = main_tx.clone();
                self.spawn_scraper_thread(&scope, main_tx, id, thread_rx, tags_copy);
            }

            eprintln!("{}Actually scraping the pages...", SCRAPER_HEADING);
            run_messaging(
                &mut articles,
                num_articles,
                &main_rx,
                &thread_txs,
                &mut users,
            )?;

            // Ask threads to stop
            for thread_tx in thread_txs.iter() {
                // Any dead threads mean the scope is going to panic otherwise, so leave gracefully
                // now
                if let Err(_) = thread_tx.send(ThreadResponse::EndRequest) {
                    return Err(ScrapeError::ThreadError);
                };
            }

            // Wait for all to stop
            let mut num_alive = num_threads;
            while num_alive > 0 {
                let thread_message = match main_rx.recv() {
                    Ok(message) => message,
                    Err(_) => return Err(ScrapeError::ThreadError),
                };

                match thread_message {
                    ThreadResponse::Alright | ThreadResponse::ArticleRequest(_) => num_alive -= 1,
                    // Though they have all been told to stop, threads only recieve requests after
                    // sending all remaining user info, so it has to be handlede as well
                    ThreadResponse::UserInfo(user) => {
                        users.insert(user.user_id, user);
                    }
                    _ => unreachable!(),
                };
            }

            Ok(())
        })?;

        let scraped_info = ScrapeInfo {
            articles,
            users,
            tags,
        };

        Ok(scraped_info)
    }

    /// Adds all tags on the wiki to the collection of tags.
    /// This avoids having to build the taglist manually from the pages, which saves a lot of
    /// complexity when multithreading
    fn scrape_all_tags(&self) -> Result<Vec<String>, ScrapeError> {
        let mut tag_collection = Vec::new();
        let client = blocking::Client::new();

        eprintln!("{}Making tags page request", SCRAPER_HEADING);
        let response = self.retry_get_request(&client, TAG_PREFIX)?;
        eprintln!("{}Getting tags page response", SCRAPER_HEADING);
        let document = Html::parse_document(response.text()?.as_str());
        let page_item = Selector::parse(".tag").expect("Hardcoded selector shouldn't fail");
        let page_elements = document.select(&page_item);

        for page in page_elements {
            let element_html = page.inner_html();

            tag_collection.push(element_html);
        }

        Ok(tag_collection)
    }

    /// Retry the given request several times to avoid minor internet errors
    fn retry_request(
        &self,
        client: &Client,
        headers: &HeaderMap,
        data: &String,
        method: reqwest::Method,
        url: &str,
    ) -> Result<Response, ScrapeError> {
        let request;
        let mut retries = 0;

        loop {
            let response = client
                .request(method.clone(), url)
                .headers(headers.clone())
                .body(data.clone())
                .send();

            match response {
                Ok(value) => {
                    request = value;
                    break;
                }
                Err(err) => {
                    retries += 1;
                    if retries >= MAX_RETRIES {
                        return Err(err.into());
                    }
                    thread::sleep(Duration::from_millis(self.download_delay));
                }
            }
        }

        Ok(request)
    }

    /// Shorthand to send a get request to the url as many times as it takes
    fn retry_get_request(&self, client: &Client, url: &str) -> Result<Response, ScrapeError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "user-agent",
            "Mozilla/5.0"
                .parse()
                .expect("Hardcoded header shoud be valid"),
        );
        let data = String::new();
        self.retry_request(client, &headers, &data, reqwest::Method::GET, url)
    }

    fn listpages_scrape(
        &self,
        client: &blocking::Client,
        listpages: Vec<&'static str>,
    ) -> Result<HashMap<String, String>, ScrapeError> {
        eprintln!("{}Getting SCP names from ListPages", SCRAPER_HEADING);
        let mut name_map = HashMap::new();
        let selector = Selector::parse("h1~ul>li").expect("Hardcoded selector should not fail");
        for listpage in listpages {
            let mut url = String::from(WIKI_PREFIX);
            url.push_str(listpage);
            let response = self.retry_get_request(client, url.as_str())?;
            let document = Html::parse_document(response.text()?.as_str());
            let elements = document.select(&selector);

            // The first 2 elements are not actual page links
            for element in elements.skip(3) {
                // Store the url as the key
                let link_element = element.first_child().unwrap().value().as_element().unwrap();

                // The only class possible is newpage
                if link_element.attr("class").is_some() {
                    continue;
                }

                let key = link_element.attr("href");

                let key = match key {
                    Some(string) => String::from(&string[1..]),
                    None => continue,
                };

                // Store the text of the elements as the value
                let value = element.text().collect::<Vec<&str>>().join("");

                name_map.insert(key, value);
            }
        }

        Ok(name_map)
    }

    /// Adds all articles on a system page to a Vec then returns it.
    /// Does not add directly to the Vec to make multithreading easy.
    /// This is blocking since this should be run before starting the real
    /// scraper and should have a very limited number of requests.
    fn extract_links_from_syspage(
        &self,
        client: &blocking::Client,
        url: &str,
        page_item: &Selector,
        name_pattern: &Regex,
        name_map: &HashMap<String, String>,
    ) -> Result<Vec<Article>, ScrapeError> {
        let response = self.retry_get_request(client, url)?;
        let document = Html::parse_document(response.text()?.as_str());
        let page_elements = document.select(page_item);

        let mut pages = Vec::new();
        for page in page_elements {
            let element_html = page.inner_html();

            let captures = match name_pattern.captures(element_html.as_str()) {
                Some(cap) => cap,
                None => return Err(ScrapeError::RegexError),
            };

            let url = match captures.name("url") {
                Some(url) => String::from(
                    url.as_str()
                        .get(1..)
                        .expect("Valid links should always have more than 1 char"),
                ),
                None => return Err(ScrapeError::RegexError),
            };

            // I admire and admonish the individual who is making me write this case
            // TODO Fix this in a better way (check at the pid stage)
            if url == "scp-1047-j" {
                continue;
            }

            let name = if name_map.contains_key(&url) {
                name_map
                    .get(&url)
                    .expect("Manually checked existence")
                    .clone()
            } else {
                match captures.name("name") {
                    Some(name) => String::from(name.as_str()),
                    None => return Err(ScrapeError::RegexError),
                }
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

    /// Create a thread within the scope that will make scraper requests
    fn spawn_scraper_thread<'a, 'scope, 'env>(
        &'a self,
        scope: &'scope Scope<'scope, 'env>,
        main_tx: Sender<ThreadResponse>,
        id: usize,
        thread_rx: Receiver<ThreadResponse>,
        tags_copy: Arc<&'a mut Vec<String>>,
    ) where
        'a: 'scope,
    {
        let tag_selector =
            Selector::parse(r#"div.page-tags a"#).expect("Hardcoded selector should not fail");

        scope.spawn(move || {
            let client = blocking::Client::new();
            loop {
                // Tell main which thread needs an article
                main_tx
                    .send(ThreadResponse::ArticleRequest(id))
                    .expect("The reciever should never be deallocated");

                // Wait for a response
                let article_ptr = match thread_rx
                    .recv()
                    .expect("The sender should never be disconnected")
                {
                    ThreadResponse::ArticleResponse(raw_pointer) => raw_pointer,
                    ThreadResponse::EndRequest => {
                        main_tx
                            .send(ThreadResponse::Alright)
                            .expect("The reciever should never be deallocated");
                        break;
                    }
                    _ => unreachable!(),
                };

                // This is safe since the main thread only passes out one raw mutable pointer at a
                // time.
                let article: &mut Article = unsafe { article_ptr.get_mut_ptr() };

                let url = String::from(WIKI_PREFIX) + article.url.as_str();

                eprintln!("{}Request sent: {}", SCRAPER_HEADING, url);

                let document_text;
                let mut retries = 0;

                loop {
                    let response = self
                        .retry_get_request(&client, &url)
                        .expect("Too many failed web requests");

                    match response.text() {
                        Ok(other) => {
                            document_text = other;
                            break;
                        }
                        Err(e) => {
                            if retries >= MAX_RETRIES {
                                panic!("Multi-retry error: {:?}", e);
                            }
                            retries += 1;
                        }
                    };
                }

                eprintln!("{}Response recieved: {}", SCRAPER_HEADING, url);

                let page_id_captures = PAGE_ID_PATTERN
                    .captures(document_text.as_str())
                    .expect("Match falied in document response");

                let page_id = page_id_captures
                    .get(1)
                    .expect("Page ID match failed")
                    .as_str();
                let page_id: u64 = page_id.parse().expect("Page ID parse failed");

                let document = Html::parse_document(document_text.as_str());
                let tags: Vec<_> = document
                    .select(&tag_selector)
                    .map(|a| {
                        let tag_string = a.inner_html();
                        tags_copy
                            .iter()
                            .enumerate()
                            .find(|(_, tag)| **tag == tag_string)
                            .expect("All tags should be known by this point")
                            .0
                            .try_into()
                            .expect("There should never be more tags than a u16")
                    })
                    .collect();

                article.tags = tags;
                article.page_id = page_id;

                let text = match self.make_vote_request(&client, page_id) {
                    Ok(text) => text,
                    Err(e) => panic!("Multi-retry: {:?}", e),
                };

                // Send the user responses
                match update_article_votes(text, &main_tx, article) {
                    Err(e) => panic!("Thread error: {:?}", e),
                    _ => (),
                }
            }
        });
    }

    /// Request the vote records for a page
    fn make_vote_request(&self, client: &Client, page_id: u64) -> Result<String, ScrapeError> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Content-Type",
            "application/x-www-form-urlencoded; charset=UTF-8"
                .parse()
                .expect("Hardcoded header should be valid"),
        );
        headers.insert(
            "user-agent",
            "Mozilla/5.0"
                .parse()
                .expect("Hardcoded header shoud be valid"),
        );
        headers.insert(
            "Cookie",
            format!("wikidot_token7={}", WIKIDOT_TOKEN)
                .parse()
                .expect("Predictable header should be valid"),
        );

        let data = format!(
            "pageId={}&moduleName=pagerate%2FWhoRatedPageModule&wikidot_token7={}",
            page_id, WIKIDOT_TOKEN
        );

        // Retry more times if getting EOF error
        let text;
        let mut retries = 0;

        loop {
            let request = self.retry_request(
                client,
                &headers,
                &data,
                reqwest::Method::POST,
                "https://scp-wiki.wikidot.com/ajax-module-connector.php",
            )?;

            match request.text() {
                Ok(other) => {
                    text = other;
                    break;
                }
                Err(e) => {
                    if retries >= MAX_RETRIES {
                        eprintln!("{}Multi-retry error: {:?}", SCRAPER_HEADING, e);
                        panic!();
                    }
                    retries += 1;
                }
            };
        }

        Ok(text)
    }
}

/// Run the messaging mechanism until all the articles have been sent out (though not recieved)
fn run_messaging(
    articles: &mut Vec<Article>,
    num_articles: usize,
    main_rx: &Receiver<ThreadResponse>,
    thread_txs: &Vec<Sender<ThreadResponse>>,
    users: &mut HashMap<u64, User>,
) -> Result<(), ScrapeError> {
    let mut next_article = 0;

    while next_article < num_articles {
        let response = main_rx.recv()?;
        match response {
            ThreadResponse::ArticleRequest(id) => {
                let next_article_ptr = RawPointerWrapper {
                    raw: articles
                        .get_mut(next_article)
                        .expect("next_article should never be OOB"),
                };

                thread_txs
                    .get(id)
                    .expect("ID should never be OOB")
                    .send(ThreadResponse::ArticleResponse(next_article_ptr))?;

                next_article += 1;
            }
            ThreadResponse::UserInfo(user) => {
                users.insert(user.user_id, user);
            }
            _ => unreachable!(),
        };
    }

    Ok(())
}

/// Parse an article page, and tell the main thread about all the users in it
fn update_article_votes(
    text: String,
    main_tx: &Sender<ThreadResponse>,
    article: &mut Article,
) -> Result<(), ScrapeError> {
    for reg_match in USER_PATTERN.captures_iter(text.as_str()) {
        let user_id = reg_match
            .get(1)
            .expect("User id should match")
            .as_str()
            .parse()
            .expect("User id should be representable as u64");
        let url = String::from(reg_match.get(2).expect("User URL should match").as_str());
        let name = String::from(reg_match.get(3).expect("User name should match").as_str());
        main_tx.send(ThreadResponse::UserInfo(User { user_id, url, name }))?;

        let vote = match reg_match.get(4).expect("User vote should match").as_str() {
            "+" => 1,
            "-" => -1,
            _ => unreachable!(),
        };

        article.votes.push((vote, user_id));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::panic;

    // Directory where output files can be found
    const OUTPUT_DIR: &str = "../output";

    // Files to save scraped data to
    const ARTICLE_OUTPUT: &str = formatcp!("{}/articles.parquet", OUTPUT_DIR);
    const TAGS_OUTPUT: &str = formatcp!("{}/tags.parquet", OUTPUT_DIR);
    const USERS_OUTPUT: &str = formatcp!("{}/users.parquet", OUTPUT_DIR);
    const VOTES_OUTPUT: &str = formatcp!("{}/votes.parquet", OUTPUT_DIR);

    #[test]
    fn scrape() {
        let scraper = Scraper::new();

        let outputs = OutputFiles {
            article_output: String::from(ARTICLE_OUTPUT),
            tags_output: String::from(TAGS_OUTPUT),
            users_output: String::from(USERS_OUTPUT),
            votes_output: String::from(VOTES_OUTPUT),
        };

        match scraper.scrape(8, vec!["scp"], outputs) {
            Ok(_) => (),
            Err(e) => {
                println!("Something went wrong! Specifically, this:");
                println!("{:?}", e);
                panic!();
            }
        }
    }

    #[test]
    fn basic_listpages_scrape() {
        let client = blocking::Client::new();
        let scraper = Scraper::new();
        let name_map = &scraper
            .listpages_scrape(&client, vec!["/scp-series"])
            .unwrap();

        assert!(name_map.get("scp-096").unwrap() == "SCP-096 - The \"Shy Guy\"");
    }
}
