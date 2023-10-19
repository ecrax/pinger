use std::net::IpAddr;
use std::time::Duration;
use std::{fs::File, io::BufReader};

use futures::future::join_all;
use rand::random;
use surge_ping::{Client, Config, IcmpPacket, PingIdentifier, PingSequence, ICMP};
use tokio::time;

use crate::structs::{RecordWithGeolocation, RecordWithTime};

pub async fn ping_ips() -> anyhow::Result<()> {
    let input = File::open("./with_geolocations.json")?;
    let records: Vec<RecordWithGeolocation> = serde_json::from_reader(BufReader::new(input))?;

    let mut tasks = Vec::new();
    let client_v4 = Client::new(&Config::default())?;
    let client_v6 = Client::new(&Config::builder().kind(ICMP::V6).build())?;

    for r in records {
        //NOTE: the throttle here is arbitrary, higher values might produce more accurate results
        time::sleep(Duration::from_millis(50)).await;
        match r.ip.parse() {
            Ok(IpAddr::V4(addr)) => {
                tasks.push(tokio::spawn(ping(client_v4.clone(), IpAddr::V4(addr), r)))
            }
            Ok(IpAddr::V6(addr)) => {
                tasks.push(tokio::spawn(ping(client_v6.clone(), IpAddr::V6(addr), r)))
            }
            Err(e) => println!("{} parse to ipaddr error: {}", r.ip, e),
        }
    }

    let results = join_all(tasks).await;
    let results = results
        .into_iter()
        .filter_map(|r| r.ok())
        .flatten()
        .collect::<Vec<_>>();

    let output = File::create("./with_times.json")?;
    serde_json::to_writer_pretty(output, &results)?;

    Ok(())
}

async fn ping(
    client: Client,
    addr: IpAddr,
    record: RecordWithGeolocation,
) -> Option<RecordWithTime> {
    let payload = [0; 56];
    let mut pinger = client.pinger(addr, PingIdentifier(random())).await;
    pinger.timeout(Duration::from_secs(5));

    let res = match pinger.ping(PingSequence(0), &payload).await {
        Ok((IcmpPacket::V4(_), dur)) => Some(RecordWithTime {
            ip: record.ip.clone(),
            location: record.location.clone(),
            time: dur.as_secs_f64(),
        }),
        Ok((IcmpPacket::V6(_), dur)) => Some(RecordWithTime {
            ip: record.ip.clone(),
            location: record.location.clone(),
            time: dur.as_secs_f64(),
        }),
        Err(e) => {
            println!("Err: {} ping {}", pinger.host, e);
            None
        }
    };

    println!("[+] {} done.", pinger.host);

    res
}