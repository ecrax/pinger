use std::{env, fs::File, io::BufReader};

use ipinfo::{IpInfo, IpInfoConfig};

use crate::structs::{RecordWithIp, RecordWithGeolocation};

pub async fn collect_geolocations() -> anyhow::Result<()> {
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
                    .values()
                    .map(|d| RecordWithGeolocation {
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