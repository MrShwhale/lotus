use super::{Article, ScrapeInfo, User};
use crate::{ARTICLE_OUTPUT, OUTPUT_DIR, TAGS_OUTPUT, USERS_OUTPUT, VOTES_OUTPUT};
use arrow_array::{
    builder::{ListBuilder, UInt16Builder},
    ArrayRef, Int8Array, RecordBatch, StringArray, UInt64Array,
};
use arrow_schema::{DataType, Field, Schema};
use parquet::arrow::ArrowWriter;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::Error,
    sync::Arc,
};

pub fn record_info(scraped_info: ScrapeInfo) -> Result<(), Error> {
    // Ensure there is always a folder to output to
    fs::create_dir_all(OUTPUT_DIR)?;

    // Save the user information as a parquet file
    record_users(scraped_info.users, USERS_OUTPUT)?;

    // Tags
    record_tags(scraped_info.tags, TAGS_OUTPUT)?;

    // Articles and votes
    record_articles_votes(scraped_info.articles, ARTICLE_OUTPUT, VOTES_OUTPUT)?;

    Ok(())
}

fn record_users(users: HashMap<u64, User>, output_name: &str) -> Result<(), Error> {
    let schema = Schema::new(vec![
        Field::new("name", DataType::Utf8, false),
        Field::new("url", DataType::Utf8, false),
        Field::new("uid", DataType::UInt64, false),
    ]);

    let mut name = Vec::with_capacity(users.len());
    let mut url = Vec::with_capacity(users.len());
    let mut user_id = Vec::with_capacity(users.len());

    for user in users.into_values() {
        name.push(user.name);
        url.push(user.url);
        user_id.push(user.user_id);
    }

    let records: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(name)),
        Arc::new(StringArray::from(url)),
        Arc::new(UInt64Array::from(user_id)),
    ];

    record_batch(schema, output_name, records)
}

fn record_tags(tags: Vec<String>, output_name: &str) -> Result<(), Error> {
    let schema = Schema::new(vec![Field::new("tag", DataType::Utf8, false)]);

    let records: Vec<ArrayRef> = vec![Arc::new(StringArray::from(tags))];

    record_batch(schema, output_name, records)
}

fn record_articles_votes(
    articles: Vec<Article>,
    articles_output: &str,
    votes_output: &str,
) -> Result<(), Error> {
    // Article vecs
    let mut names = Vec::with_capacity(articles.len());
    let mut urls = Vec::with_capacity(articles.len());
    let mut article_pids = Vec::with_capacity(articles.len());
    let mut tag_lists = Vec::with_capacity(articles.len());

    // Vote vecs
    // These are garunteed to be at least as long as articles, but will likely be longer. However
    // since we have no idea how much longer, we go with this lower bound.
    let mut vote_pids = Vec::with_capacity(articles.len());
    let mut uids = Vec::with_capacity(articles.len());
    let mut ratings = Vec::with_capacity(articles.len());

    for article in articles {
        // Add the article info to the vecs
        names.push(article.name);
        urls.push(article.url);
        article_pids.push(article.page_id);
        tag_lists.push(article.tags);

        // Read the vote info into the vecs
        for (rating, uid) in article.votes {
            vote_pids.push(article.page_id);
            uids.push(uid);
            ratings.push(rating);
        }
    }

    record_articles(names, urls, article_pids, tag_lists, articles_output)?;
    record_votes(vote_pids, uids, ratings, votes_output)?;

    Ok(())
}

fn record_articles(
    names: Vec<String>,
    urls: Vec<String>,
    pids: Vec<u64>,
    tag_lists: Vec<Vec<u16>>,
    output_name: &str,
) -> Result<(), Error> {
    let tag_field = Field::new(
        "tags",
        DataType::List(Arc::new(Field::new("item", DataType::UInt16, false))),
        false,
    );

    let schema = Schema::new(vec![
        Field::new("name", DataType::Utf8, false),
        Field::new("url", DataType::Utf8, false),
        Field::new("pid", DataType::UInt64, false),
        tag_field,
    ]);

    let mut builder = ListBuilder::new(UInt16Builder::new());
    for list in tag_lists {
        builder.values().append_slice(list.as_slice());
        builder.append(true);
    }

    builder = builder.with_field(Field::new("item", DataType::UInt16, false));

    let records: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(names)),
        Arc::new(StringArray::from(urls)),
        Arc::new(UInt64Array::from(pids)),
        Arc::new(builder.finish()),
    ];

    record_batch(schema, output_name, records)
}

fn record_votes(
    pids: Vec<u64>,
    uids: Vec<u64>,
    ratings: Vec<i8>,
    output_name: &str,
) -> Result<(), Error> {
    let schema = Schema::new(vec![
        Field::new("pid", DataType::UInt64, false),
        Field::new("uid", DataType::UInt64, false),
        Field::new("rating", DataType::Int8, false),
    ]);

    let records: Vec<ArrayRef> = vec![
        Arc::new(UInt64Array::from(pids)),
        Arc::new(UInt64Array::from(uids)),
        Arc::new(Int8Array::from(ratings)),
    ];

    record_batch(schema, output_name, records)
}

fn record_batch(schema: Schema, file_name: &str, record_vec: Vec<ArrayRef>) -> Result<(), Error> {
    let mut buffer = File::create(file_name)?;
    let to_write = RecordBatch::try_new(Arc::new(schema), record_vec)
        .expect("Hardcoded schema should align with other hardcoded");

    let mut writer = ArrowWriter::try_new(&mut buffer, to_write.schema(), None)?;
    writer.write(&to_write)?;
    writer.close()?;

    Ok(())
}
