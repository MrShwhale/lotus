mod recommender;
mod lotus_core;

use core::panic;
use polars::prelude::*;

use recommender::Recommender;

fn main() {
    let recommender = Recommender::new(10);
    let recommender = match recommender {
        Ok(r) => r,
        Err(e) => {
            println!("{:?}", e);
            panic!("This shouldn't happen");
        }
    };


    println!("Getting rec");
    let recs = recommender.get_recommendations_by_uid(7904845, Vec::new(), Vec::new()).collect().unwrap();
    let top_recs = recs.head(Some(30));

    for r in top_recs.column("pid").unwrap().iter() {
        println!("Rec: ");
        match r {
            AnyValue::UInt64(pid) => match recommender.get_page_by_pid(pid)[0] {
                AnyValue::String(page_name) => println!("{}", page_name),
                _ => unreachable!()
            },
            _ => unreachable!()
        }
    }
}
