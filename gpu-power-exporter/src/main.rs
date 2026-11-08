// Copyright 2020 The scaphandre authors
// Copyright 2026 pain459
// SPDX-License-Identifier: MIT

use clap::Parser;
use colored::Colorize;
use log::{error, info, warn};
use std::process::Command;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};

// Atomics to share the latest GPU readings between the polling thread
// and the HTTP handler without a mutex.

static GPU_POWER_MW: AtomicU64 = AtomicU64::new(0); // power in milliwatts
static GPU_UTIL_PERCENT: AtomicU8 = AtomicU8::new(0);
static GPU_MEM_USED_MIB: AtomicU64 = AtomicU64::new(0);
static GPU_MEM_TOTAL_MIB: AtomicU64 = AtomicU64::new(0);
static GPU_TEMP_C: AtomicU8 = AtomicU8::new(0);
static GPU_POWER_LIMIT_W: AtomicU64 = AtomicU64::new(0);
static GPU_NAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();
static LAST_POLL_OK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static POLL_COUNT: AtomicU64 = AtomicU64::new(0);

#[derive(Parser, Debug)]
#[command(name = "gpu-power-exporter")]
#[command(about = "Prometheus exporter for NVIDIA GPU metrics (power, utilization, memory)")]
struct Cli {
    /// Port to listen on
    #[arg(short, long, default_value_t = 8082)]
    port: u16,

    /// Polling interval in milliseconds
    #[arg(short, long, default_value_t = 1000)]
    interval_ms: u64,

