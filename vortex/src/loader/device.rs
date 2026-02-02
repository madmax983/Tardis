//! Device selection and creation.
//!
//! Handles parsing device specifications and creating Candle devices.

#[cfg(any(feature = "cuda", feature = "metal"))]
use crate::error::VortexError;
use crate::error::VortexResult;
use candle_core::Device;

/// Device specification parsed from string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceSpec {
    /// CPU device.
    Cpu,
    /// CUDA GPU with device ID.
    Cuda(usize),
    /// Metal GPU (macOS).
    Metal,
}

impl DeviceSpec {
    /// Parse a device specification string.
    ///
    /// Supported formats:
    /// - "cpu" -> CPU
    /// - "cuda" or "cuda:0" -> CUDA device 0
    /// - "cuda:N" -> CUDA device N
    /// - "metal" -> Metal (macOS)
    #[must_use]
    pub fn parse(spec: &str) -> Self {
        let spec = spec.to_lowercase();

        if spec == "cpu" {
            return Self::Cpu;
        }

        if spec == "metal" {
            return Self::Metal;
        }

        if spec == "cuda" || spec == "gpu" {
            return Self::Cuda(0);
        }

        if let Some(id_str) = spec.strip_prefix("cuda:") {
            if let Ok(id) = id_str.parse::<usize>() {
                return Self::Cuda(id);
            }
        }

        // Default to CPU for unrecognized specs
        tracing::warn!("Unrecognized device spec '{}', defaulting to CPU", spec);
        Self::Cpu
    }
}

/// Create a Candle device from a specification.
///
/// # Errors
///
/// Returns an error if the requested device is not available.
pub fn create_device(spec: &DeviceSpec) -> VortexResult<Device> {
    match spec {
        DeviceSpec::Cpu => {
            tracing::info!("Using CPU device");
            Ok(Device::Cpu)
        }
        DeviceSpec::Cuda(ordinal) => {
            #[cfg(feature = "cuda")]
            {
                tracing::info!("Using CUDA device {}", ordinal);
                Device::new_cuda(*ordinal).map_err(|e| {
                    VortexError::DeviceError(format!(
                        "Failed to create CUDA device {}: {}",
                        ordinal, e
                    ))
                })
            }
            #[cfg(not(feature = "cuda"))]
            {
                tracing::warn!(
                    "CUDA requested but not compiled with cuda feature, falling back to CPU"
                );
                let _ = ordinal; // Suppress unused warning
                Ok(Device::Cpu)
            }
        }
        DeviceSpec::Metal => {
            #[cfg(feature = "metal")]
            {
                tracing::info!("Using Metal device");
                Device::new_metal(0).map_err(|e| {
                    VortexError::DeviceError(format!("Failed to create Metal device: {}", e))
                })
            }
            #[cfg(not(feature = "metal"))]
            {
                tracing::warn!(
                    "Metal requested but not compiled with metal feature, falling back to CPU"
                );
                Ok(Device::Cpu)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cpu() {
        assert_eq!(DeviceSpec::parse("cpu"), DeviceSpec::Cpu);
        assert_eq!(DeviceSpec::parse("CPU"), DeviceSpec::Cpu);
    }

    #[test]
    fn test_parse_cuda() {
        assert_eq!(DeviceSpec::parse("cuda"), DeviceSpec::Cuda(0));
        assert_eq!(DeviceSpec::parse("cuda:0"), DeviceSpec::Cuda(0));
        assert_eq!(DeviceSpec::parse("cuda:1"), DeviceSpec::Cuda(1));
        assert_eq!(DeviceSpec::parse("gpu"), DeviceSpec::Cuda(0));
    }

    #[test]
    fn test_parse_metal() {
        assert_eq!(DeviceSpec::parse("metal"), DeviceSpec::Metal);
        assert_eq!(DeviceSpec::parse("Metal"), DeviceSpec::Metal);
    }

    #[test]
    fn test_parse_unknown() {
        // Unknown specs should default to CPU
        assert_eq!(DeviceSpec::parse("unknown"), DeviceSpec::Cpu);
    }

    #[test]
    fn test_create_cpu_device() {
        let device = create_device(&DeviceSpec::Cpu).unwrap();
        assert!(matches!(device, Device::Cpu));
    }
}
