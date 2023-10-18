use std::net::IpAddr;
use std::time::Duration;
use std::{env, fs::File, io::BufReader};

use dns_lookup::lookup_host;
use dotenv::dotenv;
use futures::future::join_all;
use ipinfo::{IpInfo, IpInfoConfig};
use plotpy::{Curve, Plot};
use rand::random;
use rayon::prelude::*;
use surge_ping::{Client, Config, IcmpPacket, PingIdentifier, PingSequence, ICMP};
use tokio::time;

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
    ip: String,
    location: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
struct RecordWithTime {
    ip: String,
    location: String,
    time: f64,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
struct RecordWithDistance {
    ip: String,
    location: String,
    time: f64,
    distance: f64,
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

async fn ping_ips() -> anyhow::Result<()> {
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

async fn calculate_distances() -> anyhow::Result<()> {
    let input = File::open("./with_times.json")?;
    let records: Vec<RecordWithTime> = serde_json::from_reader(BufReader::new(input))?;

    let mut cities = cities::all().to_vec();
    cities.sort_by(|a, b| a.city.cmp(b.city));

    let results = records
        .iter()
        .filter_map(|r| {
            // binary search does not work because list is not sorted by city names
            if let Ok(idx) = cities.binary_search_by(|c| c.city.cmp(r.location.as_str())) {
                let city = &cities[idx];
                let distance = geoutils::Location::new(city.latitude, city.longitude)
                    .distance_to(&geoutils::Location::new(50.9375, 6.9603))
                    .unwrap();
                Some(RecordWithDistance {
                    ip: r.ip.clone(),
                    location: r.location.clone(),
                    time: r.time,
                    distance: distance.meters() / 1000f64,
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let output = File::create("./with_distances.json")?;
    serde_json::to_writer_pretty(output, &results)?;

    Ok(())
}

fn plot_data() -> anyhow::Result<()> {
    let input = File::open("./with_distances.json")?;
    let records: Vec<RecordWithDistance> = serde_json::from_reader(BufReader::new(input))?;

    let mut curve = Curve::new();
    curve.set_line_style("None");
    curve.set_marker_style("o");
    curve.set_marker_size(1.5);

    curve.points_begin();
    for r in records {
        curve.points_add(r.time * 1000f64, r.distance);
    }
    curve.points_end();

    let mut plot = Plot::new();
    plot.add(&curve)
        .grid_and_labels("Time (ms)", "Distance (km)")
        .set_title("Ping time vs. distance")
        .set_figure_size_points(1600., 1000.)
        .set_ticks_x(100., 50., "");
    plot.save("plot.png").unwrap();

    let mut plot = Plot::new();
    plot.add(&curve)
        .grid_and_labels("Time (ms)", "Distance (km)")
        .set_title("Ping time vs. distance")
        .set_figure_size_points(1600., 1000.)
        .set_log_x(true)
        .set_log_y(true)
        .set_ticks_x(0., 0., "");
    plot.save("plot_log.png").unwrap();

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv()?;
    // collect_ips()?;
    // collect_geolocations().await?;
    // ping_ips().await?;
    // calculate_distances().await?;

    plot_data()?;

    Ok(())
}
