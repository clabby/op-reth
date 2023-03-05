use std::fs;

use serde_json::Value;

fn main() {
    let time = std::time::Instant::now();
    println!("Time now: {:?}", time);
    let file_content = fs::read_to_string("data/alloc_everything_4061224_final.json").unwrap();
    let value = serde_json::from_str::<Value>(&file_content).unwrap();
    println!("Time after: {:?}", time.elapsed());
    let account = "0x4200000000000000000000000000000000000006";
    println!("Account value: {:#?}", value.get(account).unwrap());
    // println!("First storage value: {:#?}", value.get(account).unwrap().get("0x0000000000000000000000000000000000000000"));
    // println!("")
}
