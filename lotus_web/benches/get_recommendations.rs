use std::hint::black_box;

use const_format::formatcp;
use lotus_web::recommender::{Recommender, RecommenderOptions};

// Directory where output files can be found
const OUTPUT_DIR: &str = "../output";

// Files to save scraped data to
const ARTICLE_OUTPUT: &str = formatcp!("{}/articles.parquet", OUTPUT_DIR);
const TAGS_OUTPUT: &str = formatcp!("{}/tags.parquet", OUTPUT_DIR);
const USERS_OUTPUT: &str = formatcp!("{}/users.parquet", OUTPUT_DIR);
const VOTES_OUTPUT: &str = formatcp!("{}/votes.parquet", OUTPUT_DIR);

use criterion::{criterion_group, criterion_main, Criterion};
pub fn criterion_benchmark(c: &mut Criterion) {
    let options = RecommenderOptions::new()
        .with_articles_file(&ARTICLE_OUTPUT)
        .with_users_file(&USERS_OUTPUT)
        .with_votes_file(&VOTES_OUTPUT)
        .with_tags_file(&TAGS_OUTPUT);
    let recommender = Recommender::new_with_options(&options).unwrap();

    let mut group = c.benchmark_group("recommender");
    // Configure Criterion.rs to detect smaller differences and increase sample size to improve
    // precision and counteract the resulting noise.
    group.significance_level(0.1).sample_size(100);
    group.bench_function("get basic recommendation", |b| {
        b.iter(|| {
            recommender.get_recommendations_by_uid(black_box(7904845), Vec::new(), Vec::new())
        })
    });
    group.bench_function("get tag-restricted recommendation", |b| {
        b.iter(|| recommender.get_recommendations_by_uid(black_box(7904845), vec![734], Vec::new()))
    });
    group.bench_function("get pid-restricted recommendation", |b| {
        b.iter(|| recommender.get_recommendations_by_uid(black_box(7904845), Vec::new(), vec![734]))
    });
    group.bench_function("get tag/pid-restricted recommendation", |b| {
        b.iter(|| {
            recommender.get_recommendations_by_uid(black_box(7904845), vec![734], vec![25282833])
        })
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
