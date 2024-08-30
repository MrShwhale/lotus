use crate::recommender::Recommender;
use lazy_static::lazy_static;
use serde_json;
use std::collections::HashMap;
use polars::prelude::*;
use urlencoding;
use axum;
use askama_axum::Template;
use serde::Serialize;

lazy_static! {
    pub static ref RECOMMENDER: Recommender = set_up_recommender();
}

// CONS deleting this, it is basically Article but no votes
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

pub fn set_up_recommender() -> Recommender {
    Recommender::new().unwrap()
}

// Display the homepage
pub async fn root() -> RootTemplate {
    let tags = RECOMMENDER.get_tags();
    let tags = serde_json::to_string(&tags).unwrap();

    RootTemplate { tags }
}

// Handles actually returning the recommendations
pub async fn get_rec(
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> String {
    eprintln!(
        "{}Recommendation request with params: {:?}",
        crate::SERVER_HEADING, params
    );

    let user_param = params.get("user");
    let tag_param = params.get("tags");
    let ban_param = params.get("bans");

    // Check if the uid exists
    let uid: u64 = if let Some(user_string) = user_param {
        // Look for the string in the database
        match RECOMMENDER.get_user_by_username(user_string) {
            Ok(user) => match user[2] {
                AnyValue::UInt64(uid) => uid,
                _ => unreachable!(),
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

    // Try to create a list of required tags from the param
    let tags: Vec<u16> = if let Some(tag_string) = tag_param {
        tag_string
            .split("+")
            .map(|tag| tag.parse().unwrap())
            .collect()
    } else {
        Vec::new()
    };

    // Try to create a list of bans from the param
    let bans: Vec<u64> = if let Some(ban_string) = ban_param {
        urlencoding::decode(ban_string)
            .unwrap()
            .split_whitespace()
            .map(|ban| ban.parse().unwrap())
            .collect()
    } else {
        Vec::new()
    };

    eprintln!("{}Bans: {:?}", crate::SERVER_HEADING, bans);

    // Try to create a list of banned pages from the param
    let bans: Vec<u64> = if let Some(ban_string) = ban_param {
        urlencoding::decode(ban_string)
            .unwrap()
            .split_whitespace()
            .map(|ban| ban.parse().unwrap())
            .collect()
    } else {
        Vec::new()
    };

    // Actually get the recommendation now
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

    // Return the 500 best recommendations
    let top_recs = recs.head(Some(500));
    let mut pages = Vec::new();

    for r in top_recs.column("pid").unwrap().iter() {
        let rec: Recommendation;
        match r {
            AnyValue::UInt64(pid) => match RECOMMENDER.get_page_by_pid(pid) {
                Ok(vec) => {
                    rec = Recommendation {
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
                                .iter()
                                .map(|value| match value {
                                    AnyValue::UInt16(tag) => {
                                        RECOMMENDER.get_tag_by_id(tag).unwrap()
                                    }
                                    _ => unreachable!(),
                                })
                                .collect(),
                            _ => unreachable!(),
                        },
                    }
                }
                Err(e) => {
                    eprintln!("{:?}", e);
                    panic!("This shouldn't happen");
                }
            },
            _ => unreachable!(),
        }

        pages.push(rec);
    }

    serde_json::to_string(&pages).unwrap()
}
