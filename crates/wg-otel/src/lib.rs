#![forbid(unsafe_code)]

//! OpenTelemetry instrumentation placeholders.

/// Minimal telemetry configuration for the placeholder pipeline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TelemetryConfig {
    /// Whether telemetry export is enabled.
    pub enabled: bool,
}

/// Placeholder telemetry pipeline handle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TelemetryPipeline {
    config: TelemetryConfig,
}

impl TelemetryPipeline {
    /// Creates a new placeholder telemetry pipeline.
    #[must_use]
    pub const fn new(config: TelemetryConfig) -> Self {
        Self { config }
    }

    /// Returns the current placeholder telemetry configuration.
    #[must_use]
    pub const fn config(&self) -> TelemetryConfig {
        self.config
    }
}

impl Default for TelemetryPipeline {
    fn default() -> Self {
        Self::new(TelemetryConfig { enabled: false })
    }
}
