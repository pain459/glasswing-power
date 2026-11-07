//! glasswing-power is an extensible monitoring agent for energy consumption metrics.
//!
//! It gathers energy consumption data from the system or other data sources thanks to components called *sensors*.
//!
//! Final monitoring data is sent to or exposed for monitoring tools thanks to *exporters*.
#[macro_use]
extern crate log;
pub mod exporters;
pub mod sensors;

use sensors::powercap_rapl;

/// Create a new [`Sensor`] instance with the default sensor available,
/// with its default options.
pub fn get_default_sensor() -> impl sensors::Sensor {
    powercap_rapl::PowercapRAPLSensor::new(
        powercap_rapl::DEFAULT_BUFFER_PER_SOCKET_MAX_KBYTES,
        powercap_rapl::DEFAULT_BUFFER_PER_DOMAIN_MAX_KBYTES,
        false,
    )
}

//  Copyright 2020 The scaphandre authors.
//  Copyright 2026 pain459
//
//  Licensed under the MIT License.
//  You may obtain a copy of the License at
//
//      https://opensource.org/licenses/MIT
