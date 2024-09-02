use const_format::formatcp;

// Directory where output files can be found
const OUTPUT_DIR: &str = "./output";

// Files to save scraped data to
const ARTICLE_OUTPUT: &str = formatcp!("{}/articles.parquet", OUTPUT_DIR);
const TAGS_OUTPUT: &str = formatcp!("{}/tags.parquet", OUTPUT_DIR);
const USERS_OUTPUT: &str = formatcp!("{}/users.parquet", OUTPUT_DIR);
const VOTES_OUTPUT: &str = formatcp!("{}/votes.parquet", OUTPUT_DIR);

// CONS nonstatic lifetime
#[derive(Clone)]
pub struct OutputFiles {
    pub article_output: &'static str,
    pub tags_output: &'static str,
    pub users_output: &'static str,
    pub votes_output: &'static str,
}

impl OutputFiles {
    pub const fn new() -> Self {
        OutputFiles {
            article_output: ARTICLE_OUTPUT,
            tags_output: TAGS_OUTPUT,
            users_output: USERS_OUTPUT,
            votes_output: VOTES_OUTPUT,
        }
    }
}

impl Default for OutputFiles {
    fn default() -> Self {
        Self::new()
    }
}
