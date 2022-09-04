use uuid::{Uuid, v1::{Timestamp, Context}};
use rand::Rng;
use chrono::prelude::*;

pub fn time_uuid() -> Uuid { 
    let mut rng = rand::thread_rng();
    let rand: [u8; 6] = rng.gen();
    let rand_num: u16 = rng.gen();
    let context = Context::new(rand_num);
    let utc: DateTime<Utc> = Utc::now(); 
    let ts = Timestamp::from_unix(&context, utc.timestamp() as u64, utc.timestamp_subsec_nanos());
    Uuid::new_v1(ts, &rand).expect("failed to generate UUID")
}