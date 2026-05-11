use std::net::IpAddr;
use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, State};
use tokio_util::sync::CancellationToken;

#[derive(Default)]
struct AppState {
    install: Mutex<Option<CancellationToken>>,
}

async fn run_installer(
    app_handle: AppHandle,
    args: Vec<String>,
    cancel_token: CancellationToken,
) -> anyhow::Result<()> {
    let token_for_blocking = cancel_token.clone();
    tokio::task::spawn_blocking(move || {
        installer::run_with_callback(
            args.iter().map(|s| s.as_str()),
            Some(Box::new(move |output| {
                if let Err(e) = app_handle.emit("installer-output", output) {
                    eprintln!("failed to emit installer output: {e}");
                }
            })),
            Some(token_for_blocking),
        )
    })
    .await?
}

#[tauri::command]
async fn install_rayhunter(
    args: Vec<String>,
    password: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let cancel_token = {
        let mut guard = state.install.lock().unwrap();
        if let Some(existing) = guard.as_ref()
            && !existing.is_cancelled()
        {
            return Err("Install already running".to_string());
        }
        let token = CancellationToken::new();
        *guard = Some(token.clone());
        token
    };

    let mut full_args: Vec<String> = Vec::with_capacity(args.len() + 2);
    if let Some(pw) = password {
        full_args.push("--admin-password".to_string());
        full_args.push(pw);
    }
    full_args.extend(args);

    let result = run_installer(app, full_args, cancel_token).await;

    {
        let mut guard = state.install.lock().unwrap();
        *guard = None;
    }

    result.map_err(|error| format!("{error:#}"))
}

#[tauri::command]
async fn cancel_installer(state: State<'_, AppState>) -> Result<(), String> {
    if let Some(token) = state.install.lock().unwrap().as_ref() {
        token.cancel();
    }
    Ok(())
}

#[tauri::command]
async fn check_device_reachable(ip: String, port: u16) -> Result<bool, String> {
    let parsed: IpAddr = ip.parse().map_err(|_| "Invalid IP address".to_string())?;
    let addr = std::net::SocketAddr::new(parsed, port);
    match tokio::time::timeout(
        Duration::from_secs(3),
        tokio::net::TcpStream::connect(&addr),
    )
    .await
    {
        Ok(Ok(_)) => Ok(true),
        _ => Ok(false),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(AppState::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            install_rayhunter,
            cancel_installer,
            check_device_reachable,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn check_device_reachable_rejects_hostname() {
        let err = check_device_reachable("example.com".to_string(), 80)
            .await
            .unwrap_err();
        assert_eq!(err, "Invalid IP address");
    }

    #[tokio::test]
    async fn check_device_reachable_rejects_garbage() {
        let err = check_device_reachable("not an ip".to_string(), 80)
            .await
            .unwrap_err();
        assert_eq!(err, "Invalid IP address");
    }

    #[tokio::test]
    async fn check_device_reachable_accepts_ipv4_literal() {
        let result = check_device_reachable("127.0.0.1".to_string(), 1).await;
        assert!(matches!(result, Ok(false) | Ok(true)));
    }

    #[tokio::test]
    async fn check_device_reachable_accepts_ipv6_literal() {
        let result = check_device_reachable("::1".to_string(), 1).await;
        assert!(matches!(result, Ok(false) | Ok(true)));
    }
}
