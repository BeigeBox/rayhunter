use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rayhunter::analysis::analyzer::{EVENT_TYPE_COUNT, EventType};
use serde::{Deserialize, Serialize};
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

/// A list of available display states
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "apidocs", derive(utoipa::ToSchema))]
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

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum StoppedReason {
    DiskFull,
    DiagError,
}

#[allow(dead_code)]
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
    pub ap_ip: Option<String>,
    pub ap_port: u16,
    pub event_counts: [u32; EVENT_TYPE_COUNT],
    pub last_event_time: Option<String>,
    pub last_event_name: Option<String>,
    pub last_event_severity: Option<EventType>,
    pub low_disk: bool,
    pub stopped_reason: Option<StoppedReason>,
    pub mcc_mnc: Option<String>,
    pub rsrp_dbm: Option<i16>,
}

#[allow(dead_code)]
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
            ap_ip: None,
            ap_port,
            event_counts: [0; EVENT_TYPE_COUNT],
            last_event_time: None,
            last_event_name: None,
            last_event_severity: None,
            low_disk: false,
            stopped_reason: None,
            mcc_mnc: None,
            rsrp_dbm: None,
        }
    }
}

#[derive(Clone)]
pub struct DeviceInfoHandle {
    inner: Arc<RwLock<DeviceInfo>>,
    notify: Arc<Notify>,
    wake_display: Arc<AtomicBool>,
}

#[allow(dead_code)]
impl DeviceInfoHandle {
    pub fn new(info: DeviceInfo) -> Self {
        Self {
            inner: Arc::new(RwLock::new(info)),
            notify: Arc::new(Notify::new()),
            wake_display: Arc::new(AtomicBool::new(false)),
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

    pub fn notify_ref(&self) -> &Notify {
        &self.notify
    }

    pub fn set_wake(&self) {
        self.wake_display.store(true, Ordering::Release);
        self.notify.notify_one();
    }

    pub fn take_wake(&self) -> bool {
        self.wake_display.swap(false, Ordering::AcqRel)
    }
}
