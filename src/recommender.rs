mod recommender_types;

use recommender_types::*;
use pivot;
use polars::prelude::*;
use polars_core::utils::Container;
use polars_lazy::{dsl::col, prelude::*};
use std::{collections::HashMap, fs::File};

pub struct Recommender {
    middle_norms: HashMap<String, f64>,
    page_frame: DataFrame,
    rating_frame: LazyFrame,
    tags_frame: DataFrame,
    user_frame: DataFrame,
    users_to_consider: u32,
}

// TODO this WHOLE FILE needs error handling
// OPT use the concurrent versions of everything, and make sure that actually has benefits
impl Recommender {
    /// Creates a new recommender.
    /// Uses the default recommender settings
    pub fn new() -> Result<Recommender, RecommenderError> {
        Self::new_with_options(&DEFAULT_REC_OPTIONS)
    }

    pub fn new_with_options(options: &RecommenderOptions) -> Result<Recommender, RecommenderError> {
        // CONS adding the ability to run with fewer users/pages for testing purposes
        let user_frame = set_up_user_frame(options.users_file)?;
        let page_frame = set_up_page_frame(options.articles_file)?;
        let rating_frame = set_up_rating_frame(options.votes_file)?;

        // BUG some weird f32/f64 stuff is going on in this file
        // Try to make it just f64, ideally option to make f32
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
            .filter(col("rating_count").gt_eq(lit(options.min_votes)))
            .collect()?;

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
            &rating_frame.collect()?,
            ["uid"],
            Some(["pid"]),
            Some(["rating"]),
            false,
            None,
            None,
        )?
        .lazy()
        .fill_null(0f64)
        .cast(casts, true);

        eprintln!("Pivoted");

        let tags_frame = set_up_tags_frame(options.tags_file)?;

        // TODO reorg this
        let mut recommender = Recommender {
            middle_norms: HashMap::new(),
            rating_frame,
            page_frame,
            tags_frame,
            user_frame,
            users_to_consider: options.users_to_consider,
        };

        // Normalize data
        recommender.normalize_rating_frame()?;
        eprintln!("Normalized");

