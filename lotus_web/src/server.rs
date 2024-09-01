use crate::{recommender::Recommender, SERVER_HEADING};
use askama_axum::Template;
use axum;
use lazy_static::lazy_static;
use polars::prelude::*;
use serde::Serialize;
use serde_json;
use std::collections::HashMap;
use urlencoding;

lazy_static! {
    pub static ref RECOMMENDER: Recommender = Recommender::new().unwrap();
}

#[derive(Serialize)]
struct Recommendation {
    name: String,
    url: String,
    tags: Vec<String>,
    pid: u64,
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct RootTemplate {
    tags: String,
}

/// Display the homepage
pub async fn root() -> RootTemplate {
    let tags = RECOMMENDER.get_tags();
    let tags = serde_json::to_string(&tags).expect("");

    RootTemplate { tags }
}

/// Returns a list of recommendations in JSON format with the given params
pub async fn get_rec(
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> String {
    eprintln!(
        "{}Recommendation request with params: {:?}",
        SERVER_HEADING, params
    );

    let ban_param = params.get("bans");
    let tag_param = params.get("tags");
    let user_param = params.get("user");

    // Check if the uid exists
    let uid: u64 = if let Some(user_string) = user_param {
        // Check if this a name
        match RECOMMENDER.get_user_by_username(user_string) {
            Ok(user) => match user[2] {
                AnyValue::UInt64(uid) => uid,
                _ => unreachable!(),
            },
            Err(_) => {
                // Check if this is is a raw uid as opposed to a username
                match user_string.parse() {
                    Ok(value) => value,
                    Err(_) => return String::from(r#"{"type":"error","code":"USER_PARSE_ERROR"}"#),
                }
            }
        }
    } else {
        return String::from(r#"{"type":"error","code":"NO_USER"}"#);
    };

    let tags: Vec<u16> = if let Some(tag_string) = tag_param {
        tag_string
            .split("+")
            .map(|tag| tag.parse().unwrap())
            .collect()
    } else {
        Vec::new()
    };

    let bans: Vec<u64> = if let Some(ban_string) = ban_param {
        urlencoding::decode(ban_string)
            .unwrap()
            .split_whitespace()
            .map(|ban| ban.parse().unwrap())
            .collect()
    } else {
        Vec::new()
    };

    eprintln!("{}Bans: {:?}", SERVER_HEADING, bans);

    let recs = match RECOMMENDER.get_recommendations_by_uid(uid, tags, bans) {
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

    let top_recs = recs.head(Some(500));

    recs_to_string(top_recs)
}

fn recs_to_string(full_recs: DataFrame) -> String {
    let pages: Vec<_> = full_recs
        .column("pid")
        .expect("pid column should always exist")
        .u64()
        .expect("pids should all be u64")
        .iter()
        .map(
            |pid| match RECOMMENDER.get_page_by_pid(pid.expect("pids should all be Some")) {
                Ok(vec) => Recommendation {
                    name: match &vec[0] {
                        AnyValue::String(page_name) => String::from(*page_name),
                        _ => unreachable!(),
                    },
                    url: match &vec[1] {
                        AnyValue::String(page_url) => String::from(*page_url),
                        _ => unreachable!(),
                    },
                    pid: match &vec[2] {
                        AnyValue::UInt64(page_id) => *page_id,
                        _ => unreachable!(),
                    },
                    tags: match &vec[3] {
                        AnyValue::List(page_tags) => page_tags
                            .u16()
                            .expect("Tags should all be u16")
                            .iter()
                            .map(|tag| {
                                RECOMMENDER
                                    .get_tag_by_id(tag.expect("Tags should all be Some"))
                                    .unwrap()
                            })
                            .collect(),
                        _ => unreachable!(),
                    },
                },
                Err(e) => {
                    eprintln!("{:?}", e);
                    panic!("This shouldn't happen");
                }
            },
        )
        .collect();

    serde_json::to_string(&pages).expect("Page vec should be serializeable")
}