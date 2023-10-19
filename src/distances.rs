use std::{fs::File, io::BufReader};

use crate::structs::{RecordWithTime, RecordWithDistance};

pub async fn calculate_distances() -> anyhow::Result<()> {
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