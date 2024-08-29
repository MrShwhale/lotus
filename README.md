# LOTUS
A collaborative filtering recommendation system for the largest collaborative fiction project in the world.

**WARNING:** The scraper, by default, scrapes the FULL wiki. This means sending around *30000 web requests*. 
It will take a while, and it will take bandwith. Do not run this often to avoid unneeded strain on the SCP servers.

To run the scraper, use "cargo run lotus_scrape"

To run the web server, use "cargo run"

# About
This is a project that comes from my love of SCPs, and a hope to learn more about data science.

This only works, by default, if you have more than 10 votes on the site. 
This helps the system run fast, and only provide results when you actually have something to collaborate with.

The name, LOTUS, is a reference to SCP-6488, which is a super-powerful AI that detects other AI becoming evil.
