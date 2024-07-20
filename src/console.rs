use std::io;
use crate::scraper;

enum ConsoleResponse {
    FunctionSuccess,
    Incomprehensible,
    Quit,
}

/// Starts the main console, where everything else can be controlled
pub fn main_console() {
    // CONS pass this to avoid making 2-3 strings for user input?
    let mut input = String::new();

    let mut scp_scraper: scraper::Scraper = scraper::Scraper::new();

    loop {
        println!("LOTUS control panel");
        println!("1) Web scraper console");
        println!("2) Recommender system console");
        println!("3) Web server console");
        println!("4) About LOTUS");
        println!("5) Quit");

        io::stdin().read_line(&mut input).expect("Invalid string input.");

        match input.as_str().trim_end() {
            "1" => web_scraper_console(&mut scp_scraper),
            "2" => recommender_system_console(),
            "3" => web_server_console(),
            "4" => about(),
            "5" => break,
            _ => println!("I do not understand that")
        }

        input.clear();
    }
}

/// Web scraper text console, where actions relating to the web scraper can be taken
fn web_scraper_console(scp_scraper: &mut scraper::Scraper) {
    let mut input = String::new();

    loop {
        println!("Web scraper control panel");
        println!("1) Scrape");
        println!("2) Dry scrape");
        println!("3) Schedule scrape");
        println!("4) Quit");

        io::stdin().read_line(&mut input).expect("Invalid string input.");

        let result = match input.as_str().trim_end() {
            "1" => {
                match scp_scraper.scrape() {
                    Ok(_) => Ok(ConsoleResponse::FunctionSuccess),
                    Err(e) => Err(e),
                }
            },
            "2" => {
                match scp_scraper.dry_scrape() {
                    Ok(_) => Ok(ConsoleResponse::FunctionSuccess),
                    Err(e) => Err(e),
                }
            },
            "3" => {
                match scp_scraper.schedule_scrape() {
                    Ok(_) => Ok(ConsoleResponse::FunctionSuccess),
                    Err(e) => Err(e),
                }
            },
            "4" => Ok(ConsoleResponse::Quit),
            _ => Ok(ConsoleResponse::Incomprehensible)
        };

        match result {
            Ok(ConsoleResponse::FunctionSuccess) => {},
            Ok(ConsoleResponse::Quit) => break,
            Ok(ConsoleResponse::Incomprehensible) => println!("I do not understand that."),
            Err(_error) => todo!(),
        }

        input.clear();
    }
}

fn recommender_system_console() {

}

fn web_server_console() {

}

/// Print information about the install, as well as a short intro to the project
fn about() {
    // Consider adding time started to this
    println!("LOTUS Version: {}", crate::VERSION);
    println!("LOTUS is a recommender meant to make finding new SCP articles to read easier.");
    println!("It uses the upvotes from every user to find users similar to you,");
    println!("then finds pages they liked which you haven't read.\n");
}
