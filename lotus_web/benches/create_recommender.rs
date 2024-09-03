use const_format::formatcp;
use lotus_web::recommender::{Recommender, RecommenderOptions};

// Directory where output files can be found
const OUTPUT_DIR: &str = "../output";

// Files to save scraped data to
const ARTICLE_OUTPUT: &str = formatcp!("{}/articles.parquet", OUTPUT_DIR);
const TAGS_OUTPUT: &str = formatcp!("{}/tags.parquet", OUTPUT_DIR);
const USERS_OUTPUT: &str = formatcp!("{}/users.parquet", OUTPUT_DIR);
const VOTES_OUTPUT: &str = formatcp!("{}/votes.parquet", OUTPUT_DIR);

use criterion::{black_box, criterion_group, criterion_main, Criterion};
pub fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("recommender");
    // Sample size is heavily limited due to length of creating a single instance
    group.significance_level(0.1).sample_size(10);
    let options = RecommenderOptions::new()
        .with_articles_file(ARTICLE_OUTPUT.into())
        .with_users_file(USERS_OUTPUT.into())
        .with_votes_file(VOTES_OUTPUT.into())
        .with_tags_file(TAGS_OUTPUT.into());
    group.bench_function("recommender default", |b| {
        b.iter(|| black_box(Recommender::new_with_options(black_box(&options))))
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
