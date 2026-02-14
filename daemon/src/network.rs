use log::warn;

const HOSTAPD_CONF_PATH: &str = "/tmp/hostapd_wlan0.conf";
const WIFI_CREDS_PATH: &str = "/data/rayhunter/wifi-creds.conf";
const WLAN1_OPERSTATE_PATH: &str = "/sys/class/net/wlan1/operstate";

pub async fn read_ap_credentials() -> (Option<String>, Option<String>) {
    use std::os::unix::fs::PermissionsExt;
    let path = std::path::Path::new(HOSTAPD_CONF_PATH);

    // hostapd config is generated asynchronously at boot; retry if not yet present
    for attempt in 0..10 {
        if path.exists() {
            break;
        }
        if attempt == 0 {
            warn!("{HOSTAPD_CONF_PATH} not found, waiting for hostapd...");
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    if let Err(e) = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o400)) {
        warn!("failed to chmod {HOSTAPD_CONF_PATH}: {e}");
        return (None, None);
    }

    let contents = match tokio::fs::read_to_string(path).await {
        Ok(c) => c,
        Err(e) => {
            warn!("failed to read {HOSTAPD_CONF_PATH}: {e}");
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000));
            return (None, None);
        }
    };

    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000));

    let mut ssid = None;
    let mut password = None;
    for line in contents.lines() {
        if let Some(val) = line.strip_prefix("ssid=") {
            ssid = Some(val.to_string());
        } else if let Some(val) = line.strip_prefix("wpa_passphrase=") {
            password = Some(val.to_string());
        }
    }
    (ssid, password)
}

pub fn read_wifi_ssid() -> Option<String> {
    let contents = std::fs::read_to_string(WIFI_CREDS_PATH).ok()?;
    for line in contents.lines() {
        if let Some(val) = line.strip_prefix("ssid=") {
            return Some(val.to_string());
        }
    }
    None
}

pub async fn poll_wifi_status() -> (bool, Option<String>) {
    let connected = match tokio::fs::read_to_string(WLAN1_OPERSTATE_PATH).await {
        Ok(s) => s.trim() == "up",
        Err(_) => false,
    };

    if !connected {
        return (false, None);
    }

    let ip = match tokio::process::Command::new("ip")
        .args(["addr", "show", "wlan1"])
        .output()
        .await
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.lines().find_map(|line| {
                let trimmed = line.trim();
                trimmed
                    .strip_prefix("inet ")
                    .and_then(|rest| rest.split('/').next())
                    .map(|s| s.to_string())
            })
        }
        Err(_) => None,
    };

    (true, ip)
}
