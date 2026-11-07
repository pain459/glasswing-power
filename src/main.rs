//! Generic sensor and transmission agent for energy consumption related metrics.

use clap::{ArgAction, Parser, Subcommand};
use colored::Colorize;
use glasswing_power::{exporters, sensors::Sensor};

use glasswing_power::sensors::powercap_rapl;

// the struct below defines the main Glasswing Power command-line interface
/// Extensible metrology agent for electricity consumption related metrics.
#[derive(Parser)]
#[command(author, version, name = "glasswing-power")]
struct Cli {
    /// The exporter module to use to output the energy consumption metrics
    #[command(subcommand)]
    exporter: ExporterChoice,

    /// Increase the verbosity level
    #[arg(short, action = ArgAction::Count, default_value_t = 0)]
    verbose: u8,

    /// Don't print the header to the standard output
    #[arg(long, default_value_t = false)]
    no_header: bool,

    /// Tell Glasswing Power that it's running in a virtual machine.
    /// You should have another instance of Glasswing Power running on the hypervisor (see docs).
    #[arg(long, default_value_t = false)]
    vm: bool,

    /// The sensor module to use to gather the energy consumption metrics
    #[arg(short, long)]
    sensor: Option<String>,

    /// Maximum memory size allowed, in KiloBytes, for storing energy consumption of each **domain**.
    /// Only available for the RAPL sensor.
    #[arg(long, default_value_t = powercap_rapl::DEFAULT_BUFFER_PER_DOMAIN_MAX_KBYTES)]
    sensor_buffer_per_domain_max_kb: u16,

    /// Maximum memory size allowed, in KiloBytes, for storing energy consumption of each **socket**.
    /// Only available for the RAPL sensor.
    #[arg(long, default_value_t = powercap_rapl::DEFAULT_BUFFER_PER_SOCKET_MAX_KBYTES)]
    sensor_buffer_per_socket_max_kb: u16,
}

/// Defines the possible subcommands, one per exporter.
///
/// ### Description style
/// Per the clap documentation, the description of commands and arguments should be written in the style applied here,
/// *not* in the third-person. That is, use "Do xyz" instead of "Does xyz".
#[derive(Subcommand)]
enum ExporterChoice {
    /// Write the metrics to the terminal
    Stdout(exporters::stdout::ExporterArgs),

    /// Write the metrics in the JSON format to a file or to stdout
    #[cfg(feature = "json")]
    Json(exporters::json::ExporterArgs),

    /// Expose the metrics to a Prometheus HTTP endpoint
    #[cfg(feature = "prometheus")]
    Prometheus(exporters::prometheus::ExporterArgs),

    /// Watch all Qemu-KVM virtual machines running on the host and expose the metrics
    /// of each of them in a dedicated folder
    #[cfg(feature = "qemu")]
    Qemu,

    /// Expose the metrics to a Riemann server
    #[cfg(feature = "riemann")]
    Riemann(exporters::riemann::ExporterArgs),

    /// Expose the metrics to a Warp10 host, through HTTP
    #[cfg(feature = "warpten")]
    Warpten(exporters::warpten::ExporterArgs),

    /// Push metrics to Prometheus Push Gateway
    #[cfg(feature = "prometheuspush")]
    PrometheusPush(exporters::prometheuspush::ExporterArgs),
}

fn main() {
    parse_cli_and_run_exporter();
}

fn parse_cli_and_run_exporter() {
    let cli = Cli::parse();
    loggerv::init_with_verbosity(cli.verbose.into()).expect("unable to initialize the logger");

    let sensor = build_sensor(
        cli.sensor_buffer_per_socket_max_kb,
        cli.sensor_buffer_per_domain_max_kb,
        cli.vm,
        cli.sensor,
    );
    let mut exporter = build_exporter(cli.exporter, &sensor);
    if !cli.no_header {
        print_glasswing_power_header(exporter.kind());
    }

    exporter.run();
}

