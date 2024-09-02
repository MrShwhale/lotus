pub mod scraper;

/// Message to print before any scraper logs
pub const SCRAPER_HEADING: &str = "[SCRAPER] ";

/// The main tag types. Any page not tagged with one of these will not be included in the scrape.
pub const TAG_TYPES: [&str; 4] = ["goi-format", "hub", "scp", "tale"];
