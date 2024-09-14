use axum::{routing::get, Router};
use lotus_web::{
    recommender::{Recommender, RecommenderOptions},
    server, SERVER_HEADING,
};
use std::io::prelude::*;
use std::{env, fs::File, process, sync::Arc};
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    let mut options = RecommenderOptions::new();

    let mut index = 1;
    let length = args.len();

    while index < length {
        options = match args[index].as_str() {
            "--article-file" | "-a" => {
                let articles_file = args.get(index + 1).expect("No article file specified");
                index += 1;
                options.with_articles_file(articles_file.clone())
            }
            "--tags-file" | "-t" => {
                let tags_file = args.get(index + 1).expect("No tags file specified");
                index += 1;
                options.with_tags_file(tags_file.clone())
            }
            "--users-file" | "-u" => {
                let users_file = args.get(index + 1).expect("No users file specified");
                index += 1;
                options.with_users_file(users_file.clone())
            }
            "--votes-file" | "-v" => {
                let votes_file = args.get(index + 1).expect("No votes file specified");
                index += 1;
                options.with_votes_file(votes_file.clone())
            }
            "--min-votes" | "-m" => {
                let min_votes = args
                    .get(index + 1)
                    .expect("No minimum votes specified")
                    .parse()
                    .expect("Wrong format of min-votes. Must be a 16 bit unsigned integer.");
                index += 1;
                options.with_min_votes(min_votes)
            }
            "--users-to-consider" | "c" => {
                let users_to_consider = args
                    .get(index + 1)
                    .expect("No users to consider specified")
                    .parse()
                    .expect(
                        "Wrong format of users-to-consider. Must be a 32 bit unsigned integer.",
                    );
                index += 1;
                options.with_users_to_consider(users_to_consider)
            }
            "--help" | "-h" => {
                println!("Usage: lotus_web [args]\n  If an arg is passed multiple times, only the rightmost is considered.\n\n  Input file arguments:          Specify the location of different data files.\n    --article-file      or -a    Default: ./output/articles.parquet\n    --tags-file         or -t    Default: ./output/tags.parquet\n    --users-file        or -u    Default: ./output/users.parquet\n    --votes-file        or -v    Default: ./output/votes.parquet\n\n  Other options:\n      Sets the minimum number of votes required for a user to be included in recommender calculations.\n      Setting this higher reduces memory usage and speeds up recommendations, but any users with\n      fewer than this many votes will not be able to use the system, and their votes will not affect others.\n    --min-votes         or -m    Default: 10\n\n      Sets the number of similar users to consider when giving a recommendation.\n      Setting this higher gets a more diverse set of opinions, but adds more possibility of popularity bias.\n    --users-to-consider or -c    Default: 30\n\n      Display this message instead of running the system.\n    --help              or -h");
                return;
            }
            other => {
                println!(
                    "Unknown command line option: {}.\nRun with --help (or -h) for valid commands.",
                    other
                );
                process::exit(1);
            }
        };

        index += 1;
    }

    let recommender = match Recommender::new_with_options(&options) {
        Ok(rec) => rec,
        Err(e) => {
            eprintln!(
                "{}Recommender startup failed with error: {:?}",
                SERVER_HEADING, e
            );
            process::exit(1);
        }
    };

    eprintln!("{}Starting web server...", SERVER_HEADING);

    // Write some things to json files in the files folder
    let tags = recommender.get_tags();
    let tags = serde_json::to_string(&tags).expect("Tags should always be serializable");
    let mut tags_file = File::create("lotus_web/files/tags.json").unwrap();
    write!(tags_file, "{}", tags).unwrap();

    let usernames = recommender.get_users_list();
    let usernames =
        serde_json::to_string(&usernames).expect("Usernames should always be serializable");
    let mut usernames_file = File::create("lotus_web/files/usernames.json").unwrap();
    write!(usernames_file, "{}", usernames).unwrap();

    let serve_dir = ServeDir::new("lotus_web/files");

    let state = Arc::new(recommender);

    let app = Router::new()
        .route("/", get(server::root))
        .route("/rec", get(server::get_rec))
        .nest_service("/files", serve_dir)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Error starting listener");
    eprintln!("{}Web server up!", SERVER_HEADING);

    axum::serve(listener, app)
        .await
        .expect("Error while serving pages");
}
