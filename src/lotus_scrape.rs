mod scraper;

use const_format::formatcp;
use scraper::Scraper;

// The main tag types. Any page not tagged with one of these will not be included in the scrape.
const TAG_TYPES: [&str; 4] = ["scp", "tale", "hub", "goi-format"];

// Output location constants
pub const OUTPUT_DIR: &str = "./output";

pub const ARTICLE_OUTPUT: &str = formatcp!("{}/articles.parquet", OUTPUT_DIR);
pub const TAGS_OUTPUT: &str = formatcp!("{}/tags.parquet", OUTPUT_DIR);
pub const USERS_OUTPUT: &str = formatcp!("{}/users.parquet", OUTPUT_DIR);
pub const VOTES_OUTPUT: &str = formatcp!("{}/votes.parquet", OUTPUT_DIR);

fn main() {
    println!("Scraping the wiki...");
    let scraper = Scraper::new();

    match scraper.scrape(usize::MAX, Vec::from(TAG_TYPES)) {
        Ok(_) => println!("Scrape completed successfully!"),
        Err(e) => {
            println!("Something went wrong! Specifically, this:");
            println!("{:?}", e);
        }
    }
}
