// Will create an exporter with a single metric that will randomize the value
// of the metric everytime the exporter is called.

use env_logger::{
    Builder,
    Env,
};
use prometheus::{
    register_gauge_vec,
};
use prometheus_exporter::{
    FinishedUpdate,
    PrometheusExporter,
};
use std::net::SocketAddr;
use libatasmart::Disk;
use std::path::Path;

fn main() {
    // Setup logger with default level info so we can see the messages from
    // prometheus_exporter.
    Builder::from_env(Env::default().default_filter_or("info")).init();

    // Parse address used to bind exporter to.
    let addr_raw = "127.0.0.1:93939";
    let addr: SocketAddr = addr_raw.parse().expect("can not parse listen addr");

    // Start exporter.
    let (request_receiver, finished_sender) = PrometheusExporter::run_and_notify(addr);

    let metric_temp = register_gauge_vec!("atasmart_temperature", "help", &["disk"]).expect("could not create temp gauge");
    let metric_status = register_gauge_vec!("atasmart_status", "help", &["disk"]).expect("could not create temp gauge");

    let disk_path = "/dev/sda";
    let mut disk = Disk::new(Path::new(disk_path)).expect("could not open disk");

    loop {
        // Will block until exporter receives http request.
        request_receiver.recv().unwrap();


        match &disk.get_temperature() {
            Ok(temp_value) => {
                metric_temp.with_label_values(&[disk_path]).set(*temp_value as f64);
            }
            _ => {}
        }

        match &disk.get_smart_status() {
            Ok(status) => {
                if *status {
                    metric_status.with_label_values(&[disk_path]).set(1.0);
                } else {
                    metric_status.with_label_values(&[disk_path]).set(0.0);
                }
            }
            _ => {}
        }

        // Notify exporter that all metrics have been updated so the caller client can
        // receive a response.
        finished_sender.send(FinishedUpdate).unwrap();
    }
}