use std::{path::PathBuf, time::Duration};

use log::{info, warn};
use rayhunter::Device;
use serde::Serialize;
use tokio::select;
use tokio_util::{sync::CancellationToken, task::TaskTracker};

use crate::{
    error::RayhunterError,
    notifications::{Notification, NotificationType},
};

use crate::display::DeviceInfoHandle;

pub mod orbic;
pub mod tmobile;
pub mod tplink;
pub mod wingtech;

const LOW_BATTERY_LEVEL: u8 = 10;

/// Device battery information
#[derive(Clone, Copy, PartialEq, Debug, Serialize)]
#[cfg_attr(feature = "apidocs", derive(utoipa::ToSchema))]
pub struct BatteryState {
    /// The current level in percentage of the device battery
    level: u8,
    /// A boolean indicating whether the battery is currently being charged
    is_plugged_in: bool,
}

async fn is_plugged_in_from_file(path: &std::path::Path) -> Result<bool, RayhunterError> {
    match tokio::fs::read_to_string(path)
        .await
        .map_err(RayhunterError::TokioError)?
        .chars()
        .next()
    {
        Some('0') => Ok(false),
        Some('1') => Ok(true),
        _ => Err(RayhunterError::BatteryPluggedInStatusParseError),
    }
}

async fn get_level_from_percentage_file(path: &std::path::Path) -> Result<u8, RayhunterError> {
    tokio::fs::read_to_string(path)
        .await
        .map_err(RayhunterError::TokioError)?
        .trim_end()
        .parse()
        .or(Err(RayhunterError::BatteryLevelParseError))
}

pub async fn get_battery_status(device: &Device) -> Result<BatteryState, RayhunterError> {
    Ok(match device {
        Device::Orbic => orbic::get_battery_state().await?,
        Device::Wingtech => wingtech::get_battery_state().await?,
        Device::Tmobile => tmobile::get_battery_state().await?,
        Device::Tplink => tplink::get_battery_state().await?,
        _ => return Err(RayhunterError::FunctionNotSupportedForDeviceError),
    })
}

#[allow(dead_code)]
pub struct SystemPollerConfig {
    pub device: Device,
    pub notification_channel: tokio::sync::mpsc::Sender<Notification>,
    pub shutdown_token: CancellationToken,
    pub device_handle: Option<DeviceInfoHandle>,
    pub qmdl_store_path: PathBuf,
}

pub fn run_system_poller(task_tracker: &TaskTracker, config: SystemPollerConfig) {
    let SystemPollerConfig {
        device,
        notification_channel,
        shutdown_token,
        device_handle,
        #[cfg(feature = "tft-ui")]
        qmdl_store_path,
        #[cfg(not(feature = "tft-ui"))]
            qmdl_store_path: _,
    } = config;

    task_tracker.spawn(async move {
        let battery_supported = !matches!(
            get_battery_status(&device).await,
            Err(RayhunterError::FunctionNotSupportedForDeviceError)
        );

        if !battery_supported {
            info!("Battery status not supported for this device, disabling battery notifications");
        }

        let mut triggered = match get_battery_status(&device).await {
            Ok(status) => status.level <= LOW_BATTERY_LEVEL,
            Err(_) => true,
        };

        loop {
            select! {
                _ = shutdown_token.cancelled() => break,
                _ = tokio::time::sleep(Duration::from_secs(15)) => {}
            }

            let battery_status = if battery_supported {
                match get_battery_status(&device).await {
                    Err(e) => {
                        warn!("Failed to get battery status: {e}");
                        None
                    }
                    Ok(status) => Some(status),
                }
            } else {
                None
            };

            #[cfg(feature = "tft-ui")]
            if let Some(ref h) = device_handle {
                let qmdl_path = qmdl_store_path.to_str().unwrap_or_default().to_string();
                h.update(|info| {
                    if let Some(status) = battery_status {
                        info.battery_level = Some(status.level);
                        info.battery_plugged = status.is_plugged_in;
                    }
                    if let Ok(disk) = crate::stats::DiskStats::new(&qmdl_path) {
                        if let Some(avail) = disk.available_bytes {
                            info.disk_available_mb = (avail / (1024 * 1024)) as u32;
                        }
                        if let Some(total) = disk.total_bytes {
                            info.disk_total_mb = (total / (1024 * 1024)) as u32;
                        }
                    }
                    if let Ok((total_kb, avail_kb)) = crate::stats::read_memory_kb() {
                        info.mem_total_mb = (total_kb / 1024) as u32;
                        info.mem_free_mb = (avail_kb / 1024) as u32;
                    }
                    if let Ok(secs) = crate::stats::read_uptime_secs() {
                        info.uptime_secs = secs;
                    }
                })
                .await;
            }

            #[cfg(not(feature = "tft-ui"))]
            if let Some(ref h) = device_handle
                && let Some(status) = battery_status
            {
                h.update(|info| {
                    info.battery_level = Some(status.level);
                    info.battery_plugged = status.is_plugged_in;
                })
                .await;
            }

            if let Some(status) = battery_status {
                if triggered && status.is_plugged_in && status.level > LOW_BATTERY_LEVEL {
                    triggered = false;
                    continue;
                }
                if !triggered && !status.is_plugged_in && status.level <= LOW_BATTERY_LEVEL {
                    notification_channel
                        .send(Notification::new(
                            NotificationType::LowBattery,
                            "Rayhunter's battery is low".to_string(),
                            None,
                        ))
                        .await
                        .expect("Failed to send to notification channel");
                    triggered = true;
                }
            }
        }
    });
}
