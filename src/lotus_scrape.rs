use lotus::{SCRAPER_HEADING, TAG_TYPES, scraper::Scraper};

fn main() {
    eprintln!("{}Scraping the wiki...", SCRAPER_HEADING);
    let scraper = Scraper::new();

    let result = scraper.scrape(usize::MAX, Vec::from(TAG_TYPES));

    match result {
        Ok(_) => eprintln!("{}Scrape completed successfully!", crate::SCRAPER_HEADING),
        Err(e) => {
            eprint!(
                "{}Something went wrong! Specifically, this: {:?}",
                SCRAPER_HEADING, e
            );
        }
    }
}
