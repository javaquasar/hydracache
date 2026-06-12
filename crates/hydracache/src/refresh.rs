use std::time::Duration;

/// Refresh behavior for loader-based cache reads.
///
/// `RefreshOptions` is opt-in and does not affect [`HydraCache::get_or_load`].
/// Use it with [`HydraCache::get_or_load_with_refresh`] when an application can
/// tolerate a recently expired value while a refresh is running or when a
/// loader temporarily fails.
///
/// # Example
///
/// ```rust
/// use std::time::Duration;
///
/// use hydracache::RefreshOptions;
///
/// let options = RefreshOptions::new()
///     .refresh_ahead(Duration::from_secs(10))
///     .stale_while_revalidate(Duration::from_secs(300))
///     .stale_on_loader_error(Duration::from_secs(600))
///     .serve_stale_on_loader_error(true);
///
/// assert_eq!(options.refresh_ahead_value(), Some(Duration::from_secs(10)));
/// assert_eq!(
///     options.stale_while_revalidate_value(),
///     Some(Duration::from_secs(300))
/// );
/// assert_eq!(
///     options.stale_on_loader_error_value(),
///     Some(Duration::from_secs(600))
/// );
/// assert!(options.serve_stale_on_loader_error_value());
/// ```
///
/// [`HydraCache::get_or_load`]: crate::HydraCache::get_or_load
/// [`HydraCache::get_or_load_with_refresh`]: crate::HydraCache::get_or_load_with_refresh
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RefreshOptions {
    stale_while_revalidate: Option<Duration>,
    stale_on_loader_error: Option<Duration>,
    refresh_ahead: Option<Duration>,
    serve_stale_on_loader_error: bool,
}

impl RefreshOptions {
    /// Create empty refresh options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return recently expired values for this window while refreshing in the
    /// background.
    pub fn stale_while_revalidate(mut self, window: Duration) -> Self {
        self.stale_while_revalidate = Some(window);
        self
    }

    /// Use a stale value for this window when the foreground loader fails.
    pub fn stale_on_loader_error(mut self, window: Duration) -> Self {
        self.stale_on_loader_error = Some(window);
        self.serve_stale_on_loader_error = true;
        self
    }

    /// Refresh a still-fresh value in the background when it is this close to
    /// expiration.
    pub fn refresh_ahead(mut self, threshold: Duration) -> Self {
        self.refresh_ahead = Some(threshold);
        self
    }

    /// Return a stale value when the foreground loader fails.
    ///
    /// When [`RefreshOptions::stale_on_loader_error`] is not configured, this
    /// uses the [`RefreshOptions::stale_while_revalidate`] window as the
    /// fallback window.
    pub fn serve_stale_on_loader_error(mut self, enabled: bool) -> Self {
        self.serve_stale_on_loader_error = enabled;
        self
    }

    /// Return the stale-while-revalidate window.
    pub fn stale_while_revalidate_value(&self) -> Option<Duration> {
        self.stale_while_revalidate
    }

    /// Return the refresh-ahead threshold.
    pub fn refresh_ahead_value(&self) -> Option<Duration> {
        self.refresh_ahead
    }

    /// Return the explicit stale-on-loader-error window.
    pub fn stale_on_loader_error_value(&self) -> Option<Duration> {
        self.stale_on_loader_error
    }

    /// Return whether loader failures may fall back to stale values.
    pub fn serve_stale_on_loader_error_value(&self) -> bool {
        self.serve_stale_on_loader_error
    }

    pub(crate) fn stale_on_loader_error_window(&self) -> Option<Duration> {
        if !self.serve_stale_on_loader_error {
            return None;
        }
        self.stale_on_loader_error.or(self.stale_while_revalidate)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::RefreshOptions;

    #[test]
    fn refresh_options_default_to_strict_cache_behavior() {
        let options = RefreshOptions::new();

        assert_eq!(options.stale_while_revalidate_value(), None);
        assert_eq!(options.stale_on_loader_error_value(), None);
        assert_eq!(options.refresh_ahead_value(), None);
        assert_eq!(options.stale_on_loader_error_window(), None);
        assert!(!options.serve_stale_on_loader_error_value());
    }

    #[test]
    fn refresh_options_builder_sets_all_values() {
        let options = RefreshOptions::new()
            .stale_while_revalidate(Duration::from_secs(60))
            .stale_on_loader_error(Duration::from_secs(120))
            .refresh_ahead(Duration::from_secs(5))
            .serve_stale_on_loader_error(true);

        assert_eq!(
            options.stale_while_revalidate_value(),
            Some(Duration::from_secs(60))
        );
        assert_eq!(
            options.stale_on_loader_error_value(),
            Some(Duration::from_secs(120))
        );
        assert_eq!(options.refresh_ahead_value(), Some(Duration::from_secs(5)));
        assert_eq!(
            options.stale_on_loader_error_window(),
            Some(Duration::from_secs(120))
        );
        assert!(options.serve_stale_on_loader_error_value());
    }

    #[test]
    fn refresh_options_can_reuse_stale_while_revalidate_window_for_loader_errors() {
        let options = RefreshOptions::new()
            .stale_while_revalidate(Duration::from_secs(60))
            .serve_stale_on_loader_error(true);

        assert_eq!(
            options.stale_on_loader_error_window(),
            Some(Duration::from_secs(60))
        );
    }
}
