//! # Sensors: to get data related to energy consumption
//!
//! `Sensor` is the root for all sensors. It defines the [Sensor] trait
//! needed to implement a sensor.

pub mod core;
pub mod domain;
pub mod powercap_rapl;
pub mod record;
pub mod socket;
pub mod topology;
pub mod traits;
pub mod units;
pub mod utils;

// Re-exports to preserve the public API
pub use core::CPUCore;
pub use domain::Domain;
pub use record::{CPUStat, Record};
pub use socket::CPUSocket;
pub use topology::Topology;
pub use traits::{RecordGenerator, RecordReader, Sensor};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn get_proc_cpuinfo() {
        let cores = Topology::generate_cpu_cores().unwrap();
        println!(
            "cores: {} attributes in core 0: {}",
            cores.len(),
            cores[0].attributes.len()
        );
        for c in &cores {
            println!("{:?}", c.attributes);
        }
        assert_eq!(!cores.is_empty(), true);
        for c in &cores {
            assert_eq!(c.attributes.len() > 3, true);
        }
    }

    #[test]
    fn read_topology_stats() {
        let sensor = powercap_rapl::PowercapRAPLSensor::new(8, 8, false);
        let topo = (*sensor.get_topology()).unwrap();
        println!("{:?}", topo.read_stats());
    }

    #[test]
    fn read_core_stats() {
        let sensor = powercap_rapl::PowercapRAPLSensor::new(8, 8, false);
        let mut topo = (*sensor.get_topology()).unwrap();
        for s in topo.get_sockets() {
            for c in s.get_cores() {
                println!("{:?}", c.read_stats());
            }
        }
    }

    #[test]
    fn read_socket_stats() {
        let sensor = powercap_rapl::PowercapRAPLSensor::new(8, 8, false);
        let mut topo = (*sensor.get_topology()).unwrap();
        for s in topo.get_sockets() {
            println!("{:?}", s.read_stats());
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
