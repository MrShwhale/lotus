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

    let mut ip = "0.0.0.0:3000";

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
            "--address" | "-i" => {
                ip = args.get(index + 1).expect("No users to consider specified");
                index += 1;
                options
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
            "--users-to-consider" | "-c" => {
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
                println!("Usage: lotus_web [args]\n  If an arg is passed multiple times, only the rightmost is considered.\n\n  Output file arguments:           Specify the save location of different data.\n    --article-file        or -a    Default: .outputarticles.parquet\n    --tags-file           or -t    Default: .outputtags.parquet\n    --users-file          or -u    Default: .outputusers.parquet\n    --votes-file          or -v    Default: .outputvotes.parquet\n\n  Other options:\n    Sets the ip address to listen for connections on, with the port specified.\n    See the default for formatting example.\n    --address           or -i    Default: 0.0.0.0:3000\n\n    Sets the minimum number of votes each user must have to be included in the recommender.\n    Setting this too low slows recommendation speed and uses a lot of memory.\n    However, any users with less than this many votes will not be considered for recommendations.\n    --min-votes         or -m    Default: 10\n\n    Sets the number of similar users to consider for each recommendation.\n    Setting this too high leads to more popularity bias and slightly slower recommendations.\n    However, it also takes more user opinions into account, which potentially gives varied recommendations.\n    --users-to-consider or -c    Default: 0\n\n    Display this message instead of running the system.\n    --help              or -h");
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

    let listener = tokio::net::TcpListener::bind(ip)
        .await
        .expect("Error starting listener");
    eprintln!("{}Web server listening on port: {}", SERVER_HEADING, ip);

    axum::serve(listener, app)
        .await
        .expect("Error while serving pages");
}
