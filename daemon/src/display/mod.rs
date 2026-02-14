use rayhunter::analysis::analyzer::EventType;
use serde::{Deserialize, Serialize};

#[cfg(feature = "orbic-ui")]
use std::sync::Arc;
#[cfg(feature = "orbic-ui")]
use tokio::sync::{Notify, RwLock};

mod generic_framebuffer;

pub mod headless;
pub mod orbic;
pub mod tmobile;
pub mod tplink;
pub mod tplink_framebuffer;
pub mod tplink_onebit;
pub mod uz801;
pub mod wingtech;

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DisplayState {
    /// We're recording but no warning has been found yet.
    Recording,
    /// We're not recording.
    Paused,
    /// A non-informational event has been detected.
    ///
    /// Note that EventType::Informational is never sent through this. If it is, it's the same as
    /// Recording
    WarningDetected { event_type: EventType },
}

#[cfg(feature = "orbic-ui")]
#[derive(Clone, Copy)]
pub enum StoppedReason {
    DiskFull,
    DiagError,
}

#[cfg(feature = "orbic-ui")]
pub struct DeviceInfo {
    pub display_state: DisplayState,
    pub disk_available_mb: u32,
    pub disk_total_mb: u32,
    pub battery_level: Option<u8>,
    pub battery_plugged: bool,
    pub colorblind_mode: bool,
    pub mem_total_mb: u32,
    pub mem_free_mb: u32,
    pub uptime_secs: u64,
    pub version: &'static str,
    pub ap_ssid: Option<String>,
    pub ap_password: Option<String>,
    pub ap_port: u16,
    pub wifi_ssid: Option<String>,
    pub wifi_connected: bool,
    pub wifi_ip: Option<String>,
    pub event_counts: [u32; 4],
    pub last_event_time: Option<String>,
    pub last_event_name: Option<String>,
    pub last_event_severity: Option<EventType>,
    pub low_disk: bool,
    pub stopped_reason: Option<StoppedReason>,
    pub wake_display: bool,
    pub mcc_mnc: Option<String>,
    pub rsrp_dbm: Option<i16>,
}

#[cfg(feature = "orbic-ui")]
impl DeviceInfo {
    pub fn new(colorblind_mode: bool, ap_port: u16) -> Self {
        Self {
            display_state: DisplayState::Recording,
            disk_available_mb: 0,
            disk_total_mb: 0,
            battery_level: None,
            battery_plugged: false,
            colorblind_mode,
            mem_total_mb: 0,
            mem_free_mb: 0,
            uptime_secs: 0,
            version: env!("CARGO_PKG_VERSION"),
            ap_ssid: None,
            ap_password: None,
            ap_port,
            wifi_ssid: None,
            wifi_connected: false,
            wifi_ip: None,
            event_counts: [0; 4],
            last_event_time: None,
            last_event_name: None,
            last_event_severity: None,
            low_disk: false,
            stopped_reason: None,
            wake_display: false,
            mcc_mnc: None,
            rsrp_dbm: None,
        }
    }
}

#[cfg(feature = "orbic-ui")]
#[derive(Clone)]
pub struct DeviceInfoHandle {
    inner: Arc<RwLock<DeviceInfo>>,
    notify: Arc<Notify>,
}

#[cfg(feature = "orbic-ui")]
impl DeviceInfoHandle {
    pub fn new(info: DeviceInfo) -> Self {
        Self {
            inner: Arc::new(RwLock::new(info)),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Update DeviceInfo and wake the display loop.
    pub async fn update(&self, f: impl FnOnce(&mut DeviceInfo)) {
        f(&mut *self.inner.write().await);
        self.notify.notify_one();
    }

    pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, DeviceInfo> {
        self.inner.read().await
    }

    pub async fn write(&self) -> tokio::sync::RwLockWriteGuard<'_, DeviceInfo> {
        self.inner.write().await
    }

    pub fn notify_ref(&self) -> &Notify {
        &self.notify
    }
}
