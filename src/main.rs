mod scraper;
mod console;

const VERSION: &str = "0.1.0";

fn main() {
    println!("Starting LOTUS...");

    console::main_console();

    println!("Shutting down LOTUS...")
}
