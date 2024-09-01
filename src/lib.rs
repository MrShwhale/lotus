use const_format::formatcp;

// Directory where output files can be found
pub const OUTPUT_DIR: &str = "./output";

// Files to save scraped data to
pub const ARTICLE_OUTPUT: &str = formatcp!("{}/articles.parquet", OUTPUT_DIR);
pub const TAGS_OUTPUT: &str = formatcp!("{}/tags.parquet", OUTPUT_DIR);
pub const USERS_OUTPUT: &str = formatcp!("{}/users.parquet", OUTPUT_DIR);
pub const VOTES_OUTPUT: &str = formatcp!("{}/votes.parquet", OUTPUT_DIR);