        Ok(recommender)
    }

    // OPT Also, WOW this is slow
    fn normalize_rating_frame(&mut self) -> Result<(), RecommenderError> {
        // SCP users have this nasty habit of being positive people.
        // This means that they mostly only upvote, which causes issues since then the average for many rows is 0
        // This could be a critical issue but if you knew what you were doing you wouldn't be here at all
        // So, change the average rating to be the mean but with a single 0 rating added
        // Then, properly normalize it (magnitude of 1)

        let mut frame = self.rating_frame.clone().collect()?;
        let mut z_vec = Vec::with_capacity(frame.width());
        z_vec.push(Series::new("pid", [0u64].as_ref()));
        for name in frame.get_column_names().iter().skip(1) {
            z_vec.push(Series::new(name, [0f32].as_ref()))
        }

        let z_frame = DataFrame::new(z_vec)?;
        frame.vstack_mut(&z_frame)?;

        let all_but_pid = col("*").exclude(["pid"]);
        let centered_adjusted =
            all_but_pid.clone() - (all_but_pid.clone().sum() / all_but_pid.clone().len());
        let normalized = centered_adjusted.clone() / centered_adjusted.clone().pow(2).sum().sqrt();
        frame = frame.lazy().select([col("pid"), normalized]).collect()?;

        let first = match frame.get(0) {
            Some(row) => row,
            None => unreachable!(),
        };

        for (i, name) in frame.get_column_names().iter().skip(1).enumerate() {
            let value = match first[i + 1] {
                AnyValue::Float64(val) => val,
                _ => unreachable!(),
            };

            self.middle_norms.insert(String::from(*name), value);
        }

        let frame = frame.slice(0, frame.len() - 1);

        self.rating_frame = frame.lazy();

        Ok(())
    }

    // Returns the Series representing the given page using the page dataframe
    pub fn get_page_by_pid(&self, pid: u64) -> Result<Vec<AnyValue>, RecommenderError> {
        // CONS this might be the worst?
        let pids = self.page_frame.column("pid")?;
        let mut index = 0;
        loop {
            let extracted = match pids.get(index)? {
                AnyValue::UInt64(value) => value,
                _ => unreachable!(),
            };

            if extracted == pid {
                break;
            }

            index += 1;
        }

        match self.page_frame.get(index) {
            Some(value) => Ok(value),
            None => Err(RecommenderError::OOBError),
        }
    }

    pub fn get_recommendations_by_uid(
        &self,
        uid: u64,
        banned_tags: Vec<String>,
        required_tags: Vec<String>,
        external_bans: Vec<u64>,
    ) -> Result<LazyFrame, RecommenderError> {
        let user_similarity = self.get_user_similarity(uid)?;
        let user_similarity = user_similarity.sort(
            ["similarity"],
            SortMultipleOptions::new().with_order_descending(true),
        );

        // Drop all users which have a similarity of about 1
        let user_similarity = user_similarity.filter(col("similarity").lt(lit(0.999f64)));

        // Get the most similar non-exact-copy users
        let user_similarity = user_similarity.clone().limit(self.users_to_consider).collect()?;

        // Get the list of pages which the most similar users have read
        let similar_cols: Vec<_> = user_similarity["uid"]
            .iter()
            .map(|a| {
                let a_string = a.to_string();
                col(&a_string[1..a_string.len() - 1])
            })
            .collect();

        let similar_users = self.rating_frame.clone().select(similar_cols);
        let similar_users = similar_users.collect()?;

        let similar_uids = user_similarity.column("uid")?;
        let similar_weights = user_similarity.column("similarity")?;

        // Create mapping of uid to similarity
        let mut similarity_map: HashMap<&str, f64> = HashMap::with_capacity(
            self.users_to_consider
                .try_into()
                .expect("u32 to usize should be safe. Maybe use smaller SIMILAR_TO_USE?"),
        );

        for i in 0..similar_uids.len() {
            let string = match similar_uids.get(i)? {
                AnyValue::String(val) => val,
                _ => unreachable!(),
            };

            let weight = match similar_weights.get(i)? {
                AnyValue::Float64(val) => val,
                _ => unreachable!(),
            };

            similarity_map.insert(string, weight);
        }

        let similarity_selector: Vec<_> = similar_users
            .get_column_names()
            .iter()
            .map(|col_name| {
                col(*col_name)
                    * lit(similarity_map
                        .get(*col_name)
                        .expect("Names generated directly from DF should be valid")
                        .clone())
            })
            .collect();

        let page_weights = self.rating_frame.clone().select(similarity_selector);

        // Sum all columns together
        let mut page_weights = match page_weights
            .collect()?
            .sum_horizontal(polars::frame::NullStrategy::Ignore)?
        {
            Some(value) => value,
            None => return Err(RecommenderError::OOBError),
        };

        page_weights.rename("weights");

        // Create series made of all read pages
        let uid_str = format!("{}", uid);
        let uid_col = col(uid_str.as_str());
        // This does not need to be that small since the difference between votes is large
        let f_uncert = lit(1e-6f64);
        let uid_unvote = lit(match self.middle_norms.get(&format!("{}", uid)) {
            Some(value) => value.to_owned(),
            None => return Err(RecommenderError::OOBError),
        });

        // Filter column
        let ignored_pages = uid_col
            .clone()
            // Check if the user has upvoted already
            .gt(uid_unvote.clone() + f_uncert.clone())
            // Check if the user has downvoted already
            .or(uid_col.clone().lt(uid_unvote.clone() - f_uncert.clone()))
            // TODO Check for the banned tags
            // .or()
            // TODO Check for the required tags
            // .or()
            // Check if this has been externally banned
            .or(col("pid").is_in(lit(Series::from_vec("ext_bans", external_bans))));

        let mut recommendations = self
            .rating_frame
            .clone()
            .select([col("pid"), uid_col.clone()])
            .collect()?;

        recommendations.insert_column(2, page_weights)?;

        let recommendations = recommendations.lazy().filter(ignored_pages.not());

        // Sort by similarity score
        Ok(recommendations.sort(
            ["weights"],
            SortMultipleOptions::new().with_order_descending(true),
        ))
    }

    fn get_user_similarity(&self, uid: u64) -> Result<LazyFrame, RecommenderError> {
        // New LazyFrame with 2 cols: uid column, and similarity
        let rating_frame = self.rating_frame.clone().collect()?;
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
            .map(|a| {
                selected
                    .dot(a)
                    .expect("Series of the same DF should have the same dimensions.")
            })
            .collect();
        let similarity = similarity.with_name("similarity");
        Ok(DataFrame::new(vec![uids, similarity])?.lazy())
    }
}

fn set_up_user_frame(user_file: &str) -> Result<DataFrame, RecommenderError> {
    let file = File::open(user_file)?;
    let user_df = ParquetReader::new(file).finish()?;

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
    let file = File::open(tags_file)?;
    let tags_df = ParquetReader::new(file).finish()?;

    Ok(tags_df)
}
