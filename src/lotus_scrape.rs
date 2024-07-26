mod scraper;
mod lotus_core;

use scraper::Scraper;

const TAG_TYPES: [&str; 4] = ["scp", "tale", "hub", "goi-format"];

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