    /// Increase verbosity
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

/// Query nvidia-smi and update the atomic globals.
fn poll_gpu() -> bool {
    let output = match Command::new("nvidia-smi")
        .args([
            "--query-gpu=power.draw,utilization.gpu,memory.used,memory.total,name,temperature.gpu,power.limit",
            "--format=csv,noheader,nounits",
        ])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            error!("Failed to execute nvidia-smi: {}", e);
            return false;
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("nvidia-smi failed: {}", stderr.trim());
        return false;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().next().unwrap_or("");

    // Parse: power.draw(W), utilization.gpu(%), memory.used(MiB), memory.total(MiB), name, temp(C), power.limit(W)
    let parts: Vec<&str> = line.split(", ").collect();
    if parts.len() < 7 {
        error!("Unexpected nvidia-smi output format: {}", line);
        return false;
    }

    let power_w: f64 = match parts[0].trim().parse() {
        Ok(v) => v,
        Err(_) => {
            error!("Failed to parse power.draw: '{}'", parts[0]);
            return false;
        }
    };
    let util: u8 = match parts[1].trim().parse() {
        Ok(v) => v,
        Err(_) => {
            error!("Failed to parse utilization: '{}'", parts[1]);
            return false;
        }
    };
    let mem_used: u64 = match parts[2].trim().parse() {
        Ok(v) => v,
        Err(_) => {
            error!("Failed to parse memory.used: '{}'", parts[2]);
            return false;
        }
    };
    let mem_total: u64 = match parts[3].trim().parse() {
        Ok(v) => v,
        Err(_) => {
            error!("Failed to parse memory.total: '{}'", parts[3]);
            return false;
        }
    };
    let name: &str = parts[4].trim();
    let temp: u8 = match parts[5].trim().parse() {
        Ok(v) => v,
        Err(_) => {
            error!("Failed to parse temperature: '{}'", parts[5]);
            return false;
        }
    };
    let power_limit_w: f64 = match parts[6].trim().parse() {
        Ok(v) => v,
        Err(_) => {
            error!("Failed to parse power.limit: '{}'", parts[6]);
            return false;
        }
    };

    // Store in atomics
    GPU_POWER_MW.store((power_w * 1000.0) as u64, Ordering::Relaxed);
    GPU_UTIL_PERCENT.store(util, Ordering::Relaxed);
    GPU_MEM_USED_MIB.store(mem_used, Ordering::Relaxed);
    GPU_MEM_TOTAL_MIB.store(mem_total, Ordering::Relaxed);
    GPU_TEMP_C.store(temp, Ordering::Relaxed);
    GPU_POWER_LIMIT_W.store(power_limit_w as u64, Ordering::Relaxed);
    let _ = GPU_NAME.get_or_init(|| name.to_string());
    LAST_POLL_OK.store(true, std::sync::atomic::Ordering::Relaxed);
    POLL_COUNT.fetch_add(1, Ordering::Relaxed);

    true
}

/// Generate Prometheus text exposition format.
fn render_metrics() -> String {
    let power_mw = GPU_POWER_MW.load(Ordering::Relaxed);
    let util = GPU_UTIL_PERCENT.load(Ordering::Relaxed);
    let mem_used = GPU_MEM_USED_MIB.load(Ordering::Relaxed);
    let mem_total = GPU_MEM_TOTAL_MIB.load(Ordering::Relaxed);
    let temp = GPU_TEMP_C.load(Ordering::Relaxed);
    let power_limit_w = GPU_POWER_LIMIT_W.load(Ordering::Relaxed);
    let name = GPU_NAME.get().map(|s| s.as_str()).unwrap_or("unknown");
    let poll_ok = LAST_POLL_OK.load(std::sync::atomic::Ordering::Relaxed) as u8;
    let poll_count = POLL_COUNT.load(Ordering::Relaxed);

    let mut out = String::with_capacity(1024);

    out.push_str("# HELP nvidia_gpu_power_watts Current GPU power draw in watts\n");
    out.push_str("# TYPE nvidia_gpu_power_watts gauge\n");
    out.push_str(&format!("nvidia_gpu_power_watts {{gpu=\"{}\"}} {}\n", name, power_mw as f64 / 1000.0));

    out.push_str("# HELP nvidia_gpu_utilization_percent GPU utilization percentage\n");
    out.push_str("# TYPE nvidia_gpu_utilization_percent gauge\n");
    out.push_str(&format!("nvidia_gpu_utilization_percent {{gpu=\"{}\"}} {}\n", name, util));

    out.push_str("# HELP nvidia_gpu_memory_used_mib GPU memory used in MiB\n");
    out.push_str("# TYPE nvidia_gpu_memory_used_mib gauge\n");
    out.push_str(&format!("nvidia_gpu_memory_used_mib {{gpu=\"{}\"}} {}\n", name, mem_used));

    out.push_str("# HELP nvidia_gpu_memory_total_mib GPU total memory in MiB\n");
    out.push_str("# TYPE nvidia_gpu_memory_total_mib gauge\n");
    out.push_str(&format!("nvidia_gpu_memory_total_mib {{gpu=\"{}\"}} {}\n", name, mem_total));

    out.push_str("# HELP nvidia_gpu_temperature_celsius GPU temperature in Celsius\n");
    out.push_str("# TYPE nvidia_gpu_temperature_celsius gauge\n");
    out.push_str(&format!("nvidia_gpu_temperature_celsius {{gpu=\"{}\"}} {}\n", name, temp));

    out.push_str("# HELP nvidia_gpu_power_limit_watts GPU power limit in watts\n");
    out.push_str("# TYPE nvidia_gpu_power_limit_watts gauge\n");
    out.push_str(&format!("nvidia_gpu_power_limit_watts {{gpu=\"{}\"}} {}\n", name, power_limit_w as f64));

    // Self metrics
    out.push_str("# HELP nvidia_gpu_exporter_poll_success 1 if last poll succeeded, 0 otherwise\n");
    out.push_str("# TYPE nvidia_gpu_exporter_poll_success gauge\n");
    out.push_str(&format!("nvidia_gpu_exporter_poll_success {}\n", poll_ok));

    out.push_str("# HELP nvidia_gpu_exporter_poll_total Total number of successful polls\n");
    out.push_str("# TYPE nvidia_gpu_exporter_poll_total counter\n");
    out.push_str(&format!("nvidia_gpu_exporter_poll_total {}\n", poll_count));

    out
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Configure logging
    let log_level = match cli.verbose {
        0 => log::LevelFilter::Info,
        1 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    let _ = loggerv::Logger::new().init();
    log::set_max_level(log_level);

    info!(
        "{}",
        format!("GPU Power Exporter v1.0.0 — listening on :{}", cli.port).bold()
    );

    // Initial poll to verify nvidia-smi is available
    if !poll_gpu() {
        warn!("{}", "Initial GPU poll failed. Will keep retrying.".yellow());
        warn!("{}", "Make sure nvidia-smi is available and a GPU is present.".yellow());
    } else {
        let name = GPU_NAME.get().cloned().unwrap_or_else(|| String::from("unknown"));
        let power = GPU_POWER_MW.load(Ordering::Relaxed) as f64 / 1000.0;
        info!(
            "{}",
            format!("GPU detected: {} (current power: {:.1}W)", name, power).green()
        );
    }

    // Spawn the polling loop
    let interval_ms = cli.interval_ms;
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(tokio::time::Duration::from_millis(interval_ms));
        ticker.tick().await; // first tick completes immediately
        loop {
            ticker.tick().await;
            let ok = poll_gpu();
            if !ok {
                log::debug!("GPU poll failed (will retry next interval)");
            }
        }
    });

    // Start the HTTP server using hyper 1.x
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], cli.port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!("HTTP server listening on http://{}:{}", addr.ip(), cli.port);

    loop {
        let (stream, _peer) = listener.accept().await.expect("accept failed");

        tokio::spawn(async move {
            use hyper::server::conn::http1;
            use hyper::service::service_fn;
            use hyper::{Request, Response};
            use hyper::body::Incoming;
            use hyper_util::rt::TokioIo;

            let io = TokioIo::new(stream);

            let service = service_fn(|_req: Request<Incoming>| {
                async move {
                    let body = render_metrics();
                    let bytes = bytes::Bytes::from(body);
                    let mut resp = Response::new(http_body_util::Full::new(bytes));
                    resp.headers_mut().insert(
                        hyper::header::CONTENT_TYPE,
                        hyper::header::HeaderValue::from_static("text/plain; version=0.0.4; charset=utf-8"),
                    );
                    Ok::<_, hyper::Error>(resp)
                }
            });

            if let Err(e) = http1::Builder::new().serve_connection(io, service).await {
                error!("Connection error: {}", e);
            }
        });
    }
}
