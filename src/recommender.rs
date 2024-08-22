use super::lotus_core::{ARTICLE_OUTPUT, TAGS_OUTPUT, USERS_OUTPUT, VOTES_OUTPUT};
use core::panic;
use pivot;
use polars::prelude::*;
use polars_core::utils::Container;
use polars_lazy::{dsl::col, prelude::*};
use std::collections::HashMap;

// Might be too high?
const SIMILAR_TO_USE: u32 = 30;

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
    rating_frame: LazyFrame,
    tags_frame: DataFrame,
    middle_norms: HashMap<String, f64>,
}

// TODO this WHOLE FILE needs error handling
// TODO use the concurrent versions of everything
impl Recommender {
    /// Creates a new recommender.
    /// Assumes the default web scraper names/locations of files
    pub fn new(min_votes: u16) -> Result<Recommender, RecommenderError> {
        let user_frame = set_up_user_frame(USERS_OUTPUT)?;
        let page_frame = set_up_page_frame(ARTICLE_OUTPUT)?;
        let rating_frame = set_up_rating_frame(VOTES_OUTPUT)?;

        // Cast the i8 values (used for minimizing save) into the usable f64
        let mut casts = PlHashMap::new();
        casts.insert("rating", DataType::Float32);
        let rating_frame = rating_frame.cast(casts, true);
        eprintln!("Frames made");

        // TODO this could probably be optimized
        // Count votes from each user
        let selected_users = rating_frame
            .clone()
            .group_by(["uid"])
            .agg([col("rating").count().alias("rating_count")])
            .filter(col("rating_count").gt_eq(lit(min_votes)))
            .collect_concurrently()
            .unwrap()
            .fetch_blocking()
            .unwrap();

        // Cut out all ratings by who have fewer than min_votes votes
        let rating_frame =
            rating_frame.filter(col("uid").is_in(lit(selected_users["uid"].clone())));
        eprintln!("Irrelevant users discarded.");

        // Rating frame has the columns as the users (since that is what operations are done by)
        // and the rows as the pages. The intersections of them is the rating that user gave that
        // page, or 0 if it is unrated.

        let mut casts = PlHashMap::new();
        casts.insert("pid", DataType::UInt64);

        let rating_frame = pivot::pivot_stable(
            &rating_frame.collect().unwrap(),
            ["uid"],
            Some(["pid"]),
            Some(["rating"]),
            false,
            None,
            None,
        )
        .unwrap()
        .lazy()
        .fill_null(0f64)
        .cast(casts, true);

        eprintln!("Pivoted");

        let tags_frame = set_up_tags_frame(TAGS_OUTPUT)?;

        // TODO reorg this
        let mut recommender = Recommender {
            user_frame,
            page_frame,
            rating_frame,
            tags_frame,
            middle_norms: HashMap::new(),
        };

        // Normalize data
        recommender.normalize_rating_frame();
        eprintln!("Normalized");

        Ok(recommender)
    }

    // BUG uid is a float by the end of this, could happen before
    // Also, WOW this is slow
    fn normalize_rating_frame(&mut self) {
        // SCP users have this nasty habit of being positive people.
        // This means that they mostly only upvote, which causes issues since then the average for many rows is 0
        // This could be a critical issue but if you knew what you were doing you wouldn't be here at all
        // So, change the average rating to be the mean but with a single 0 rating added
        // Then, properly normalize it

        let mut frame = self.rating_frame.clone().collect().unwrap();
        let mut z_vec = Vec::with_capacity(frame.width());
        z_vec.push(Series::new("pid", [0u64].as_ref()));
        for whale in frame.get_column_names().iter().skip(1) {
            z_vec.push(Series::new(whale, [0f32].as_ref()))
        }

        let z_frame = DataFrame::new(z_vec).unwrap();
        frame.vstack_mut(&z_frame).unwrap();

        let all_but_pid = col("*").exclude(["pid"]);
        let centered_adjusted =
            all_but_pid.clone() - (all_but_pid.clone().sum() / all_but_pid.clone().len());
        let normalized = centered_adjusted.clone() / centered_adjusted.clone().pow(2).sum().sqrt();
        frame = frame
            .lazy()
            .select([col("pid"), normalized])
            .collect()
            .unwrap();

        let first = frame.get(0).unwrap();

        for (i, whale) in frame.get_column_names().iter().skip(1).enumerate() {
            let value = match first[i + 1] {
                AnyValue::Float64(val) => val,
                _ => panic!("Bruch"),
            };

            self.middle_norms.insert(String::from(*whale), value);
        }

        let frame = frame.slice(0, frame.len() - 1);

        self.rating_frame = frame.lazy();
    }

    // Returns the Series representing the given page using the page dataframe
    pub fn get_page_by_pid(&self, pid: u64) -> Vec<AnyValue> {
        // CONS this might be the worst?
        let pids = self.page_frame.column("pid").unwrap();
        let mut index = 0;
        loop {
            let extracted = match pids.get(index).unwrap() {
                AnyValue::UInt64(value) => value,
                _ => unreachable!(),
            };

            if extracted == pid {
                break;
            }

            index += 1;
        }

        self.page_frame.get(index).unwrap()
    }

