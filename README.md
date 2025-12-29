# iostat

[![CI](https://github.com/Osso/iostat/actions/workflows/ci.yml/badge.svg)](https://github.com/Osso/iostat/actions/workflows/ci.yml)
[![GitHub release](https://img.shields.io/github/v/release/Osso/iostat)](https://github.com/Osso/iostat/releases)
[![GitHub Downloads](https://img.shields.io/github/downloads/Osso/iostat/total)](https://github.com/Osso/iostat/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A lightweight `iostat` replacement written in Rust. Reports CPU and I/O statistics by reading `/proc/stat` and `/proc/diskstats`.

## Features

- CPU utilization breakdown (user, system, iowait, steal, idle, irq)
- Device I/O statistics (tps, throughput)
- Extended mode with queue depth, latency, and utilization
- Filters by device name
- Static binary with no runtime dependencies (musl)

## Installation

```bash
cargo install --path .
```

## Usage

```
iostat [OPTIONS] [DEVICE...] [INTERVAL [COUNT]]
```

### Options

| Flag | Description |
|------|-------------|
| `-c` | Show CPU statistics only |
| `-d` | Show device statistics only |
| `-x` | Extended statistics (await, svctm, %util) |
| `-k` | Display in KB/s (default) |
| `-m` | Display in MB/s |
| `-y` | Omit first report (since boot) |

### Examples

```bash
# Default: CPU and device stats, 1 second interval
iostat

# Extended stats for nvme devices every 2 seconds, 5 times
iostat -x nvme 2 5

# CPU only, continuous
iostat -c 1

# Device stats in MB/s
iostat -dm 1
```

## Output

Basic mode:
```
Device          tps    kB_read/s    kB_wrtn/s
nvme0n1       45.00       180.00       520.00
```

Extended mode (`-x`):
```
Device           r/s      w/s      rkB/s      wkB/s    rrqm/s    wrqm/s   await   svctm  %util
nvme0n1        12.00    33.00     180.00     520.00      0.00      8.00    0.45    0.32   1.44
```

## License

MIT
