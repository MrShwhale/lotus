use lotus::OutputFiles;
use lotus_scrape::{scraper::Scraper, SCRAPER_HEADING, TAG_TYPES};

fn main() {
    eprintln!("{}Scraping the wiki...", SCRAPER_HEADING);
    let scraper = Scraper::new();

    let result = scraper.scrape(usize::MAX, Vec::from(TAG_TYPES), OutputFiles::default());

    match result {
        Ok(_) => eprintln!("{}Scrape completed successfully!", SCRAPER_HEADING),
        Err(e) => {
            eprintln!(
                "{}Something went wrong! Specifically, this: {:?}",
                SCRAPER_HEADING, e
            );
        }
    }
}
