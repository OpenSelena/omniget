#[derive(Debug, Clone, Default)]
pub struct ProgressUpdate {
    pub percent: f64,
    pub downloaded_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub speed_bps: Option<f64>,
    pub eta_seconds: Option<u64>,
}

impl ProgressUpdate {
    pub fn percent(percent: f64) -> Self {
        Self {
            percent,
            ..Default::default()
        }
    }

    pub fn rich(
        percent: f64,
        downloaded_bytes: Option<u64>,
        total_bytes: Option<u64>,
        speed_bps: Option<f64>,
        eta_seconds: Option<u64>,
    ) -> Self {
        Self {
            percent,
            downloaded_bytes,
            total_bytes,
            speed_bps,
            eta_seconds,
        }
    }

    pub fn has_real_metrics(&self) -> bool {
        self.downloaded_bytes.is_some() || self.speed_bps.is_some() || self.eta_seconds.is_some()
    }
}

impl From<f64> for ProgressUpdate {
    fn from(percent: f64) -> Self {
        Self::percent(percent)
    }
}
