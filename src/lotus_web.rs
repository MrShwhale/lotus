mod lotus_core;
mod recommender;

use crate::recommender::Recommender;
use axum::{response::Html, routing::get, Router};
use lazy_static::lazy_static;
use polars::prelude::*;
use serde_json;
use std::collections::HashMap;

lazy_static! {
    static ref RECOMMENDER: recommender::Recommender = set_up_recommender();
}

static SERVER_HEADING: &str = "[SERVER] ";

#[tokio::main]
async fn main() {
    // Set up the recommender
    lazy_static::initialize(&RECOMMENDER);

    eprintln!("{}Starting web server...", SERVER_HEADING);

    let app = Router::new()
        .route("/", get(root))
        .route("/rec", get(get_rec));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    eprintln!("{}Web server up!", SERVER_HEADING);
    axum::serve(listener, app).await.unwrap();
}

fn set_up_recommender() -> Recommender {
    Recommender::new().unwrap()
}

// Display the homepage
// Having trouble with relative paths around this, can't really get it working. Might just move
// everything into this folder, but ideally wouldn't
async fn root() -> Html<&'static str> {
    Html(include_str!("../web/index.html"))
}

// Handles actually returning the recommendations
async fn get_rec(
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> String {
    // TODO change this from taking uid to taking username
    // CONS allow name, url, uid which will be checked against existing in that order

    println!("{:?}", params);

    let user_param = params.get("user");

    // Check if the uid exists
    let uid: u64 = if let Some(user_string) = user_param {
        // Look for the string in the database
        match RECOMMENDER.get_user_by_username(user_string) {
            Ok(user) => match user[2] {
                AnyValue::UInt64(uid) => uid,
                _ => unreachable!()
            },
            Err(_) => {
                // Check if this is is a raw uid
                match user_string.parse() {
                    Ok(value) => value,
                    Err(_) => return String::from(r#"{"type":"error","code":"USER_PARSE_ERROR"}"#),
                }
            }
        }
    } else {
        return String::from(r#"{"type":"error","code":"NO_USER"}"#);
    };

    // Actually get the recommendation now
    // TODO add bans, restrictions, etc
    let recs =
        match RECOMMENDER.get_recommendations_by_uid(uid, Vec::new(), Vec::new(), Vec::new()) {
            Ok(lf) => lf.collect(),
            Err(e) => {
                eprintln!("{:?}", e);
                panic!("This shouldn't happen");
            }
        };

    let recs = match recs {
        Ok(df) => df,
        Err(e) => {
            eprintln!("{:?}", e);
            panic!("This shouldn't happen");
        }
    };

    // CONS changing this to be some non-30 number/var
    let top_recs = recs.head(Some(30));
    let mut pages = Vec::new();

    for r in top_recs.column("pid").unwrap().iter() {
        let mut page = HashMap::new();
        match r {
            AnyValue::UInt64(pid) => match RECOMMENDER.get_page_by_pid(pid) {
                Ok(vec) => {
                    match vec[0] {
                        AnyValue::String(page_name) => page.insert(String::from("name"), String::from(page_name)),
                        _ => unreachable!(),
                    };

                    match vec[1] {
                        AnyValue::String(page_url) => page.insert(String::from("url"), String::from(page_url)),
                        _ => unreachable!(),
                    };
                    // CONS using the other 2 fields
                }
                Err(e) => {
                    eprintln!("{:?}", e);
                    panic!("This shouldn't happen");
                }
            },
            _ => unreachable!(),
        }

        pages.push(page);
    }

    serde_json::to_string(&pages).unwrap()
}
