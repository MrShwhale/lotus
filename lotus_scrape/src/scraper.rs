mod scrape_writer;
mod scraper_types;

use crate::SCRAPER_HEADING;
use const_format::formatcp;
use http::HeaderMap;
use lotus::{ARTICLE_OUTPUT, TAGS_OUTPUT, USERS_OUTPUT, VOTES_OUTPUT};
use parking_lot::Mutex;
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

const WIKI_PREFIX: &str = "https://scp-wiki.wikidot.com/";
const TAG_PREFIX: &str = formatcp!("{}system:page-tags/tag/", WIKI_PREFIX);

// This is not a real api token, meant for debugging purposes
const WIKIDOT_TOKEN: &str = "123456";

// Holds all information that is recorded during a scrape
struct ScrapeInfo {
    articles: Vec<Article>,
    users: HashMap<u64, User>,
    tags: Vec<String>,
}

/// Used to scrape the SCP wiki for votes, tags, and users, and stores that data
pub struct Scraper {
    /// Maximum number of requests to send at once. Also the number of additional system threads to create
    max_concurrent_requests: u8,
    /// Delay between requests in milliseconds. This happens once per thread, so for a "true" value
    /// it should be divided by max_concurrent_requests.
    download_delay: u64,
    /// Number of times to try a url before giving up. In reality, urls may be tried more than this
    /// many times in rare circumstances.
    max_retries: u8,
}

impl Scraper {
    pub fn new() -> Scraper {
        Scraper {
            max_concurrent_requests: 8,
            download_delay: 0,
            max_retries: 7,
        }
    }

    // Scrapes the full SCP wiki and records the information in a format which the rest of this program can use.
    pub fn scrape(self, article_limit: usize, tag_pages: Vec<&str>) -> Result<(), ScrapeError> {
        // Get the list of articles to be scraped on the wiki
        eprintln!("{}Getting page list", SCRAPER_HEADING);
        let mut scrape_list = self.add_all_pages(tag_pages)?;

        // Limit the number of pages (debugging, mostly)
        scrape_list.truncate(article_limit);

        // Scrape the tags
        println!("Getting the list of tags...");
        let tag_group = self.add_all_tags()?;

        // Actually scrape each article
        println!("Scraping the pages...");
        let scraped_info = self.scrape_pages(scrape_list, tag_group)?;

        // Record the scraped info
        scrape_writer::record_info(
            scraped_info,
            ARTICLE_OUTPUT,
            TAGS_OUTPUT,
            USERS_OUTPUT,
            VOTES_OUTPUT,
        )?;

        Ok(())
    }