fn build_exporter(choice: ExporterChoice, sensor: &dyn Sensor) -> Box<dyn exporters::Exporter> {
    match choice {
        ExporterChoice::Stdout(args) => {
            Box::new(exporters::stdout::StdoutExporter::new(sensor, args))
        }
        #[cfg(feature = "json")]
        ExporterChoice::Json(args) => {
            Box::new(exporters::json::JsonExporter::new(sensor, args)) // keep this in braces
        }
        #[cfg(feature = "prometheus")]
        ExporterChoice::Prometheus(args) => {
            Box::new(exporters::prometheus::PrometheusExporter::new(sensor, args))
        }
        #[cfg(feature = "qemu")]
        ExporterChoice::Qemu => {
            Box::new(exporters::qemu::QemuExporter::new(sensor)) // keep this in braces
        }
        #[cfg(feature = "riemann")]
        ExporterChoice::Riemann(args) => {
            Box::new(exporters::riemann::RiemannExporter::new(sensor, args))
        }
        #[cfg(feature = "warpten")]
        ExporterChoice::Warpten(args) => {
            Box::new(exporters::warpten::Warp10Exporter::new(sensor, args))
        }
        #[cfg(feature = "prometheuspush")]
        ExporterChoice::PrometheusPush(args) => Box::new(
            exporters::prometheuspush::PrometheusPushExporter::new(sensor, args),
        ),
    }
    // Note that invalid choices are automatically turned into errors by `parse()` before the Cli is populated,
    // that's why they don't appear in this function.
}

/// Returns the sensor to use, given the command-line arguments.
/// Unless sensor-specific options are provided, this should return
/// the same thing as [`glasswing_power::get_default_sensor`].
fn build_sensor(
    sensor_buffer_per_socket_max_kb: u16,
    sensor_buffer_per_domain_max_kb: u16,
    vm: bool,
    sensor: Option<String>,
) -> impl Sensor {
    match sensor.as_deref() {
        Some("powercap_rapl") | None => {
            powercap_rapl::PowercapRAPLSensor::new(
                sensor_buffer_per_socket_max_kb,
                sensor_buffer_per_domain_max_kb,
                vm,
            )
        }
        Some(s) => panic!("Unknown sensor type {}", s),
    }
}

fn print_glasswing_power_header(exporter_name: &str) {
    let title = format!("Glasswing Power {exporter_name} exporter");
    println!("{}", title.red().bold());
    println!("Sending ⚡ metrics");
}

#[cfg(test)]
mod test {
    use super::*;

    const SUBCOMMANDS: &[&str] = &[
        "stdout",
        #[cfg(feature = "prometheus")]
        "prometheus",
        #[cfg(feature = "riemann")]
        "riemann",
        #[cfg(feature = "json")]
        "json",
        #[cfg(feature = "warpten")]
        "warpten",
        #[cfg(feature = "qemu")]
        "qemu",
    ];

    /// Test that `--help` works for glasswing-power _and_ for each subcommand.
    /// This also ensures that all the subcommands are properly defined, as Clap will check some constraints
    /// when trying to parse a subcommand (for instance, it will check that no two short options have the same name).
    #[test]
    fn test_help() {
        fn assert_shows_help(args: &[&str]) {
            match Cli::try_parse_from(args) {
                Ok(_) => panic!(
                    "The CLI didn't generate a help message for {args:?}, are the inputs correct?"
                ),
                Err(e) => assert_eq!(
                    e.kind(),
                    clap::error::ErrorKind::DisplayHelp,
                    "The CLI emitted an error for {args:?}:\n{e}"
                ),
            };
        }
        assert_shows_help(&["glasswing-power", "--help"]);
        for cmd in SUBCOMMANDS {
            assert_shows_help(&["glasswing-power", cmd, "--help"]);
        }
    }
}

//  Copyright 2020 The scaphandre authors.
//  Copyright 2026 pain459
//
//  Licensed under the MIT License.
//  You may obtain a copy of the License at
//
//      https://opensource.org/licenses/MIT