    pub fn get_recommendations_by_uid(
        &self,
        uid: u64,
        banned_tags: Vec<String>,
        external_bans: Vec<u64>,
    ) -> LazyFrame {
        let user_similarity = self.get_user_similarity(uid);
        let user_similarity = user_similarity.sort(
            ["similarity"],
            SortMultipleOptions::new().with_order_descending(true),
        );

        // Drop all users which have a similarity of about 1
        let user_similarity = user_similarity.filter(col("similarity").lt(lit(0.999f64)));

        // Get the most similar non-exact-copy users
        let user_similarity = user_similarity
            .clone()
            .limit(SIMILAR_TO_USE)
            .collect()
            .unwrap();

        // Get the list of pages which the most similar users have read
        let similar_cols: Vec<_> = user_similarity["uid"]
            .iter()
            .map(|a| {
                let a_string = a.to_string();
                col(&a_string[1..a_string.len() - 1])
            })
            .collect();

        let similar_users = self.rating_frame.clone().select(similar_cols);

        // similar_users has the 10 uids of the most similar users
        // Against their ratings for every page
        // Multiply each column by the user similarity for that column
        let similar_users = similar_users.collect().unwrap();

        let similar_uids = user_similarity.column("uid").unwrap();
        let similar_weights = user_similarity.column("similarity").unwrap();

        // Create mapping of uid to similarity
        let mut similarity_map: HashMap<&str, f64> = HashMap::with_capacity(
            SIMILAR_TO_USE
                .try_into()
                .expect("u32 to usize should be safe. Maybe use smaller SIMILAR_TO_USE?"),
        );

        for i in 0..similar_uids.len() {
            let string = match similar_uids.get(i).unwrap() {
                AnyValue::String(val) => val,
                _ => panic!("bruh :("),
            };

            let weight = match similar_weights.get(i).unwrap() {
                AnyValue::Float64(val) => val,
                _ => panic!("bruh 2 :("),
            };

            similarity_map.insert(string, weight);
        }

        let similarity_selector: Vec<_> = similar_users
            .get_column_names()
            .iter()
            .map(|col_name| col(*col_name) * lit(similarity_map.get(*col_name).unwrap().clone()))
            .collect();

        let page_weights = self.rating_frame.clone().select(similarity_selector);

        // Sum all columns together
        let mut page_weights = page_weights
            .collect()
            .unwrap()
            .sum_horizontal(polars::frame::NullStrategy::Ignore)
            .unwrap()
            .unwrap();
        page_weights.rename("weights");

        // TODO fix this bro :(
        let pids = self
            .rating_frame
            .clone()
            .collect()
            .unwrap()
            .column("pid")
            .unwrap()
            .clone();

        // Create series made of all read pages
        let uid_col = col(format!("{}", uid).as_str());
        let f_uncert = lit(1e-3f64);
        let uid_unvote = lit(self
            .middle_norms
            .get(&format!("{}", uid))
            .unwrap()
            .to_owned());

        // BUG this is not working: things I have upvoted (and maybe downvoted) are showing up
        // Filter column
        let ignored_pages = uid_col
            .clone()
            // Check if the user has upvoted already
            .gt(uid_unvote.clone() + f_uncert.clone())
            // Check if the user has downvoted already
            .or(uid_col.clone().lt(uid_unvote.clone() - f_uncert.clone()))
            // TODO Check for the banned tags
            // .or()
            // Check if this has been externally banned
            .or(col("pid").is_in(lit(Series::from_vec("ext_bans", external_bans))))
            .alias("usable");

        // This is sketchy, should likely be filter not select
        let read_pages = self
            .rating_frame
            .clone()
            .select([ignored_pages])
            .collect()
            .unwrap();

        let read_pages = read_pages.column("usable").unwrap().clone();

        // Combine with pid info again
        let page_weights = DataFrame::new(vec![read_pages, pids, page_weights])
            .unwrap()
            .lazy();

        // Drop all read pages
        let page_weights = page_weights
            .filter(col("usable"))
            .select([col("pid"), col("weights")]);

        // Sort by score
        page_weights.sort(
            ["weights"],
            SortMultipleOptions::new().with_order_descending(true),
        )
    }

    fn get_user_similarity(&self, uid: u64) -> LazyFrame {
        // New LazyFrame with 2 cols: uid column, and similarity
        let rating_frame = self.rating_frame.clone().collect().unwrap();
        let uids: Series = rating_frame
            .get_column_names()
            .iter()
            .skip(1)
            .map(|a| *a)
            .collect();
        let uids = uids.with_name("uid");
        let selected = rating_frame[format!("{}", uid).as_str()].clone();
        let similarity: Series = rating_frame
            .iter()
            .skip(1)
            .map(|a| selected.dot(a).unwrap())
            .collect();
        let similarity = similarity.with_name("similarity");
        DataFrame::new(vec![uids, similarity]).unwrap().lazy()
    }
}

fn set_up_user_frame(user_file: &str) -> Result<DataFrame, RecommenderError> {
    let file = std::fs::File::open(user_file).unwrap();
    let user_df = ParquetReader::new(file).finish().unwrap();

    Ok(user_df)
}

fn set_up_page_frame(page_file: &str) -> Result<DataFrame, RecommenderError> {
    let args = ScanArgsParquet::default();
    let page_lf = LazyFrame::scan_parquet(page_file, args)?;

    // Remove duplicates. Fixes issues with stuff like "The Troll"
    let page_lf = page_lf.unique(Some(vec!["pid".into()]), UniqueKeepStrategy::Any);

    Ok(page_lf.collect()?)
}

fn set_up_rating_frame(rating_file: &str) -> Result<LazyFrame, RecommenderError> {
    let args = ScanArgsParquet::default();
    let rating_lf = LazyFrame::scan_parquet(rating_file, args)?;

    // Remove duplicates. Fixes issues with stuff like "The Troll"
    let rating_lf = rating_lf.unique(
        Some(vec!["pid".into(), "uid".into()]),
        UniqueKeepStrategy::Any,
    );

    Ok(rating_lf)
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
    fn make_recommender() {
        // Limit to only users with very many upvotes for tests
        Recommender::new(100).unwrap();
    }
}
