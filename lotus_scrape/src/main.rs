use lotus::OutputFiles;
use lotus_scrape::{scraper::Scraper, SCRAPER_HEADING, TAG_TYPES};
use std::{env, process};

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut index = 1;
    let length = args.len();

    let mut article_limit = usize::MAX;
    let mut max_concurrent_requests = 8;
    let mut download_delay = 0;
    let mut outputs = OutputFiles::new();

    while index < length {
        match args[index].as_str() {
            "--article-file" | "-a" => {
                let articles_file = args.get(index + 1).expect("No article file specified");
                index += 1;
                outputs.article_output = articles_file.clone();
            }
            "--tags-file" | "-t" => {
                let tags_file = args.get(index + 1).expect("No tags file specified");
                index += 1;
                outputs.tags_output = tags_file.clone();
            }
            "--users-file" | "-u" => {
                let users_file = args.get(index + 1).expect("No users file specified");
                index += 1;
                outputs.users_output = users_file.clone();
            }
            "--votes-file" | "-v" => {
                let votes_file = args.get(index + 1).expect("No votes file specified");
                index += 1;
                outputs.votes_output = votes_file.clone();
            }
            "--article-limit" | "-l" => {
                article_limit = args
                    .get(index + 1)
                    .expect("No article limit specified")
                    .parse()
                    .expect("Article limit should be a number");
                index += 1;
            }
            "--concurrent-requests" | "-r" => {
                max_concurrent_requests = args
                    .get(index + 1)
                    .expect("No max concurrent requests specified")
                    .parse()
                    .expect("Max concurrent requests should be a number");
                index += 1;
            }
            "--download-delay" | "-d" => {
                download_delay = args
                    .get(index + 1)
                    .expect("No download delay specified")
                    .parse()
                    .expect("Download delay should be a number");
                index += 1;
            }
            "--help" | "-h" => {
                eprintln!("Usage: lotus_scrape [args]\n  If an arg is passed multiple times, only the rightmost is considered.\n\n  Output file arguments:           Specify the save location for different data.\n    --article-file        or -a    Default: ./output/articles.parquet\n    --tags-file           or -t    Default: ./output/tags.parquet\n    --users-file          or -u    Default: ./output/users.parquet\n    --votes-file          or -v    Default: ./output/votes.parquet\n\n  Other options:\n    Sets the number of articles to fetch from the wiki. Each article takes about 2 web requests to get.\n    --article-limit       or -l    Default: maximum\n\n    Sets the number of requests to make at one time (the number of additional threads to make).\n    --concurrent-requests or -c    Default: 8\n\n    Sets the additional approximate delay between requests, in milliseconds.\n    This time is added in between each web request.\n    --download-delay      or -d    Default: 0\n\n    Display this message instead of running the system.\n    --help                or -h");
                process::exit(1)
            }
            other => {
                eprintln!(
                    "Unknown command line option: {}.\nRun with --help (or -h) for valid commands.",
                    other
                );
                process::exit(1)
            }
        };

        index += 1;
    }

    eprintln!("{}Scraping the wiki...", SCRAPER_HEADING);
    let scraper = Scraper::new_with_options(max_concurrent_requests, download_delay);

    let result = scraper.scrape(article_limit, Vec::from(TAG_TYPES), outputs);

    match result {
        Ok(_) => eprintln!("{}Scrape completed successfully!", SCRAPER_HEADING),
        Err(e) => {
            eprintln!(
                "{}Something went wrong! Specifically, this: {:?}",
                SCRAPER_HEADING, e
            );
            process::exit(1)
        }
    }
}
