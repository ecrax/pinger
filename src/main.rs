use std::{env, fs::File, io::BufReader};

use dns_lookup::lookup_host;
use dotenv::dotenv;
use ipinfo::{IpInfo, IpInfoConfig};
use rayon::prelude::*;

#[derive(Debug, serde::Deserialize)]
struct Record {
    name: String,
    url: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
struct RecordWithIp {
    name: String,
    url: String,
    ip: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
struct RecordWithGeolocation {
    // name: String,
    // url: String,
    ip: String,
    location: String,
}

fn collect_ips() -> anyhow::Result<()> {
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

async fn collect_geolocations() -> anyhow::Result<()> {
    let input = File::open("./with_ips.json")?;
    let records: Vec<RecordWithIp> = serde_json::from_reader(BufReader::new(input))?;

    let config = IpInfoConfig {
        token: Some(env::var("IPINFO")?.to_string()),
        ..Default::default()
    };

    let mut ipinfo = IpInfo::new(config).expect("should construct");

    let ips = records
        .iter()
        .map(|r: &RecordWithIp| r.ip.clone())
        .collect::<Vec<_>>();
    let ips = ips.iter().map(|i| i as &str).collect::<Vec<_>>();
    let ips = ips.chunks(500).collect::<Vec<_>>();

    let mut results = Vec::new();

    for ips in ips {
        println!("lookup: {:?}", ips);
        let res = ipinfo.lookup_batch(ips, Default::default()).await;
        match res {
            Ok(res) => {
                let res = res
                    .iter()
                    .map(|(_, d)| RecordWithGeolocation {
                        ip: d.ip.clone(),
                        location: d.city.clone(),
                    })
                    .collect::<Vec<_>>();
                results.extend(res);
            }
            Err(e) => {
                println!("error: {:?}", e);
            }
        }
    }

    let output = File::create("./with_geolocations.json")?;
    serde_json::to_writer_pretty(output, &results)?;

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;
    //collect_ips()?;
    collect_geolocations().await?;
    // ping_ips()?;

    Ok(())
}