    /// Scrapes the links to the given articles, overwriting existing data in each element
    fn scrape_pages(
        &self,
        mut articles: Vec<Mutex<Article>>,
        mut tags: Vec<String>,
    ) -> Result<ScrapeInfo, ScrapeError> {
        let num_articles = articles.len();
        let num_threads = self.max_concurrent_requests;

        let mut users = HashMap::new();

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
            for (id, thread_rx) in thread_rxs.into_iter().enumerate() {
                let articles_copy = articles_arc.clone();
                let tags_copy = tags_arc.clone();
                let main_tx = main_tx.clone();
                spawn_scraper_thread(&scope, main_tx, id, thread_rx, articles_copy, tags_copy);
            }

            // TODO add error checking below this point
            println!("Actually scraping the pages...");
            run_messaging(
                num_articles,
                &main_rx,
                &thread_txs,
                self.download_delay,
                &mut users,
            );

            // Ask threads to stop
            let mut num_alive = num_threads;
            for thread_tx in thread_txs.iter() {
                // Dead threads are considered lost
                if let Err(_) = thread_tx.send(ThreadResponse::EndRequest) {
                    num_alive -= 1;
                };
            }

            // Wait for all to stop
            while num_alive > 0 {
                let thread_message = main_rx.recv().unwrap();
                match thread_message {
                    ThreadResponse::Alright | ThreadResponse::ArticleRequest(_) => num_alive -= 1,
                    ThreadResponse::UserInfo(user) => {
                        users.insert(user.user_id, user);
                    }
                    // Threads will only send the above 2 requests
                    _ => unreachable!(),
                }
            }
        });

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

    // Adds all tags on the wiki to the collection of tags.
    // This avoids having to build the taglist manually from the pages, which saves a lot of
    // complexity when multithreading
    fn add_all_tags(&self) -> Result<Vec<String>, ScrapeError> {
        let mut tag_collection = Vec::new();
        let tag_url = "https://scp-wiki.wikidot.com/system:page-tags";
        let client = blocking::Client::new();

        println!("Making request");
        let response = retry_get_request(&client, tag_url)?;
        println!("Getting response");
        let document = Html::parse_document(response.text()?.as_str());
        let page_item = Selector::parse(".tag").expect("Hardcoded selector shouldn't fail.");
        let page_elements = document.select(&page_item);

        for page in page_elements {
            let element_html = page.inner_html();

            tag_collection.push(element_html);
        }

        Ok(tag_collection)
    }

    // Adds all pages on the wiki to the list of pages to scrape.
    // Pages are determined to be "on" the wiki if they have one of the major tag types (things
    // like "tale" or "scp"). Since articles must (I think) have exactly one of these, it is
    // reasonable to use this to discover pages.
    fn add_all_pages(&self, tag_types: Vec<&str>) -> Result<Vec<Mutex<Article>>, ScrapeError> {
        let mut tag_url;
        let mut articles = Vec::new();
        let client = blocking::Client::new();
        for tag in tag_types.iter() {
            tag_url = String::from(TAG_PREFIX);
            tag_url.push_str(tag);
            let mut pages = self.extract_links_from_syspage(&client, &tag_url)?;
            articles.append(&mut pages);

            // Avoid throttling
            thread::sleep(Duration::from_millis(self.download_delay + 100));
        }

        Ok(articles)
    }

    // Adds all articles on a system page to a Vec then returns it.
    // Does not add directly to the Vec to make multithreading easy.
    // This is blocking since this should be run before starting the real
    // scraper and should have a very limited number of requests.
    fn extract_links_from_syspage(
        &self,
        client: &blocking::Client,
        url: &String,
    ) -> Result<Vec<Mutex<Article>>, ScrapeError> {
        let response = retry_get_request(client, url)?;
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

            // I admire and admonish the individual who is making me write this case
            // TODO make sure this works
            if url == "scp-1047-j" {
                continue;
            }

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

    fn retry_get_request(&self, client: &Client, url: &str) -> Result<Response, ScrapeError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "user-agent",
            "Mozilla/5.0"
                .parse()
                .expect("Hardcoded header shoud be valid."),
        );
        let data = String::new();
        self.retry_request(client, &headers, &data, reqwest::Method::GET, url)
    }

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
                },
                Err(err) => {
                    retries += 1;
                    if retries >= self.max_retries {
                        return err;
                    }
                },
            }
        }

        Ok(request)
    }

    fn spawn_scraper_thread<'a, 'scope, 'env>(
        scope: &'scope Scope<'scope, 'env>,
        main_tx: Sender<ThreadResponse>,
        id: usize,
        thread_rx: Receiver<ThreadResponse>,
        articles_copy: Arc<&'a mut Vec<Mutex<Article>>>,
        tags_copy: Arc<&'a mut Vec<String>>,
    ) where
        'a: 'scope,
    {
        scope.spawn(move || {
            let client = blocking::Client::new();
            let page_id_pattern =
            Regex::new(r#"WIKIREQUEST.info.pageId = (\d+);"#).expect("hardcoded regex, shouldn't fail");
            let user_pattern = Regex::new(r#"userInfo\((\d+)\); return false;\\\"  ><img class=\\\"small\\\" src=\\\"https:\\\/\\\/www\.wikidot\.com\\\/avatar\.php\?userid=(?:\d+)&amp;amp;size=small&amp;amp;timestamp=(?:\d+)\\\" alt=\\\"(?:[^\\]+)\\\" style=\\\"background-image:url\(https:\\\/\\\/www\.wikidot\.com\\\/userkarma\.php\?u=(?:\d+)\)\\\"\\\/><\\\/a><a href=\\\"http:\\\/\\\/www\.wikidot\.com\\\/user:info\\\/([^\\]+)\\\" onclick=\\\"WIKIDOT\.page\.listeners\.userInfo\((?:\d+)\); return false;\\\" >([^<]+)<\\/a><\\/span>\\n        <span style=\\"color:#777\\">\\n(?: +)(.)"#).expect("Hardcoded regex should be valid.");
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
                let url = String::from(WIKI_PREFIX) + article.url.as_str();

                // Make the article request
                // TODO error checking
                // TODO consolidate naming
                println!("url sent: {}", url);

                let document_text;
                let mut retries = 0;

                // BUG you DO NOT know why this request (and the other one with a similar loop around
                // it) sometimes send a 200 response which have EOF in the middle of chunks. Fixing
                // this is very important, below is *NOT* a fix
                loop {
                    let response = retry_get_request(&client, &url).unwrap();

                    match response.text() {
                        Ok(other) => {
                            document_text = other;
                            break
                        },
                        Err(e) => {
                            if retries >= MAX_RETRIES {
                                println!("\x1b[93mError\x1b[0m: {:?}", e);
                                panic!("558");
                            }
                            retries += 1;
                        }
                    };
                }

                println!("url_response: {}", url);

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
                        .expect("Hardcoded header should be valid."),
                );
                headers.insert("user-agent", "Mozilla/5.0".parse().expect("Hardcoded header shoud be valid."));
                headers.insert(
                    "Cookie",
                    format!("wikidot_token7={}", WIKIDOT_TOKEN).parse().expect("Predictable header should be valid."),
                );

                let data = format!(
                "pageId={}&moduleName=pagerate%2FWhoRatedPageModule&wikidot_token7={}",
                page_id, WIKIDOT_TOKEN
            );

                // Retry more times if getting EOF error
                let text;
                let mut retries = 0;

                loop {
                    let request = retry_request(&client, &headers, &data, reqwest::Method::POST, "https://scp-wiki.wikidot.com/ajax-module-connector.php").unwrap();

                    match request.text() {
                        Ok(other) => {
                            text = other;
                            break
                        },
                        Err(e) => {
                            if retries >= MAX_RETRIES {
                                println!("\x1b[93mError\x1b[0m: {:?}", e);
                                panic!("558");
                            }
                            retries += 1;
                        }
                    };
                }

                // Update the article
                // Send the user responses
                // TODO add error handling
                for reg_match in user_pattern.captures_iter(text.as_str()) {
                    // Tell the main thread about this user
                    // CONS remove if CPU bound and not mem bound
                    let user_id = reg_match.get(1).expect("UNEX; uid match").as_str().parse().expect("User id should be representable as u64.");
                    let url = String::from(reg_match.get(2).expect("UNEX; url match").as_str());
                    let name = String::from(reg_match.get(3).expect("UNEX; name match").as_str());
                    main_tx
                        .send(ThreadResponse::UserInfo(User {
                            user_id,
                            url,
                            name,
                        }))
                        .unwrap();

                    let vote = match reg_match.get(4).expect("UNEX; vote match").as_str() {
                        "+" => 1,
                        "-" => -1,
                        _ => unreachable!()
                    };

                    article.votes.push((vote, user_id));
                }
            }
        });
    }
}

fn run_messaging(
    num_articles: usize,
    main_rx: &Receiver<ThreadResponse>,
    thread_txs: &Vec<Sender<ThreadResponse>>,
    download_delay: u64,
    users: &mut HashMap<u64, User>,
) {
    let mut next_article = 0;
    let wait_time = Duration::from_millis(download_delay);

    // TODO add error checking below this point
    while next_article < num_articles {
        // Wait for a perm request
        let response = main_rx.recv().unwrap();
        match response {
            ThreadResponse::ArticleRequest(id) => {
                thread_txs
                    .get(id)
                    .expect("ID should never be OOB.")
                    .send(ThreadResponse::ArticleRequest(next_article))
                    .unwrap();

                next_article += 1;
                thread::sleep(wait_time);
            }
            ThreadResponse::UserInfo(user) => {
                users.insert(user.user_id, user);
            }
            // Threads can only send the above requests
            _ => unreachable!(),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::panic;

    #[test]
    fn scrape() {
        let scraper = Scraper::new();

        match scraper.scrape(8, vec!["hub"]) {
            Ok(_) => (),
            Err(e) => {
                println!("Something went wrong! Specifically, this:");
                println!("{:?}", e);
                panic!();
            }
        }
    }
}
