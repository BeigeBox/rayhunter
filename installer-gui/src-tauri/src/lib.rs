use std::sync::Mutex;
use std::time::Duration;

use anyhow::Context;
use tauri::Emitter;
use tokio::task::AbortHandle;

struct InstallerState(Mutex<Option<AbortHandle>>);

async fn run_installer(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, InstallerState>,
    args: String,
) -> anyhow::Result<()> {
    let args_vec = shlex::split(&args).context("Failed to parse arguments: unclosed quote")?;
    let handle = tokio::task::spawn_blocking(move || {
        installer::run_with_callback(
            args_vec.iter().map(|s| s.as_str()),
            Some(Box::new(move |output| {
                app_handle
                    .emit("installer-output", output)
                    .expect("Error sending Rayhunter CLI installer output to GUI frontend");
            })),
        )
    });
    *state.0.lock().unwrap() = Some(handle.abort_handle());
    match handle.await {
        Ok(result) => result,
        Err(e) if e.is_cancelled() => anyhow::bail!("Installation cancelled."),
        Err(e) => Err(e.into()),
    }
}

#[tauri::command]
async fn install_rayhunter(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, InstallerState>,
    args: String,
) -> Result<(), String> {
    run_installer(app_handle, state, args)
        .await
        .map_err(|error| format!("{error:?}"))
}

#[tauri::command]
async fn cancel_installer(state: tauri::State<'_, InstallerState>) -> Result<(), String> {
    if let Some(handle) = state.0.lock().unwrap().take() {
        handle.abort();
    }
    Ok(())
}

#[tauri::command]
async fn check_device_reachable(ip: String, port: u16) -> Result<bool, String> {
    let addr = format!("{ip}:{port}");
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
        .manage(InstallerState(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            install_rayhunter,
            cancel_installer,
            check_device_reachable,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
