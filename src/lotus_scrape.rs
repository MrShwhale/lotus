mod scraper;

use const_format::formatcp;
use scraper::Scraper;

// CONS make proper docs with doc comments

// Message to print before all scraper logs
const SCRAPER_HEADING: &str = "[SCRAPER] ";

// The main tag types. Any page not tagged with one of these will not be included in the scrape.
const TAG_TYPES: [&str; 4] = ["scp", "tale", "hub", "goi-format"];

// Directory where output files can be found
pub const OUTPUT_DIR: &str = "./output";

// Files to save scraped data to
pub const ARTICLE_OUTPUT: &str = formatcp!("{}/articles.parquet", OUTPUT_DIR);
pub const TAGS_OUTPUT: &str = formatcp!("{}/tags.parquet", OUTPUT_DIR);
pub const USERS_OUTPUT: &str = formatcp!("{}/users.parquet", OUTPUT_DIR);
pub const VOTES_OUTPUT: &str = formatcp!("{}/votes.parquet", OUTPUT_DIR);

fn main() {
    eprintln!("{}Scraping the wiki...", SCRAPER_HEADING);
    let scraper = Scraper::new();

    let result = scraper.scrape(usize::MAX, Vec::from(TAG_TYPES));

    match result {
        Ok(_) => eprintln!("{}Scrape completed successfully!", SCRAPER_HEADING),
        Err(e) => {
            eprint!(
                "{}Something went wrong! Specifically, this: {:?}",
                SCRAPER_HEADING, e
            );
        }
    }
}
