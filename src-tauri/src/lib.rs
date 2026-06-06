use std::sync::Mutex;

use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, WebviewUrl, WindowEvent};
use tauri::webview::WebviewBuilder;

const MAIN_WINDOW: &str = "main";
const WEBVIEW_PREFIX: &str = "target-";

#[derive(Default)]
struct WebviewState {
    active_label: Mutex<Option<String>>,
    sidebar_width: Mutex<f64>,
}

#[tauri::command]
fn select_target(
    app: AppHandle,
    state: tauri::State<WebviewState>,
    id: String,
    url: String,
    sidebar_width: f64,
) -> Result<(), String> {
    *state.sidebar_width.lock().map_err(|error| error.to_string())? = sidebar_width;

    let label = target_label(&id);
    ensure_webview(&app, &label, &url, sidebar_width)?;

    if let Some(window) = app.get_window(MAIN_WINDOW) {
        for webview in window.webviews() {
            if is_target_webview(webview.label()) {
                if webview.label() == label {
                    webview.show().map_err(|error| error.to_string())?;
                    webview.set_focus().map_err(|error| error.to_string())?;
                } else {
                    let _ = webview.hide();
                }
            }
        }
    }

    *state.active_label.lock().map_err(|error| error.to_string())? = Some(label);
    Ok(())
}

#[tauri::command]
fn sync_webview_layout(
    app: AppHandle,
    state: tauri::State<WebviewState>,
    sidebar_width: f64,
) -> Result<(), String> {
    *state.sidebar_width.lock().map_err(|error| error.to_string())? = sidebar_width;
    sync_all_bounds(&app, sidebar_width)
}

#[tauri::command]
fn hide_all_webviews(app: AppHandle, state: tauri::State<WebviewState>) -> Result<(), String> {
    if let Some(window) = app.get_window(MAIN_WINDOW) {
        for webview in window.webviews() {
            if is_target_webview(webview.label()) {
                let _ = webview.hide();
            }
        }
    }

    *state.active_label.lock().map_err(|error| error.to_string())? = None;
    Ok(())
}

#[tauri::command]
fn remove_target(app: AppHandle, state: tauri::State<WebviewState>, id: String) -> Result<(), String> {
    let label = target_label(&id);
    if let Some(webview) = app.get_webview(&label) {
        webview.close().map_err(|error| error.to_string())?;
    }

    let mut active_label = state.active_label.lock().map_err(|error| error.to_string())?;
    if active_label.as_deref() == Some(label.as_str()) {
        *active_label = None;
    }

    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .manage(WebviewState::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            select_target,
            sync_webview_layout,
            hide_all_webviews,
            remove_target
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();
            if let Some(window) = app.get_window(MAIN_WINDOW) {
                window.on_window_event(move |event| match event {
                    WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                        if let Some(state) = app_handle.try_state::<WebviewState>() {
                            if let Ok(sidebar_width) = state.sidebar_width.lock() {
                                let _ = sync_all_bounds(&app_handle, *sidebar_width);
                            }
                        }
                    }
                    _ => {}
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}

fn ensure_webview(app: &AppHandle, label: &str, url: &str, sidebar_width: f64) -> Result<(), String> {
    if app.get_webview(label).is_some() {
        sync_one_bounds(app, label, sidebar_width)?;
        return Ok(());
    }

    let window = app
        .get_window(MAIN_WINDOW)
        .ok_or_else(|| "main window not found".to_string())?;
    let parsed_url = url.parse().map_err(|error| format!("invalid url: {error}"))?;
    let webview = window
        .add_child(
            WebviewBuilder::new(label, WebviewUrl::External(parsed_url)),
            LogicalPosition::new(sidebar_width, 0.0),
            content_size(&window, sidebar_width)?,
        )
        .map_err(|error| error.to_string())?;

    webview.hide().map_err(|error| error.to_string())?;
    Ok(())
}

fn sync_all_bounds(app: &AppHandle, sidebar_width: f64) -> Result<(), String> {
    if let Some(window) = app.get_window(MAIN_WINDOW) {
        let position = LogicalPosition::new(sidebar_width, 0.0);
        let size = content_size(&window, sidebar_width)?;

        for webview in window.webviews() {
            if is_target_webview(webview.label()) {
                webview.set_position(position).map_err(|error| error.to_string())?;
                webview.set_size(size).map_err(|error| error.to_string())?;
            }
        }
    }

    Ok(())
}

fn sync_one_bounds(app: &AppHandle, label: &str, sidebar_width: f64) -> Result<(), String> {
    let window = app
        .get_window(MAIN_WINDOW)
        .ok_or_else(|| "main window not found".to_string())?;
    let webview = app
        .get_webview(label)
        .ok_or_else(|| format!("webview {label} not found"))?;

    webview
        .set_position(LogicalPosition::new(sidebar_width, 0.0))
        .map_err(|error| error.to_string())?;
    webview
        .set_size(content_size(&window, sidebar_width)?)
        .map_err(|error| error.to_string())?;

    Ok(())
}

fn content_size(window: &tauri::Window, sidebar_width: f64) -> Result<LogicalSize<f64>, String> {
    let scale_factor = window.scale_factor().map_err(|error| error.to_string())?;
    let size = window
        .inner_size()
        .map_err(|error| error.to_string())?
        .to_logical::<f64>(scale_factor);

    Ok(LogicalSize::new((size.width - sidebar_width).max(1.0), size.height.max(1.0)))
}

fn target_label(id: &str) -> String {
    format!("{WEBVIEW_PREFIX}{id}")
}

fn is_target_webview(label: &str) -> bool {
    label.starts_with(WEBVIEW_PREFIX)
}
