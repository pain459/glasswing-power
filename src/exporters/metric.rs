use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

/// General metric definition.
#[derive(Debug)]
pub struct Metric {
    /// `name` is the metric name, it will be used as service field for Riemann.
    pub name: String, // Will be used as service for Riemann
    /// `metric_type` mostly used by Prometheus, define is it is a gauge, counter...
    pub metric_type: String,
    /// `ttl` time to live for this metric used by Riemann.
    #[allow(dead_code)]
    pub ttl: f32,
    /// `hostname` host that provides the metric.
    pub hostname: String,
    /// `state` used by Riemann, define a state like Ok or Ko regarding this metric.
    #[allow(dead_code)]
    pub state: String,
    /// `tags` used by Riemann, tags attached to the metric.
    #[allow(dead_code)]
    pub tags: Vec<String>,
    /// `attributes` used by exporters to better qualify the metric. In Prometheus context
    /// this is used as a metric tag (socket_id) : `scaph_self_socket_stats_nb{socket_id="0"} 2`.
    /// Note: metric names use the `scaph_` prefix for backward compatibility with existing dashboards.
    pub attributes: HashMap<String, String>,
    /// `description` metric description and units used.
    pub description: String,
    /// `metric_value` the value of the metric. This is possible to pass different types using
    /// [MetricValueType] enum. It allows to do specific exporter processing based on types
    /// allowing flexibility.
    pub metric_value: MetricValueType,
    /// `timestamp` is the timestamp of the moment of the data measurement, stored as a Duration
    pub timestamp: Duration,
}

pub enum MetricValueType {
    // IntSigned(i64),
    // Float(f32),
    Text(String),
    //FloatDouble(f64),
    IntUnsigned(u64),
}

impl fmt::Display for MetricValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            // MetricValueType::IntSigned(value) => write!(f, "{}", value),
            // MetricValueType::Float(value) => write!(f, "{}", value),
            MetricValueType::Text(text) => write!(f, "{text}"),
            //MetricValueType::FloatDouble(value) => write!(f, "{value}"),
            MetricValueType::IntUnsigned(value) => write!(f, "{value}"),
        }
    }
}

impl fmt::Debug for MetricValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            // MetricValueType::IntSigned(value) => write!(f, "{}", value),
            // MetricValueType::Float(value) => write!(f, "{}", value),
            MetricValueType::Text(text) => write!(f, "{text}"),
            //MetricValueType::FloatDouble(value) => write!(f, "{value}"),
            MetricValueType::IntUnsigned(value) => write!(f, "{value}"),
        }
    }
}
