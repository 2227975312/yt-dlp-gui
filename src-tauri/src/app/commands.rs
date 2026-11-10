//! 不属于业务域的应用级 Tauri 命令。

use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem};

use super::cli::CliOpenRequest;

#[derive(Default)]
pub(crate) struct CliRequestState(Mutex<Option<CliOpenRequest>>);

impl CliRequestState {
    pub(crate) fn new(request: Option<CliOpenRequest>) -> Self {
        Self(Mutex::new(request))
    }
}

#[tauri::command]
pub(crate) fn take_cli_open_request(
    state: tauri::State<'_, CliRequestState>,
) -> Result<Option<CliOpenRequest>, String> {
    state
        .0
        .lock()
        .map(|mut request| request.take())
        .map_err(|e| format!("err_cli_request_state:{}", e))
}

#[tauri::command]
pub(crate) fn update_tray_menu(
    app: tauri::AppHandle,
    show_label: String,
    quit_label: String,
) -> Result<(), String> {
    if let Some(tray) = app.tray_by_id("main") {
        let show = MenuItem::with_id(&app, "show", &show_label, true, None::<&str>)
            .map_err(|e| e.to_string())?;
        let quit = MenuItem::with_id(&app, "quit", &quit_label, true, None::<&str>)
            .map_err(|e| e.to_string())?;
        let menu = Menu::with_items(&app, &[&show, &quit]).map_err(|e| e.to_string())?;
        tray.set_menu(Some(menu)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn set_tray_visible(app: tauri::AppHandle, visible: bool) -> Result<(), String> {
    let tray = app
        .tray_by_id("main")
        .ok_or_else(|| "system tray is not available".to_string())?;
    tray.set_visible(visible).map_err(|e| e.to_string())
}
