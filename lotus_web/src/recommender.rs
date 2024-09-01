mod recommender_types;

use pivot;
use polars::{
    datatypes::{PlHashMap, PlHashSet},
    prelude::*,
};
use polars_core::utils::Container;
use polars_lazy::{dsl::col, prelude::*};
use recommender_types::*;
use std::fs::File;

pub use recommender_types::RecommenderOptions;

pub struct Recommender {
    middle_norms: PlHashMap<String, f64>,
    page_frame: DataFrame,
    page_map: PlHashMap<u64, usize>,
    rating_frame: LazyFrame,
    tags_frame: DataFrame,
    user_frame: DataFrame,
    users_to_consider: u32,
}

static RECOMENDER_HEADING: &str = "[RECOMMENDER] ";

// TODO this WHOLE FILE needs error handling
// Concurrent LazyFrame collection did not have measurable benefits
impl Recommender {
    /// Creates a new recommender.
    /// Uses the default recommender settings.
    #[inline]
    pub fn new() -> Result<Recommender, RecommenderError> {
        Self::new_with_options(&RecommenderOptions::new())
    }

    /// Creates a new recommender with the provided settings
    pub fn new_with_options(options: &RecommenderOptions) -> Result<Recommender, RecommenderError> {
        let page_frame = set_up_page_frame(options.articles_file)?;
        let user_frame = set_up_user_frame(options.users_file)?;
        let rating_frame = set_up_rating_frame(options.votes_file)?;
        // CONS storing as a Series instead of Dataframe
        let tags_frame = set_up_tags_frame(options.tags_file)?;

        // Create a map of page ids to indicies here
        // This means that the page_frame ordering should NEVER be changed without also changing this map
        let mut page_map = PlHashMap::with_capacity(page_frame.len());
        for (i, pid) in page_frame
            .column("pid")
            .expect("Hardcoded column name.")
            .iter()
            .enumerate()
        {
            match pid {
                AnyValue::UInt64(pid) => {
                    // Should never have duplicate pids due to checks in page frame creation
                    page_map.insert_unique_unchecked(pid, i);
                }
                _ => unreachable!(),
            }
        }

        // i8 values are used for minimizing memory, cast them into usable f64
        let mut casts = PlHashMap::new();
        casts.insert("rating", DataType::Float64);
        let rating_frame = rating_frame.cast(casts, true);
        eprintln!("{}Frames made", RECOMENDER_HEADING);

        // OPT this could probably be optimized
        // Count votes from each user
        let selected_users = rating_frame
            .clone()
            .group_by(["uid"])
            .agg([col("rating").count().alias("rating_count")])
            .filter(col("rating_count").gt_eq(lit(options.min_votes)))
            .collect()?;

        let rating_frame =
            rating_frame.filter(col("uid").is_in(lit(selected_users["uid"].clone())));
        eprintln!("{}Irrelevant users discarded", RECOMENDER_HEADING);

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
        .cast(casts, false);

        eprintln!("{}Pivoted", RECOMENDER_HEADING);

        let mut recommender = Recommender {
            middle_norms: PlHashMap::new(),
            rating_frame,
            page_frame,
            page_map,
            tags_frame,
            user_frame,
            users_to_consider: options.users_to_consider,
        };

        recommender.normalize_rating_frame()?;
        eprintln!("{}Normalized", RECOMENDER_HEADING);

        Ok(recommender)
    }

    // OPT This could be sped up with in-place
    fn normalize_rating_frame(&mut self) -> Result<(), RecommenderError> {
        let mut frame = self.rating_frame.clone().collect()?;
        let mut z_vec = Vec::with_capacity(frame.width());
        z_vec.push(Series::new("pid", [0u64].as_ref()));
        for name in frame.get_column_names().iter().skip(1) {
            z_vec.push(Series::new(name, [0f64].as_ref()))
        }

        let z_frame = DataFrame::new(z_vec)?;
        frame.vstack_mut(&z_frame)?;

        let all_but_pid = col("*").exclude(["pid"]);
        let centered_adjusted = all_but_pid.clone() - all_but_pid.clone().mean();
        let l_frame = frame.lazy().select([col("pid"), centered_adjusted]);

        // Normalization must occur for cosine similarity to be done easily
        let normalized = all_but_pid.clone() / all_but_pid.clone().pow(2).sum().sqrt();
        let frame = l_frame.select([col("pid"), normalized]).collect()?;

        // Normalize each non-pid column in-place
        // OPT unsafe access mutable columns since the lengths are not changing
        // Then modify in place
        // unsafe {
        //     frame.get_columns_mut().into_par_iter().for_each(|column| normalize_column(column));
        // };

        let first = match frame.get(0) {
            Some(row) => row,
            None => unreachable!(),
        };

        for (i, name) in frame.get_column_names().iter().skip(1).enumerate() {
            // We can index at i + 1 since we skipped 1 earlier
            let value = match first[i + 1] {
                AnyValue::Float64(val) => val,
                _ => unreachable!(),
            };

            // This is used later to see if users have voted on a page or not
            self.middle_norms.insert(String::from(*name), value);
        }

        let frame = frame.slice(0, frame.len() - 1);

        self.rating_frame = frame.lazy();

        Ok(())
    }

