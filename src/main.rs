use env_logger::{Builder, Env};
use libatasmart::Disk;
use libatasmart_sys::SkSmartOverall;
use log::error;
use prometheus::register_gauge_vec;
use prometheus_exporter::{FinishedUpdate, PrometheusExporter};
use std::net::SocketAddr;
use std::path::Path;

fn main() {
    // Setup logger with default level info so we can see the messages from
    // prometheus_exporter.
    Builder::from_env(Env::default().default_filter_or("info")).init();

    // Parse address used to bind exporter to.
    let addr_raw = "127.0.0.1:9393";
    let addr: SocketAddr = addr_raw.parse().expect("can not parse listen addr");

    // Start exporter.
    let (request_receiver, finished_sender) = PrometheusExporter::run_and_notify(addr);

    let metric_temp = register_gauge_vec!("atasmart_temperature", "help", &["disk"])
        .expect("could not create temp gauge");
    let metric_bad_sectors = register_gauge_vec!("atasmart_bad_sectors", "help", &["disk"])
        .expect("could not create temp gauge");
    let metric_power_cycles = register_gauge_vec!("atasmart_power_cycles", "help", &["disk"])
        .expect("could not create temp gauge");
    let metric_status = register_gauge_vec!("atasmart_status", "help", &["disk"])
        .expect("could not create temp gauge");
    let metric_overall = register_gauge_vec!("atasmart_overall", "help", &["disk", "status"])
        .expect("could not create temp gauge");

    let disk_path = "/dev/sda";
    let mut disk = Disk::new(Path::new(disk_path)).expect("could not open disk");

    loop {
        // Will block until exporter receives http request.
        request_receiver.recv().unwrap();

        match disk.refresh_smart_data() {
            Ok(_) => {}
            _ => {
                error!("Call to refresh_smart_data failed");
            }
        }

        match &disk.get_temperature() {
            Ok(temp_value) => {
                let temp_value = (*temp_value as f64) / 10000.0;
                metric_temp.with_label_values(&[disk_path]).set(temp_value);
            }
            _ => {
                error!("Failed to extract temperature");
            }
        }

        match &disk.get_bad_sectors() {
            Ok(bad_sectors) => {
                let bad_sectors = *bad_sectors as f64;
                metric_bad_sectors
                    .with_label_values(&[disk_path])
                    .set(bad_sectors);
            }
            _ => {
                error!("Failed to extract bad sector count");
            }
        }

        match &disk.get_power_cycle_count() {
            Ok(power_cycle_count) => {
                let power_cycle_count = *power_cycle_count as f64;
                metric_power_cycles
                    .with_label_values(&[disk_path])
                    .set(power_cycle_count);
            }
            _ => {
                error!("Failed to extract power cycle count");
            }
        }

        match &disk.get_smart_status() {
            Ok(status) => {
                if *status {
                    metric_status.with_label_values(&[disk_path]).set(1.0);
                } else {
                    metric_status.with_label_values(&[disk_path]).set(0.0);
                }
            }
            _ => {
                error!("Failed to extract smart status");
            }
        }

        match &disk.smart_get_overall() {
            Ok(overall) => {
                let label = match *overall {
                    SkSmartOverall::SK_SMART_OVERALL_GOOD => "good",
                    SkSmartOverall::SK_SMART_OVERALL_BAD_ATTRIBUTE_IN_THE_PAST => {
                        "bad_attr_in_past"
                    }
                    SkSmartOverall::SK_SMART_OVERALL_BAD_SECTOR => "bad_sector",
                    SkSmartOverall::SK_SMART_OVERALL_BAD_ATTRIBUTE_NOW => "bad_attr_now",
                    SkSmartOverall::SK_SMART_OVERALL_BAD_SECTOR_MANY => "bad_sector_many",
                    SkSmartOverall::SK_SMART_OVERALL_BAD_STATUS => "bad_status",
                    SkSmartOverall::SK_SMART_OVERALL_MAX => "overall_max",
                };

                metric_overall
                    .with_label_values(&[disk_path, label])
                    .set(1.0);
            }
            _ => {
                error!("Failed to extract smart overall");
            }
        }

        // Notify exporter that all metrics have been updated so the caller client can
        // receive a response.
        finished_sender.send(FinishedUpdate).unwrap();
    }
}
