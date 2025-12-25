use clap::Parser;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "iostat", about = "Report I/O statistics")]
struct Args {
    /// Show extended statistics
    #[arg(short = 'x', long)]
    extended: bool,

    /// Show CPU statistics
    #[arg(short = 'c', long)]
    cpu: bool,

    /// Show device statistics (default if no flags)
    #[arg(short = 'd', long)]
    device: bool,

    /// Display in kilobytes per second
    #[arg(short = 'k', long)]
    kilobytes: bool,

    /// Display in megabytes per second
    #[arg(short = 'm', long)]
    megabytes: bool,

    /// Omit first report with stats since boot
    #[arg(short = 'y', long)]
    omit_first: bool,

    /// Interval in seconds
    #[arg(default_value = "1")]
    interval: f64,

    /// Number of reports (0 = infinite)
    #[arg(default_value = "0")]
    count: u32,
}

#[derive(Debug, Clone, Default)]
struct CpuStats {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
    steal: u64,
}

impl CpuStats {
    fn total(&self) -> u64 {
        self.user + self.nice + self.system + self.idle + self.iowait + self.irq + self.softirq + self.steal
    }

    fn delta(&self, prev: &CpuStats) -> CpuStats {
        CpuStats {
            user: self.user.saturating_sub(prev.user),
            nice: self.nice.saturating_sub(prev.nice),
            system: self.system.saturating_sub(prev.system),
            idle: self.idle.saturating_sub(prev.idle),
            iowait: self.iowait.saturating_sub(prev.iowait),
            irq: self.irq.saturating_sub(prev.irq),
            softirq: self.softirq.saturating_sub(prev.softirq),
            steal: self.steal.saturating_sub(prev.steal),
        }
    }

    fn percentages(&self) -> (f64, f64, f64, f64, f64, f64) {
        let total = self.total() as f64;
        if total == 0.0 {
            return (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        }
        (
            (self.user + self.nice) as f64 / total * 100.0,
            self.system as f64 / total * 100.0,
            self.iowait as f64 / total * 100.0,
            self.steal as f64 / total * 100.0,
            self.idle as f64 / total * 100.0,
            (self.irq + self.softirq) as f64 / total * 100.0,
        )
    }
}

#[derive(Debug, Clone, Default)]
struct DiskStats {
    reads_completed: u64,
    reads_merged: u64,
    sectors_read: u64,
    read_time_ms: u64,
    writes_completed: u64,
    writes_merged: u64,
    sectors_written: u64,
    write_time_ms: u64,
    io_in_progress: u64,
    io_time_ms: u64,
    weighted_io_time_ms: u64,
}

impl DiskStats {
    fn delta(&self, prev: &DiskStats) -> DiskStats {
        DiskStats {
            reads_completed: self.reads_completed.saturating_sub(prev.reads_completed),
            reads_merged: self.reads_merged.saturating_sub(prev.reads_merged),
            sectors_read: self.sectors_read.saturating_sub(prev.sectors_read),
            read_time_ms: self.read_time_ms.saturating_sub(prev.read_time_ms),
            writes_completed: self.writes_completed.saturating_sub(prev.writes_completed),
            writes_merged: self.writes_merged.saturating_sub(prev.writes_merged),
            sectors_written: self.sectors_written.saturating_sub(prev.sectors_written),
            write_time_ms: self.write_time_ms.saturating_sub(prev.write_time_ms),
            io_in_progress: self.io_in_progress,
            io_time_ms: self.io_time_ms.saturating_sub(prev.io_time_ms),
            weighted_io_time_ms: self.weighted_io_time_ms.saturating_sub(prev.weighted_io_time_ms),
        }
    }
}

fn read_cpu_stats() -> io::Result<CpuStats> {
    let content = fs::read_to_string("/proc/stat")?;
    for line in content.lines() {
        if line.starts_with("cpu ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 9 {
                return Ok(CpuStats {
                    user: parts[1].parse().unwrap_or(0),
                    nice: parts[2].parse().unwrap_or(0),
                    system: parts[3].parse().unwrap_or(0),
                    idle: parts[4].parse().unwrap_or(0),
                    iowait: parts[5].parse().unwrap_or(0),
                    irq: parts[6].parse().unwrap_or(0),
                    softirq: parts[7].parse().unwrap_or(0),
                    steal: parts[8].parse().unwrap_or(0),
                });
            }
        }
    }
    Ok(CpuStats::default())
}

