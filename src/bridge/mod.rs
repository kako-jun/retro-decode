//! Multi-language bridge for Python and TypeScript engines


#[cfg(feature = "python-bridge")]
pub mod python;

pub mod typescript;

/// Bridge configuration
#[derive(Debug, Clone)]
pub struct BridgeConfig {
    pub parallel: bool,
    pub gpu: bool,
    pub step_by_step: bool,
    pub verbose: bool,
}

impl From<&crate::Config> for BridgeConfig {
    fn from(config: &crate::Config) -> Self {
        Self {
            parallel: config.parallel,
            gpu: config.gpu,
            step_by_step: config.step_by_step,
            verbose: config.verbose,
        }
    }
}