    /// Returns the Series representing the given user using the page dataframe
    pub fn get_user_by_username(&self, username: &str) -> Result<Vec<AnyValue>, RecommenderError> {
        eprintln!("{}Searching for user: {}", RECOMENDER_HEADING, username);
        // OPT this is not a hot function, though this could likely be improved with concurrency or
        // sorting. This is not done in case user order is later needed to be kept.
        let names = self.user_frame.column("name")?;
        let mut index = 0;
        loop {
            let extracted = match names.get(index)? {
                AnyValue::String(value) => value,
                _ => unreachable!(),
            };

            if extracted == username {
                eprintln!("{}User found at position {}", RECOMENDER_HEADING, index);
                break;
            }

            index += 1;
        }

        match self.user_frame.get(index) {
            Some(value) => Ok(value),
            None => Err(RecommenderError::Bounds),
        }
    }

    // Returns the Series representing the given page using the page dataframe
    pub fn get_page_by_pid(&self, pid: u64) -> Result<Vec<AnyValue>, RecommenderError> {
        match self.page_map.get(&pid) {
            Some(index) => Ok(self
                .page_frame
                .get(*index)
                .expect("Recorded index should always be in bounds.")),
            None => Err(RecommenderError::Bounds),
        }
    }

    // Return every page ordered by how highly they are recommended
    pub fn get_recommendations_by_uid(
        &self,
        uid: u64,
        required_tags: Vec<u16>,
        external_bans: Vec<u64>,
    ) -> Result<LazyFrame, RecommenderError> {
        let similarity_selector = self.get_similarity_selector(uid)?;

        let page_weights = self.rating_frame.clone().select(similarity_selector);

        // Sum all columns together
        let mut page_weights = match page_weights
            .collect()?
            .sum_horizontal(polars::frame::NullStrategy::Ignore)?
        {
            Some(value) => value,
            None => return Err(RecommenderError::Bounds),
        };

        page_weights.rename("weights");

        // Create series made of all read pages
        let uid_str = format!("{}", uid);
        let uid_col = col(uid_str.as_str());
        // This does not need to be that small since the difference between votes is large
        let f_uncert = lit(1e-6f64);
        let uid_unvote = lit(match self.middle_norms.get(&format!("{}", uid)) {
            Some(value) => value.to_owned(),
            None => return Err(RecommenderError::Bounds),
        });

        // Filter column
        let ignored_pages = uid_col
            .clone()
            // Check if the user has upvoted already
            .gt(uid_unvote.clone() + f_uncert.clone())
            // Check if the user has downvoted already
            .or(uid_col.clone().lt(uid_unvote.clone() - f_uncert.clone()))
            // Check if this has been externally banned
            .or(col("pid").is_in(lit(Series::from_vec("ext_bans", external_bans))));

        let mut recommendations = self
            .rating_frame
            .clone()
            .select([col("pid"), uid_col.clone()])
            .collect()?;

        recommendations.insert_column(2, page_weights)?;

        if !required_tags.is_empty() {
            let req_tag_set: PlHashSet<_> = required_tags.into_iter().collect();

            // OPT this is a little slow
            // Create a Boolean Series which contains tag bans
            let mask: Series = recommendations
                .column("pid")
                .unwrap()
                .iter()
                .map(|pid| {
                    let pid = match pid {
                        AnyValue::UInt64(value) => value,
                        _ => unreachable!(),
                    };
                    let page = self.get_page_by_pid(pid).unwrap();
                    let tag_list = match &page[3] {
                        AnyValue::List(value) => value,
                        _ => unreachable!(),
                    };

                    // Find page tags from pid
                    let page_tags: PlHashSet<_> = tag_list
                        .iter()
                        .map(|value| match value {
                            AnyValue::UInt16(int_val) => int_val,
                            _ => unreachable!(),
                        })
                        .collect();

                    page_tags.is_superset(&req_tag_set)
                })
                .collect();

            recommendations = recommendations.filter(mask.bool()?)?;
        }

        let recommendations = recommendations.lazy().filter(ignored_pages.not());

        Ok(recommendations.sort(
            ["weights"],
            SortMultipleOptions::new().with_order_descending(true),
        ))
    }

