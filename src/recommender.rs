use polars::prelude::*;
use polars::df;
use crate::scraper::{ARTICLE_OUTPUT, TAGS_OUTPUT, USERS_OUTPUT, VOTES_OUTPUT};

#[derive(Debug)]
pub enum RecommenderError {
    PolarsError(PolarsError),
}

impl From<PolarsError> for RecommenderError {
    fn from(value: PolarsError) -> Self {
        RecommenderError::PolarsError(value)
    }
}

pub struct Recommender {
    user_frame: DataFrame,
    page_frame: DataFrame,
    rating_frame: DataFrame,
}

impl Recommender {
    /// Creates a new recommender.
    /// Assumes the default web scraper names/locations of files
    pub fn new() -> Result<Recommender, RecommenderError> {
        let user_frame = set_up_user_frame(USERS_OUTPUT)?;
        let page_frame = set_up_page_frame(ARTICLE_OUTPUT)?;
        let tags_frame = set_up_tags_frame(TAGS_OUTPUT)?;
        let rating_frame = set_up_rating_frame(VOTES_OUTPUT)?;

        let rating_frame = pivot::pivot(&rating_frame, "uid", Some(["pid"]), Some(["rating"]), false, None, None);

        let recommender = Recommender {
            user_frame,
            page_frame,
            rating_frame,
        };

        // Ok(recommender)
        todo!();
    }
}

fn set_up_user_frame(user_file: &str) -> Result<DataFrame, RecommenderError> {
    let file = std::fs::File::open(user_file).unwrap();
    let user_df = ParquetReader::new(file).finish().unwrap();

    Ok(user_df)
}

fn set_up_page_frame(page_file: &str) -> Result<DataFrame, RecommenderError> {
    let file = std::fs::File::open(page_file).unwrap();
    let page_df = ParquetReader::new(file).finish().unwrap();

    Ok(page_df)
}

fn set_up_rating_frame(rating_file: &str) -> Result<DataFrame, RecommenderError> {
    let file = std::fs::File::open(rating_file).unwrap();
    let rating_df = ParquetReader::new(file).finish().unwrap();

    println!("{}", rating_df);

    Ok(rating_df)
}

fn set_up_tags_frame(tags_file: &str) -> Result<DataFrame, RecommenderError> {
    let file = std::fs::File::open(tags_file).unwrap();
    let tags_df = ParquetReader::new(file).finish().unwrap();

    Ok(tags_df)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_user_frame() {
        set_up_user_frame(USERS_OUTPUT).unwrap();
    }

    #[test]
    fn make_page_frame() {
        set_up_page_frame(ARTICLE_OUTPUT).unwrap();
    }

    #[test]
    fn make_rating_frame() {
        set_up_rating_frame(VOTES_OUTPUT).unwrap();
    }

    #[test]
    fn make_tags_frame() {
        set_up_tags_frame(TAGS_OUTPUT).unwrap();
    }
}
