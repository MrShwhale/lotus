use arrow_array::{
    builder::{ListBuilder, UInt16Builder},
    Int8Array, RecordBatch, StringArray, UInt16Array, UInt64Array,
};
use arrow_schema::{DataType, Field, Schema};
use const_format::formatcp;
use parquet::arrow::ArrowWriter;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::Error,
    sync::Arc,
};

use super::{Article, ScrapeInfo, User};

// CONS make these fully compile time instead of building them each write
pub const OUTPUT_DIR: &str = "./output";

pub const ARTICLE_OUTPUT: &str = formatcp!("{}/articles.parquet", OUTPUT_DIR);

pub const TAGS_OUTPUT: &str = formatcp!("{}/tags.parquet", OUTPUT_DIR);

pub const USERS_OUTPUT: &str = formatcp!("{}/users.parquet", OUTPUT_DIR);

pub const VOTES_OUTPUT: &str = formatcp!("{}/votes.parquet", OUTPUT_DIR);

// TODO Error checking
// TODO Extract common code
pub fn record_info(scraped_info: ScrapeInfo) -> Result<(), Error> {
    // Ensures there is always a folder to output to
    fs::create_dir_all(OUTPUT_DIR)?;

    // Save the user information as a parquet
    record_users(scraped_info.users).unwrap();

    // Tags
    record_tags(scraped_info.tags).unwrap();

    // TODO Votes & Articles
    record_articles_votes(scraped_info.articles).unwrap();

    Ok(())
}

fn record_users(users: HashMap<u64, User>) -> Result<(), Error> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("name", DataType::Utf8, false),
        Field::new("url", DataType::Utf8, false),
        Field::new("uid", DataType::UInt64, false),
    ]));

    let mut name: Vec<String> = Vec::with_capacity(users.len());
    let mut url: Vec<String> = Vec::with_capacity(users.len());
    let mut user_id: Vec<u64> = Vec::with_capacity(users.len());

    for user in users.into_values() {
        name.push(user.name);
        url.push(user.url);
        user_id.push(user.user_id);
    }

    let mut buffer = File::create(USERS_OUTPUT)?;
    let to_write = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(name)),
            Arc::new(StringArray::from(url)),
            Arc::new(UInt64Array::from(user_id)),
        ],
    )
    .unwrap();

    let mut writer = ArrowWriter::try_new(&mut buffer, to_write.schema(), None).unwrap();
    writer.write(&to_write).unwrap();
    writer.close().unwrap();

    Ok(())
}

fn record_tags(tags: Vec<String>) -> Result<(), Error> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("index", DataType::UInt16, false),
        Field::new("tag", DataType::Utf8, false),
    ]));

    let indicies: Vec<u16> = Vec::from_iter(
        0..u16::try_from(tags.len()).expect("Shouldn't have more tags than u16 range."),
    );

    let mut buffer = File::create(TAGS_OUTPUT)?;
    let to_write = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(UInt16Array::from(indicies)),
            Arc::new(StringArray::from(tags)),
        ],
    )
    .unwrap();

    let mut writer = ArrowWriter::try_new(&mut buffer, to_write.schema(), None).unwrap();
    writer.write(&to_write).unwrap();
    writer.close().unwrap();

    Ok(())
}

fn record_articles_votes(articles: Vec<Article>) -> Result<(), Error> {
    // Article vecs
    let mut names: Vec<_> = Vec::new();
    let mut urls: Vec<_> = Vec::new();
    let mut article_pids: Vec<_> = Vec::new();
    let mut tag_lists: Vec<_> = Vec::new();

    // Vote vecs
    let mut vote_pids: Vec<_> = Vec::new();
    let mut uids: Vec<_> = Vec::new();
    let mut ratings: Vec<_> = Vec::new();

    // TODO this algorithm makes you want to cry
    // Google iterator, I'm begging
    for article in articles {
        // Read the vote info into the vecs
        for (rating, uid) in article.votes {
            vote_pids.push(article.page_id);
            uids.push(uid);
            ratings.push(rating);
        }

        // Add the article info to the vecs
        names.push(article.name);
        urls.push(article.url);
        article_pids.push(article.page_id);
        tag_lists.push(article.tags);
    }

    record_votes(vote_pids, uids, ratings).unwrap();
    record_articles(names, urls, article_pids, tag_lists).unwrap();

    Ok(())
}

fn record_votes(pids: Vec<u64>, uids: Vec<u64>, ratings: Vec<i8>) -> Result<(), Error> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("pid", DataType::UInt64, false),
        Field::new("uid", DataType::UInt64, false),
        Field::new("rating", DataType::Int8, false),
    ]));

    let mut buffer = File::create(VOTES_OUTPUT)?;
    let to_write = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(UInt64Array::from(pids)),
            Arc::new(UInt64Array::from(uids)),
            Arc::new(Int8Array::from(ratings)),
        ],
    )
    .unwrap();

    let mut writer = ArrowWriter::try_new(&mut buffer, to_write.schema(), None).unwrap();
    writer.write(&to_write).unwrap();
    writer.close().unwrap();

    Ok(())
}

fn record_articles(
    names: Vec<String>,
    urls: Vec<String>,
    pids: Vec<u64>,
    tag_lists: Vec<Vec<u16>>,
) -> Result<(), Error> {
    let tag_field = Field::new("tags", DataType::List(Arc::new(Field::new("item", DataType::UInt16, false))), false);
    let schema = Arc::new(Schema::new(vec![
        Field::new("name", DataType::Utf8, false),
        Field::new("url", DataType::Utf8, false),
        Field::new("pid", DataType::UInt64, false),
        tag_field
    ]));

    let mut builder = ListBuilder::new(UInt16Builder::new());
    for list in tag_lists {
        builder.values().append_slice(list.as_slice());
        builder.append(true);
    }

    builder = builder.with_field(Field::new("item", DataType::UInt16, false));

    let mut buffer = File::create(ARTICLE_OUTPUT)?;
    let to_write = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(names)),
            Arc::new(StringArray::from(urls)),
            Arc::new(UInt64Array::from(pids)),
            Arc::new(builder.finish()),
        ],
    )
    .unwrap();

    let mut writer = ArrowWriter::try_new(&mut buffer, to_write.schema(), None).unwrap();
    writer.write(&to_write).unwrap();
    writer.close().unwrap();

    Ok(())
}
