//! Explicit adapter selection policy; fallback is never implicit.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackPolicy {
    DenySoftware,
    AllowSoftware,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdapterPolicy {
    pub power_preference: wgpu::PowerPreference,
    pub fallback: FallbackPolicy,
}

impl Default for AdapterPolicy {
    fn default() -> Self {
        Self {
            power_preference: wgpu::PowerPreference::HighPerformance,
            fallback: FallbackPolicy::DenySoftware,
        }
    }
}

impl AdapterPolicy {
    pub fn request_options<'a>(
        &self,
        surface: Option<&'a wgpu::Surface<'a>>,
    ) -> wgpu::RequestAdapterOptions<'a, 'a> {
        wgpu::RequestAdapterOptions {
            power_preference: self.power_preference,
            force_fallback_adapter: false,
            compatible_surface: surface,
        }
    }

    pub fn allows_fallback(&self) -> bool {
        matches!(self.fallback, FallbackPolicy::AllowSoftware)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fallback_policy_is_explicit() {
        assert!(!AdapterPolicy::default().allows_fallback());
        assert!(AdapterPolicy {
            fallback: FallbackPolicy::AllowSoftware,
            ..AdapterPolicy::default()
        }
        .allows_fallback());
    }
}
