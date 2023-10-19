use std::{fs::File, io::BufReader};

use plotpy::{Curve, Plot};

use crate::structs::RecordWithDistance;

pub fn plot_data() -> anyhow::Result<()> {
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
        .set_figure_size_points(1000., 600.)
        .set_ticks_x(100., 50., "");
    plot.save("plot.svg").unwrap();

    let mut plot = Plot::new();
    plot.add(&curve)
        .grid_and_labels("Time (ms)", "Distance (km)")
        .set_title("Ping time vs. distance (log-log)")
        .set_figure_size_points(1000., 600.)
        .set_log_x(true)
        .set_log_y(true)
        .set_ticks_x(0., 0., "");
    plot.save("plot_log.svg").unwrap();

    let mut plot = Plot::new();
    plot.add(&curve)
        .grid_and_labels("Time (ms)", "Distance (km)")
        .set_title("Ping time vs. distance (log-log)")
        .set_range(0., 100., 0., 2500.)
        .set_figure_size_points(1000., 600.);
    plot.save("plot_crop.svg").unwrap();

    Ok(())
}