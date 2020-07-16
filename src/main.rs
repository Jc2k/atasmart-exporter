use env_logger::{Builder, Env};
use libatasmart::Disk;
use libatasmart_sys::SkSmartOverall;
use log::error;
use prometheus::register_gauge_vec;
use prometheus_exporter::{FinishedUpdate, PrometheusExporter};
use std::net::SocketAddr;
use std::path::Path;

fn get_drives() -> std::vec::Vec<Disk> {
    let path = Path::new("/sys/bus/scsi/devices");
    let mut drives = Vec::new();

    for entry in path.read_dir().expect("read_dir call failed") {
        if let Ok(entry) = entry {
            let block_dir = entry.path().join("block");
            if !block_dir.exists() {
                continue;
            }

            let type_file = entry.path().join("type");
            if !type_file.exists() {
                continue;
            }

            let type_string = std::fs::read_to_string(type_file).unwrap();
            let type_num: i32 = type_string.trim().parse().unwrap();

            // 0 => Direct Access, 5 => CD-ROM
            // https://elixir.bootlin.com/linux/v4.0/source/drivers/scsi/scsi.c
            if type_num != 0 {
                continue;
            }

            for entry in block_dir.read_dir().expect("read_dir call failed") {
                if let Ok(entry) = entry {
                    let path = Path::new("/dev");
                    let path = path.join(entry.path().file_name().unwrap());
                    drives.push(Disk::new(&path).unwrap());
                }
            }
        }
    }

    return drives;
}

fn main() {
    // Setup logger with default level info so we can see the messages from
    // prometheus_exporter.
    Builder::from_env(Env::default().default_filter_or("info")).init();

    // Parse address used to bind exporter to.
    let addr_raw = "127.0.0.1:9393";
    let addr: SocketAddr = addr_raw.parse().expect("can not parse listen addr");

    // Start exporter.
    let (request_receiver, finished_sender) = PrometheusExporter::run_and_notify(addr);

    let metric_disk_size = register_gauge_vec!("atasmart_disk_size", "help", &["disk"])
        .expect("could not create temp gauge");
    let metric_sleep_mode = register_gauge_vec!("atasmart_sleep_mode", "help", &["disk"])
        .expect("could not create temp gauge");
    let metric_temp = register_gauge_vec!("atasmart_temperature", "help", &["disk"])
        .expect("could not create temp gauge");
    let metric_bad_sectors = register_gauge_vec!("atasmart_bad_sectors", "help", &["disk"])
        .expect("could not create temp gauge");
    let metric_power_cycles = register_gauge_vec!("atasmart_power_cycles", "help", &["disk"])
        .expect("could not create temp gauge");
    let metric_power_on = register_gauge_vec!("atasmart_power_on", "help", &["disk"])
        .expect("could not create temp gauge");
    let metric_status = register_gauge_vec!("atasmart_status", "help", &["disk"])
        .expect("could not create temp gauge");
    let metric_overall = register_gauge_vec!("atasmart_overall", "help", &["disk", "status"])
        .expect("could not create temp gauge");
    let metric_identify_is_available =
        register_gauge_vec!("atasmart_identify_available", "help", &["disk"])
            .expect("could not create temp gauge");
    let metric_smart_is_available =
        register_gauge_vec!("atasmart_smart_available", "help", &["disk"])
            .expect("could not create temp gauge");

    let mut disks = get_drives();

    loop {
        request_receiver.recv().unwrap();

        for disk in disks.iter_mut() {
            let disk_path = disk.disk.clone();
            let disk_path = disk_path.to_str().unwrap();

            match disk.refresh_smart_data() {
                Ok(_) => {}
                _ => {
                    error!("{}: Call to refresh_smart_data failed", disk_path);
                }
            }

            match &disk.get_disk_size() {
                Ok(disk_size) => {
                    let disk_size = *disk_size as f64;
                    metric_disk_size
                        .with_label_values(&[disk_path])
                        .set(disk_size);
                }
                _ => {
                    error!("{}: Failed to extract disk size", disk_path);
                }
            }

            match &disk.check_sleep_mode() {
                Ok(sleep_mode) => {
                    let sleep_mode = match sleep_mode {
                        true => 1.0,
                        false => 0.0,
                    };
                    metric_sleep_mode
                        .with_label_values(&[disk_path])
                        .set(sleep_mode);
                }
                _ => {
                    error!("{}: Failed to extract sleep mode", disk_path);
                }
            }

            match &disk.get_temperature() {
                Ok(temp_value) => {
                    let temp_value = (*temp_value as f64) / 10000.0;
                    metric_temp.with_label_values(&[disk_path]).set(temp_value);
                }
                _ => {
                    error!("{}: Failed to extract temperature", disk_path);
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
                    error!("{}: Failed to extract bad sector count", disk_path);
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
                    error!("{}: Failed to extract power cycle count", disk_path);
                }
            }

            match &disk.get_power_on() {
                Ok(power_on) => {
                    let power_on = *power_on as f64;
                    metric_power_on
                        .with_label_values(&[disk_path])
                        .set(power_on);
                }
                _ => {
                    error!("{}: Failed to extract power_on", disk_path);
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
                    error!("{}: Failed to extract smart status", disk_path);
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
                    error!("{}: Failed to extract smart overall", disk_path);
                }
            }

            match &disk.identify_is_available() {
                Ok(identify_is_available) => {
                    let identify_is_available = match identify_is_available {
                        true => 1.0,
                        false => 0.0,
                    };
                    metric_identify_is_available
                        .with_label_values(&[disk_path])
                        .set(identify_is_available);
                }
                _ => {
                    error!("{}: Failed to extract identify_is_available", disk_path);
                }
            }

            match &disk.smart_is_available() {
                Ok(smart_is_available) => {
                    let smart_is_available = match smart_is_available {
                        true => 1.0,
                        false => 0.0,
                    };
                    metric_smart_is_available
                        .with_label_values(&[disk_path])
                        .set(smart_is_available);
                }
                _ => {
                    error!("{}: Failed to extract smart_is_available", disk_path);
                }
            }
        }

        // Notify exporter that all metrics have been updated so the caller client can
        // receive a response.
        finished_sender.send(FinishedUpdate).unwrap();
    }
}
