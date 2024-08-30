pub mod recommender;
pub mod server;
pub mod scraper;

use const_format::formatcp;

// CONS make proper docs with doc comments

// Message to print before all logs from different sources
pub const SCRAPER_HEADING: &str = "[SCRAPER] ";
pub const SERVER_HEADING: &str = "[SERVER] ";

// The main tag types. Any page not tagged with one of these will not be included in the scrape.
pub const TAG_TYPES: [&str; 4] = ["scp", "tale", "hub", "goi-format"];

// Directory where output files can be found
pub const OUTPUT_DIR: &str = "./output";

// Files to save scraped data to
pub const ARTICLE_OUTPUT: &str = formatcp!("{}/articles.parquet", OUTPUT_DIR);
pub const TAGS_OUTPUT: &str = formatcp!("{}/tags.parquet", OUTPUT_DIR);
pub const USERS_OUTPUT: &str = formatcp!("{}/users.parquet", OUTPUT_DIR);
pub const VOTES_OUTPUT: &str = formatcp!("{}/votes.parquet", OUTPUT_DIR);
