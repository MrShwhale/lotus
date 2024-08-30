use super::{Article, ScrapeInfo, User};
use arrow_array::{
    builder::{ListBuilder, UInt16Builder},
    ArrayRef, Int8Array, RecordBatch, StringArray, UInt64Array,
};
use arrow_schema::{DataType, Field, Schema};
use parquet::arrow::ArrowWriter;
use std::{collections::HashMap, fs::File, io::Error, sync::Arc};

// Message to print before all writer logs
const WRITER_HEADING: &str = "[WRITER] ";

// CONS adding schema description comments to each parquet save function

// Record all the scraped info to parquet files for fast, efficient access
pub fn record_info(
    scraped_info: ScrapeInfo,
    article_output: &str,
    tag_output: &str,
    user_output: &str,
    vote_output: &str,
) -> Result<(), Error> {
    eprintln!("{}Starting writing", WRITER_HEADING);
    record_articles_votes(scraped_info.articles, article_output, vote_output)?;
    record_users(scraped_info.users, user_output)?;
    record_tags(scraped_info.tags, tag_output)?;
    eprintln!("{}Writing completed successfully", WRITER_HEADING);
    Ok(())
}

fn record_articles_votes(
    articles: Vec<Article>,
    articles_output: &str,
    votes_output: &str,
) -> Result<(), Error> {
    let mut article_pids = Vec::with_capacity(articles.len());
    let mut names = Vec::with_capacity(articles.len());
    let mut tag_lists = Vec::with_capacity(articles.len());
    let mut urls = Vec::with_capacity(articles.len());

    // These are garunteed to be at least as long as articles, but will likely be longer. However
    // since we have idea how much longer, we use this lower bound.
    let mut ratings = Vec::with_capacity(articles.len());
    let mut vote_pids = Vec::with_capacity(articles.len());
    let mut uids = Vec::with_capacity(articles.len());

    // Article info must be deconstructed into vectors for both articles and votes
    eprintln!("{}Deconstructing articles", WRITER_HEADING);
    for article in articles {
        article_pids.push(article.page_id);
        names.push(article.name);
        tag_lists.push(article.tags);
        urls.push(article.url);

        for (rating, uid) in article.votes {
            ratings.push(rating);
            uids.push(uid);
            vote_pids.push(article.page_id);
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
    eprintln!("{}Recording articles", WRITER_HEADING);
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

    record_batch(schema, output_name, records)?;
    eprintln!("{}Articles recorded successfully", WRITER_HEADING);
    Ok(())
}

fn record_tags(tags: Vec<String>, output_name: &str) -> Result<(), Error> {
    eprintln!("{}Recording tags", WRITER_HEADING);
    let schema = Schema::new(vec![Field::new("tag", DataType::Utf8, false)]);

    let records: Vec<ArrayRef> = vec![Arc::new(StringArray::from(tags))];

    record_batch(schema, output_name, records)?;
    eprintln!("{}Tags recorded successfully", WRITER_HEADING);

    Ok(())
}

fn record_users(users: HashMap<u64, User>, output_name: &str) -> Result<(), Error> {
    eprintln!("{}Recording users", WRITER_HEADING);
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

    record_batch(schema, output_name, records)?;
    eprintln!("{}Users recorded successfully", WRITER_HEADING);
    Ok(())
}

fn record_votes(
    pids: Vec<u64>,
    uids: Vec<u64>,
    ratings: Vec<i8>,
    output_name: &str,
) -> Result<(), Error> {
    eprintln!("{}Recording votes", WRITER_HEADING);
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

    record_batch(schema, output_name, records)?;
    eprintln!("{}Votes recorded successfully", WRITER_HEADING);
    Ok(())
}

// Records information in a parquet file
fn record_batch(schema: Schema, file_name: &str, record_vec: Vec<ArrayRef>) -> Result<(), Error> {
    let mut buffer = File::create(file_name)?;
    let to_write = RecordBatch::try_new(Arc::new(schema), record_vec)
        .expect("Hardcoded schema should align with other hardcoded schema");

    let mut writer = ArrowWriter::try_new(&mut buffer, to_write.schema(), None)?;
    writer.write(&to_write)?;
    writer.close()?;

    Ok(())
}
