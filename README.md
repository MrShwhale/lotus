# LOTUS
A collaborative filtering recommendation system for the largest collaborative fiction project in the world.

**WARNING:** The scraper, by default, scrapes the FULL wiki. This means sending around *30000 web requests*. 
It will take a while, and it will take bandwith. Do not run this often to avoid unneeded strain on the SCP servers.

# Building

To build the scraper, use `cargo build lotus_scrape --release`.

To build the web server, use `cargo build lotus_web --release`.

The --release argument is very important, otherwise the recommender will be too slow to provide a good user experience.

# Running the project
I have included sample output files from a scrape. These exist to allow users to run a basic version of the recommender immediately, and for testing purposes.
The scrape was done on September 2nd 2024, so any actions taken on the wiki after that date will not be included.

To run the scraper, use `cargo run lotus_scrape --release`.

To run the web server, use `cargo run lotus_web --release`.

## Using the project

In order to have the best user experience, this system needs to have up-to-date information about votes and articles on the wiki.

However, constantly scraping the SCP wiki would put a lot of strain on their servers, for little benefit (not much changes hour-to-hour).

So, it is suggested that a scrape is run every week (or few weeks), with the recommendation server being restarted after each scrape has completed.

Since it only takes a few seconds to start the server, this means only a few seconds of required server downtime every week.

A sample script, which would be paired with a weekly/monthly cronjob, is [included in this project](start_server.sh).

# About
This is a project that comes from my love of SCPs, and a hope to learn more about data science.

This only works, by default, if you have more than 10 votes on the site. 
This helps the system run fast, and only provide results when you actually have something to collaborate with.

## Naming

The name, LOTUS, is a reference to SCP-6488, which is a super-powerful AI that detects other AI becoming evil.