fn read_disk_stats() -> io::Result<HashMap<String, DiskStats>> {
    let content = fs::read_to_string("/proc/diskstats")?;
    let mut stats = HashMap::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 14 {
            let name = parts[2].to_string();

            // Skip partitions (devices ending in digit after letters, like nvme0n1p1)
            // Keep whole disks: sda, nvme0n1, etc.
            if is_partition(&name) {
                continue;
            }

            stats.insert(
                name,
                DiskStats {
                    reads_completed: parts[3].parse().unwrap_or(0),
                    reads_merged: parts[4].parse().unwrap_or(0),
                    sectors_read: parts[5].parse().unwrap_or(0),
                    read_time_ms: parts[6].parse().unwrap_or(0),
                    writes_completed: parts[7].parse().unwrap_or(0),
                    writes_merged: parts[8].parse().unwrap_or(0),
                    sectors_written: parts[9].parse().unwrap_or(0),
                    write_time_ms: parts[10].parse().unwrap_or(0),
                    io_in_progress: parts[11].parse().unwrap_or(0),
                    io_time_ms: parts[12].parse().unwrap_or(0),
                    weighted_io_time_ms: parts[13].parse().unwrap_or(0),
                },
            );
        }
    }

    Ok(stats)
}

fn is_partition(name: &str) -> bool {
    // NVMe partitions: nvme0n1p1, nvme0n1p2
    if name.contains("nvme") && name.contains('p') {
        let parts: Vec<&str> = name.split('p').collect();
        if parts.len() > 1 {
            if let Some(last) = parts.last() {
                return last.chars().all(|c| c.is_ascii_digit()) && !last.is_empty();
            }
        }
    }
    // SCSI/SATA partitions: sda1, sdb2
    if name.starts_with("sd") || name.starts_with("hd") || name.starts_with("vd") {
        let suffix: String = name.chars().skip(3).collect();
        return suffix.chars().all(|c| c.is_ascii_digit()) && !suffix.is_empty();
    }
    // Loop devices with partitions
    if name.starts_with("loop") && name.contains('p') {
        return true;
    }
    false
}

fn print_cpu_header() {
    println!(
        "{:>6} {:>6} {:>6} {:>6} {:>6} {:>6}",
        "%user", "%sys", "%iowait", "%steal", "%idle", "%irq"
    );
}

fn print_cpu_stats(delta: &CpuStats) {
    let (user, sys, iowait, steal, idle, irq) = delta.percentages();
    println!(
        "{:>6.2} {:>6.2} {:>6.2} {:>6.2} {:>6.2} {:>6.2}",
        user, sys, iowait, steal, idle, irq
    );
}

fn print_device_header(extended: bool) {
    if extended {
        println!(
            "{:<12} {:>8} {:>8} {:>10} {:>10} {:>8} {:>8} {:>7} {:>7} {:>6}",
            "Device", "r/s", "w/s", "rkB/s", "wkB/s", "rrqm/s", "wrqm/s", "await", "svctm", "%util"
        );
    } else {
        println!(
            "{:<12} {:>8} {:>10} {:>10}",
            "Device", "tps", "kB_read/s", "kB_wrtn/s"
        );
    }
}

