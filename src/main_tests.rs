use super::*;

fn disk_stats() -> DiskStats {
    DiskStats {
        reads_completed: 100,
        reads_merged: 10,
        sectors_read: 2048,
        read_time_ms: 400,
        writes_completed: 50,
        writes_merged: 5,
        sectors_written: 1024,
        write_time_ms: 200,
        io_in_progress: 2,
        io_time_ms: 300,
        weighted_io_time_ms: 600,
    }
}

#[test]
fn parse_positional_splits_devices_interval_and_count() {
    let args = vec![
        "/dev/sda".to_string(),
        "nvme0n1".to_string(),
        "2.5".to_string(),
        "4".to_string(),
    ];

    let parsed = parse_positional(&args);

    assert_eq!(parsed.devices, vec!["sda", "nvme0n1"]);
    assert_eq!(parsed.interval, 2.5);
    assert_eq!(parsed.count, 4);
}

#[test]
fn parse_positional_defaults_and_single_trailing_interval() {
    let parsed = parse_positional(&["vda".to_string(), "3".to_string()]);

    assert_eq!(parsed.devices, vec!["vda"]);
    assert_eq!(parsed.interval, 3.0);
    assert_eq!(parsed.count, 0);
}

#[test]
fn cpu_total_delta_and_percentages_are_saturating() {
    let prev = CpuStats {
        user: 100,
        nice: 10,
        system: 40,
        idle: 200,
        iowait: 20,
        irq: 5,
        softirq: 5,
        steal: 10,
    };
    let curr = CpuStats {
        user: 130,
        nice: 15,
        system: 45,
        idle: 260,
        iowait: 25,
        irq: 7,
        softirq: 8,
        steal: 12,
    };

    let delta = curr.delta(&prev);
    let percentages = delta.percentages();

    assert_eq!(delta.total(), 112);
    assert_eq!(delta.user, 30);
    assert_eq!(delta.idle, 60);
    assert!((percentages.0 - 31.25).abs() < 0.01);
    assert!((percentages.4 - 53.57).abs() < 0.01);
    assert_eq!(
        CpuStats::default().percentages(),
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
    );
}

#[test]
fn disk_delta_rates_and_extended_stats_are_computed() {
    let prev = DiskStats {
        reads_completed: 80,
        reads_merged: 4,
        sectors_read: 1024,
        read_time_ms: 100,
        writes_completed: 40,
        writes_merged: 2,
        sectors_written: 512,
        write_time_ms: 50,
        io_in_progress: 1,
        io_time_ms: 100,
        weighted_io_time_ms: 200,
    };
    let curr = disk_stats();

    let delta = curr.delta(&prev);
    let rates = compute_device_rates(&delta, 2.0, 1.0);
    let extended = compute_extended_device_stats(&delta, 2.0);

    assert_eq!(delta.reads_completed, 20);
    assert_eq!(delta.io_in_progress, 2);
    assert_eq!(rates.reads_per_sec, 10.0);
    assert_eq!(rates.writes_per_sec, 5.0);
    assert_eq!(rates.tps, 15.0);
    assert_eq!(rates.kb_read_per_sec, 256.0);
    assert_eq!(rates.kb_written_per_sec, 128.0);
    assert_eq!(extended.rrqm_per_sec, 3.0);
    assert_eq!(extended.wrqm_per_sec, 1.5);
    assert_eq!(extended.await_ms, 15.0);
    assert!((extended.svctm - 6.666).abs() < 0.01);
    assert_eq!(extended.util, 10.0);
}

#[test]
fn helpers_classify_devices_filters_and_zero_average() {
    assert!(is_partition("sda1"));
    assert!(is_partition("nvme0n1p2"));
    assert!(!is_partition("sda"));
    assert!(!is_partition("nvme0n1"));

    assert!(matches_filter("nvme0n1", &[]));
    assert!(matches_filter("nvme0n1", &["nvme".to_string()]));
    assert!(!matches_filter("sda", &["nvme".to_string()]));
    assert_eq!(average_or_zero(10, 0), 0.0);
    assert_eq!(average_or_zero(10, 2), 5.0);
}

#[test]
fn build_run_config_derives_modes_units_and_infinite_count() {
    let args = Args {
        extended: true,
        cpu: false,
        device: true,
        kilobytes: false,
        megabytes: true,
        omit_first: true,
        positional: Vec::new(),
    };
    let parsed = ParsedArgs {
        devices: vec!["sda".to_string()],
        interval: 2.5,
        count: 0,
    };

    let cfg = build_run_config(args, parsed);

    assert!(!cfg.show_cpu);
    assert!(cfg.show_device);
    assert_eq!(cfg.unit_divisor, 1024.0);
    assert_eq!(cfg.interval, Duration::from_secs_f64(2.5));
    assert_eq!(cfg.interval_secs, 2.5);
    assert_eq!(cfg.count, u32::MAX);
    assert!(cfg.extended);
    assert!(cfg.omit_first);
    assert_eq!(cfg.devices, vec!["sda"]);
}
