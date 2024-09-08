use const_format::formatcp;

// Directory where output files can be found
const OUTPUT_DIR: &str = "./output";

// Files to save scraped data to

const ARTICLE_OUTPUT: &str = formatcp!("{}/articles.parquet", OUTPUT_DIR);
const TAGS_OUTPUT: &str = formatcp!("{}/tags.parquet", OUTPUT_DIR);
const USERS_OUTPUT: &str = formatcp!("{}/users.parquet", OUTPUT_DIR);
const VOTES_OUTPUT: &str = formatcp!("{}/votes.parquet", OUTPUT_DIR);

#[derive(Clone, Debug)]
pub struct OutputFiles {
    pub article_output: String,
    pub tags_output: String,
    pub users_output: String,
    pub votes_output: String,
}

impl OutputFiles {
    pub fn new() -> Self {
        OutputFiles {
            article_output: String::from(ARTICLE_OUTPUT),
            tags_output: String::from(TAGS_OUTPUT),
            users_output: String::from(USERS_OUTPUT),
            votes_output: String::from(VOTES_OUTPUT),
        }
    }
}

impl Default for OutputFiles {
    fn default() -> Self {
        Self::new()
    }
}
