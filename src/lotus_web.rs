mod lotus_core;
mod recommender;

use core::panic;
use polars::prelude::*;

use recommender::Recommender;

fn main() {
    let recommender = Recommender::new();
    let recommender = match recommender {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{:?}", e);
            panic!("This shouldn't happen");
        }
    };

    println!("Getting rec");
    let recs =
        match recommender.get_recommendations_by_uid(7904845, Vec::new(), Vec::new(), Vec::new()) {
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

    let top_recs = recs.head(Some(30));

    for r in top_recs.column("pid").unwrap().iter() {
        println!("Rec: ");
        match r {
            AnyValue::UInt64(pid) => match recommender.get_page_by_pid(pid) {
                Ok(vec) => match vec[0] {
                    AnyValue::String(page_name) => println!("{}", page_name),
                    _ => unreachable!(),
                },
                Err(e) => {
                    eprintln!("{:?}", e);
                    panic!("This shouldn't happen");
                }
            },
            _ => unreachable!(),
        }
    }
}
