use clap::Parser;
use std::{
    error::Error,
    net::SocketAddr,
    str::FromStr,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

mod raspi;
mod scd41;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("0.0.0.0:9000"))]
    server: String,
}

fn main() {
    env_logger::init();
    let args = Args::parse();

    log::info!("start scd41 exporter");

    init_prometheus(&args.server).expect("failed to install prometheus exporter");
    log::info!("start prometheus server at {:}", args.server);

    let mut i2c = raspi::init_raspi().expect("failed to init i2c");
    scd41::clean_state(&mut i2c);
    let serial = scd41::read_serial(&mut i2c).expect("failed to read serial from scd41");
    log::info!("scd41's serial number: 0x{:x}", serial);
    scd41::start_periodic_measurement(&mut i2c).expect("failed to start scd41");
    thread::sleep(Duration::from_secs(5));

    let co2 = metrics::gauge!("co2_ppm");
    let temp = metrics::gauge!("temperature_celsius");
    let hum = metrics::gauge!("humidity_rh");
    let last_measured = metrics::gauge!("last_measured_timestamp_ms");

    loop {
        thread::sleep(Duration::from_secs(1));

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .inspect_err(|e| log::warn!("failed to get current time: {:?}", e))
            .map(|d| d.as_millis() as f64)
            .unwrap_or_default();

        let is_ready = scd41::get_data_ready_status(&mut i2c);
        if is_ready.is_err() {
            log::info!("failed to get deady flag, but countinue");
            continue;
        }
        if !(is_ready.unwrap()) {
            log::trace!("scd41 is not ready, but countinue");
            continue;
        }

        let measurement = scd41::read_measurement(&mut i2c);
        match measurement {
            Err(e) => log::warn!("failed to get measurement: {:?}", e),
            Ok(m) => {
                co2.set(m.co2);
                temp.set(m.temperature);
                hum.set(m.humidity);
                last_measured.set(timestamp);
            }
        }
    }
}

fn init_prometheus(addr: &str) -> Result<(), Box<dyn Error>> {
    let socket = SocketAddr::from_str(addr)?;

    let builder = metrics_exporter_prometheus::PrometheusBuilder::new();
    builder.with_http_listener(socket).install()?;

    return Ok(());
}
