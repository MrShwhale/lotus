# LOTUS
A collaborative filtering recommendation system for the largest collaborative fiction project in the world.

This project has two main parts: 
- A web scraper, which gets information off of the SCP wiki, including the often overlooked rating information.
- And a web sever, which also runs a recommendation system, using the information gathered by the scraper.

# Building

To build the scraper, use `cargo build lotus_scrape --release`.

To build the web server, use `cargo build lotus_web --release`.

The `--release` argument is very important, otherwise the recommender will be too slow to provide a good user experience.

# Running the project
I have included sample output files from a scrape. These exist to allow users to run a basic version of the recommender immediately, and for testing purposes.
The scrape was done on September 12th 2024, so any actions taken on the wiki after that date will not be included.

**WARNING:** The scraper, by default, scrapes the FULL wiki. This means sending around *30000 web requests*!
It will take a while, and it will take bandwidth. Do not run this often to avoid unneeded strain on the SCP servers.

To run the scraper, use `cargo run lotus_scrape --release`.

To run the web server, use `cargo run lotus_web --release`.

## Command line arguments

## Using the project

In order to have the best user experience, this system needs to have up-to-date information about votes and articles on the wiki.
However, constantly scraping the SCP wiki would put a lot of strain on their servers, for little benefit (not much changes hour-to-hour).

So, it is suggested that a scrape is run every week (or few weeks), with the recommendation server being restarted after each scrape has completed.
Since it only takes a few seconds to start the server, this means only a few seconds of required server downtime every week.

A sample script, which would be paired with a weekly/monthly `cronjob`, is [included in this project](start_server.sh).

# About
The SCP wiki recently reached 20,000 articles, spanning just about every genre and level of quality.
Some of these include some of my favorite pieces of media, while others are clearly middle schoolers' first writings.
It can be hard to find things that you really like in this massive mixed bag, so in order to determine good from bad, the wiki knew they needed some kind of indication of quality.
Hopefully, this system would make it easier for users to find articles that they truly loved.

## The existing "recommendation" systems
The only objective indication of quality on the wiki is the "rating" system: each user can rate an article positively or negatively.
However, while negatively rated articles are generally bad, high-rating articles are variable due to the diversity of the wiki.
There are also many articles which are only highly rated due to there being outside media that caused a rating increase unrelated to the article at all.
This all comes together to show that while valuable, ratings are not enough to help users find articles they'll like.

## LOTUS
LOTUS seeks to help solve that issue. By having a more personalized, less popularity-influenced system of getting recommendations, users should be able to find articles they like more.
It should also help get articles with fewer votes due to their niche target audience be fully appreciated.

And if it doesn't end up working for people, at least it was fun to make (and gave me some cool articles to read).

## How does it work?
The recommendations come from a collaborative filtering system. Since who voted on each page is publicly available, by looking at this info across every article, similar users can be found.
Since similar users like similar things by definition, it is then as simple as find pages which the most similar users liked but the current user has not read. These are the recommendations.
This process has some additional steps, but that is the core idea behind the system.

## Why is it in Rust?
Speed is very important for a good user experience. If it takes even a few seconds to get a recommendation, the system will feel very unresponsive.

When I started this project, I wanted to do it in Python. This, despite being very well documented and easy to use, led to long wait times, many memory issues, and no obvious way forward for me.

So, I decided to use Rust for its speed and memory safety, especially across threads. I also think it's a lot of fun to write, so it was a great choice.

## Why LOTUS?
The name, LOTUS, is a reference to [SCP-6488](https://scp-wiki.wikidot.com/scp-6488), which is about an AI that destroys all other AI so that it doesn't become evil.
It's one of my favorites, and since it dealt with AI I thought it was a good fit.