    // Return a vector which can be used in a select on the rating frame to create page weights
    fn get_similarity_selector(&self, uid: u64) -> Result<Vec<Expr>, RecommenderError> {
        let user_similarity = self.get_user_similarity(uid)?;
        let user_similarity = user_similarity.sort(
            ["similarity"],
            SortMultipleOptions::new().with_order_descending(true),
        );

        // Drop all users which have a similarity of about 1
        let user_similarity = user_similarity.filter(col("similarity").lt(lit(0.999f64)));

        // Get the most similar non-exact-copy users
        let user_similarity = user_similarity
            .clone()
            .limit(self.users_to_consider)
            .collect()?;

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
        let mut similarity_map: PlHashMap<&str, f64> = PlHashMap::with_capacity(
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
                col(col_name)
                    * lit(*similarity_map
                        .get(col_name)
                        .expect("Names generated directly from DF should be valid"))
            })
            .collect();

        Ok(similarity_selector)
    }

    // Get the similarity (0-1.0) of one user to every other user
    fn get_user_similarity(&self, uid: u64) -> Result<LazyFrame, RecommenderError> {
        // New LazyFrame with 2 cols: uid column, and similarity
        let rating_frame = self.rating_frame.clone().collect()?;
        let uids: Series = rating_frame
            .get_column_names()
            .iter()
            .skip(1)
            .copied()
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

    pub fn get_tag_by_id(&self, index: u16) -> Option<String> {
        // eprintln!("{}Searching for tag with id: {}", RECOMENDER_HEADING, index);
        match self.tags_frame.get(index.into())?.first()? {
            AnyValue::String(value) => {
                // eprintln!("{}Tag with id {} is {}", RECOMENDER_HEADING, index, *value);
                Some(String::from(*value))
            }
            _ => unreachable!(),
        }
    }

    pub fn get_tags(&self) -> Vec<&str> {
        self.tags_frame
            .column("tag")
            .unwrap()
            .str()
            .unwrap()
            .into_no_null_iter()
            .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use const_format::formatcp;
    // Directory where output files can be found
    const OUTPUT_DIR: &str = "../output";

    // Files to save scraped data to
    const ARTICLE_OUTPUT: &str = formatcp!("{}/articles.parquet", OUTPUT_DIR);
    const TAGS_OUTPUT: &str = formatcp!("{}/tags.parquet", OUTPUT_DIR);
    const USERS_OUTPUT: &str = formatcp!("{}/users.parquet", OUTPUT_DIR);
    const VOTES_OUTPUT: &str = formatcp!("{}/votes.parquet", OUTPUT_DIR);

    #[test]
    fn create_recommender() {
        Recommender::new().unwrap();
    }

    #[test]
    fn get_recommendation() {
        let options = RecommenderOptions::new()
            .with_articles_file(&ARTICLE_OUTPUT)
            .with_users_file(&USERS_OUTPUT)
            .with_votes_file(&VOTES_OUTPUT)
            .with_tags_file(&TAGS_OUTPUT)
            // Users limited so that it runs faster
            .with_min_votes(100);

        let rec = Recommender::new_with_options(&options).expect("Recommender not created");

        let rating_frame = set_up_rating_frame(options.votes_file)
            .expect("Ratings not read")
            .collect()
            .expect("Ratings collected");

        let column = match rating_frame.get(0).expect("Row found")[1] {
            AnyValue::UInt64(value) => value,
            _ => unreachable!(),
        };

        rec.get_recommendations_by_uid(column, Vec::new(), Vec::new())
            .expect("Recommendation not made")
            .collect()
            .expect("Not collected");
    }
}