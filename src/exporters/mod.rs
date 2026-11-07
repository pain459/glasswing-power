//! # Exporters: to make data accessible to monitoring toolchains
//!
//! `Exporter` is the root for all exporters. It defines the [Exporter] trait
//! needed to implement an exporter.
#[cfg(feature = "json")]
pub mod json;
#[cfg(feature = "prometheus")]
pub mod prometheus;
#[cfg(feature = "prometheuspush")]
pub mod prometheuspush;
pub mod qemu;
#[cfg(feature = "riemann")]
pub mod riemann;
pub mod stdout;
pub mod utils;
#[cfg(feature = "warpten")]
pub mod warpten;

pub mod generator;
pub mod metric;

// Re-exports to preserve the public API
pub use generator::{Exporter, MetricGenerator};
pub use metric::{Metric, MetricValueType};

//  Copyright 2020 The scaphandre authors.
//  Copyright 2026 pain459
//
//  Licensed under the MIT License.
//  You may obtain a copy of the License at
//
//      https://opensource.org/licenses/MIT
