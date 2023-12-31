use std::{fs::File, io::BufReader};

use dns_lookup::lookup_host;
use rayon::prelude::*;

use crate::structs::{RecordWithIp, Record};

pub fn collect_ips() -> anyhow::Result<()> {
    let input = File::open("./data.csv")?;
    let mut rdr = csv::Reader::from_reader(BufReader::new(input));
    let results = rdr
        .deserialize()
        .map(|r: Result<Record, csv::Error>| r.unwrap())
        .collect::<Vec<_>>();

    let ips = results
        .par_iter()
        .enumerate()
        .map(|(i, r)| {
            let url = r
                .url
                .replace("http://", "")
                .replace("https://", "")
                .replace('/', "");

            if let Ok(ips) = lookup_host(url.as_str()) {
                println!("lookup ({i}/{}): {:?}", results.len(), ips[0]);
                Some(RecordWithIp {
                    ip: ips[0].to_string(),
                    name: r.name.clone(),
                    url: r.url.clone(),
                })
            } else {
                None
            }
        })
        .filter(|r| r.is_some())
        .map(|r| r.unwrap())
        .collect::<Vec<_>>();

    let output = File::create("./with_ips.json")?;
    serde_json::to_writer_pretty(output, &ips)?;

    Ok(())
}
