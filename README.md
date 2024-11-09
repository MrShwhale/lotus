# LOTUS
A collaborative filtering recommendation system for the largest collaborative fiction project in the world.

This project has two main parts: 
- A web scraper, which gets information off of the SCP wiki, including the often overlooked rating information.
- And a web sever, which also runs a recommendation system, using the information gathered by the scraper.

# Running the project from the releases tab
Simply download the zip, unzip it, then run the programs.
File read/write permissions are required for both the scraper and the web server.

# Building from source

This project uses Rust version `1.82.0-nightly`.
A nightly build is required so that SIMD instructions can be used by external libraries to maximize performance.

To build the scraper, use `cargo build -p lotus_scrape --release`.

To build the web server, use `cargo build -p lotus_web --release`.

The `--release` argument is very important, otherwise the recommender will be too slow to provide a good user experience.

# Running the project from source
I have included sample output files from a scrape. These exist to allow users to run a basic version of the recommender immediately, and for testing purposes.
The scrape was done on September 12th 2024, so any actions taken on the wiki after that date will not be included.

**WARNING:** The scraper, by default, scrapes the FULL wiki. This means sending over *30000 web requests*!
It will take a while, and it will take bandwidth. Do not run this often to avoid unneeded strain on the SCP servers.

To run the scraper, use `cargo run -p lotus_scrape --release`.

To run the web server, use `cargo run -p lotus_web --release`.

The `--release` argument is very important, otherwise the recommender will be too slow to provide a good user experience.

These commands should be run from the `lotus` folder in the given folder structure.

## Command line arguments

Both programs have various command line arguments, which can be used to customize functionality easily.

The --help (or -h) arguments will print this information instead of running the program.

### Wiki Scraper
```
Usage: lotus_scrape [args]
  If an arg is passed multiple times, only the rightmost is considered.

  Output file arguments:           Specify the save location for different data.
    --article-file        or -a    Default: ./output/articles.parquet
    --tags-file           or -t    Default: ./output/tags.parquet
    --users-file          or -u    Default: ./output/users.parquet
    --votes-file          or -v    Default: ./output/votes.parquet

  Other options:
    Sets the number of articles to fetch from the wiki. Each article takes about 2 web requests to get.
    --article-limit       or -l    Default: maximum

    Sets the number of requests to make at one time (the number of additional threads to make).
    --concurrent-requests or -c    Default: 8

    Sets the additional approximate delay between requests, in milliseconds.
    This time is added in between each web request.
    --download-delay      or -d    Default: 0

    Display this message instead of running the system.
    --help                or -h
```

### Web Server
```
Usage: lotus_web [args]
  If an arg is passed multiple times, only the rightmost is considered.

  Output file arguments:           Specify the save location of different data.
    --article-file        or -a    Default: ./output/articles.parquet
    --tags-file           or -t    Default: ./output/tags.parquet
    --users-file          or -u    Default: ./output/users.parquet
    --votes-file          or -v    Default: ./output/votes.parquet

  Other options:
    Sets the ip address to listen for connections on, with the port specified.
    See the default for formatting example.
    --address           or -i    Default: 0.0.0.0:3000

    Sets the minimum number of votes each user must have to be included in the recommender.
    Setting this too low slows recommendation speed and uses a lot of memory.
    However, any users with less than this many votes will not be considered for recommendations.
    --min-votes         or -m    Default: 10

    Sets the number of similar users to consider for each recommendation.
    Setting this too high leads to more popularity bias and slightly slower recommendations.
    However, it also takes more user opinions into account, which potentially gives varied recommendations.
    --users-to-consider or -c    Default: 0

    Display this message instead of running the system.
    --help              or -h
```

## Using the project
In order to have the best user experience, this system needs to have up-to-date information about votes and articles on the wiki.
However, constantly scraping the SCP wiki would put a lot of strain on their servers, for little benefit (not much changes hour-to-hour).

So, it is suggested that a scrape is run every week (or few weeks), with the recommendation server being restarted after each scrape has completed.
Since it only takes a few seconds to start the server, this means only a few seconds of required server downtime every week.

A sample script, which would be paired with a weekly/monthly `cronjob`, is [included in this project](start_server.sh).

# About
The SCP wiki is the largest collaborative fiction project in the world, recently reaching 20,000 pages, spanning just about every genre and level of quality.
Some of these are my favorite pieces of media, while others are clearly middle schoolers' first writings.
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
It's one of my favorites, and since both it and this deal with AI I think it is a good fit.

# Sources
Lotus svg (recolored from original): https://commons.wikimedia.org/wiki/File:Lotus.svg

Tags icon (recolored from original): https://www.svgrepo.com/svg/391706/tags

Search icon (recolored from original): https://www.svgrepo.com/svg/260047/magnifying-glass-search

Background image (black gradient added): https://www.hdwallpapers.in/red_fractal_swirling_hd_trippy-wallpapers.html