fn print_device_stats(
    name: &str,
    delta: &DiskStats,
    interval_secs: f64,
    extended: bool,
    unit_divisor: f64,
) {
    let reads_per_sec = delta.reads_completed as f64 / interval_secs;
    let writes_per_sec = delta.writes_completed as f64 / interval_secs;
    let tps = reads_per_sec + writes_per_sec;

    // Sectors are 512 bytes
    let kb_read_per_sec = (delta.sectors_read as f64 * 512.0) / 1024.0 / interval_secs / unit_divisor;
    let kb_written_per_sec = (delta.sectors_written as f64 * 512.0) / 1024.0 / interval_secs / unit_divisor;

    if extended {
        let rrqm_per_sec = delta.reads_merged as f64 / interval_secs;
        let wrqm_per_sec = delta.writes_merged as f64 / interval_secs;

        let total_ios = delta.reads_completed + delta.writes_completed;
        let await_ms = if total_ios > 0 {
            (delta.read_time_ms + delta.write_time_ms) as f64 / total_ios as f64
        } else {
            0.0
        };

        let svctm = if total_ios > 0 {
            delta.io_time_ms as f64 / total_ios as f64
        } else {
            0.0
        };

        let util = (delta.io_time_ms as f64 / (interval_secs * 1000.0)) * 100.0;
        let util = util.min(100.0);

        println!(
            "{:<12} {:>8.2} {:>8.2} {:>10.2} {:>10.2} {:>8.2} {:>8.2} {:>7.2} {:>7.2} {:>6.2}",
            name, reads_per_sec, writes_per_sec, kb_read_per_sec, kb_written_per_sec,
            rrqm_per_sec, wrqm_per_sec, await_ms, svctm, util
        );
    } else {
        println!(
            "{:<12} {:>8.2} {:>10.2} {:>10.2}",
            name, tps, kb_read_per_sec, kb_written_per_sec
        );
    }
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    // Determine what to show
    let show_cpu = args.cpu || (!args.cpu && !args.device);
    let show_device = args.device || (!args.cpu && !args.device);

    let unit_divisor = if args.megabytes { 1024.0 } else { 1.0 };

    let interval = Duration::from_secs_f64(args.interval);
    let mut count = if args.count == 0 { u32::MAX } else { args.count };

    let mut prev_cpu = read_cpu_stats()?;
    let mut prev_disk = read_disk_stats()?;

    // First report (since boot) unless -y
    if !args.omit_first {
        if show_cpu {
            println!("avg-cpu:");
            print_cpu_header();
            print_cpu_stats(&prev_cpu);
            println!();
        }

        if show_device {
            println!("Device:");
            print_device_header(args.extended);
            let mut devices: Vec<_> = prev_disk.keys().collect();
            devices.sort();
            for name in devices {
                print_device_stats(name, &DiskStats::default(), 1.0, args.extended, unit_divisor);
            }
            println!();
        }

        count = count.saturating_sub(1);
        if count == 0 {
            return Ok(());
        }
    }

    // Subsequent reports
    loop {
        thread::sleep(interval);
        io::stdout().flush()?;

        let curr_cpu = read_cpu_stats()?;
        let curr_disk = read_disk_stats()?;

        if show_cpu {
            println!("avg-cpu:");
            print_cpu_header();
            let delta = curr_cpu.delta(&prev_cpu);
            print_cpu_stats(&delta);
            println!();
        }

        if show_device {
            println!("Device:");
            print_device_header(args.extended);
            let mut devices: Vec<_> = curr_disk.keys().collect();
            devices.sort();
            for name in devices {
                if let (Some(curr), Some(prev)) = (curr_disk.get(name), prev_disk.get(name)) {
                    let delta = curr.delta(prev);
                    print_device_stats(name, &delta, args.interval, args.extended, unit_divisor);
                }
            }
            println!();
        }

        prev_cpu = curr_cpu;
        prev_disk = curr_disk;

        count = count.saturating_sub(1);
        if count == 0 {
            break;
        }
    }

    Ok(())
}
