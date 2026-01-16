use parking_lot::{Condvar, Mutex};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

#[derive(Clone, Default)]
pub struct FrontendReady {
    inner: Arc<(Mutex<bool>, Condvar)>,
}

impl FrontendReady {
    pub fn wait_ready_timeout(&self, timeout: Duration) -> bool {
        let (lock, cv) = &*self.inner;
        let mut ready = lock.lock();
        if *ready {
            return true;
        }
        cv.wait_for(&mut ready, timeout);
        *ready
    }

    pub fn set_ready(&self) {
        let (lock, cv) = &*self.inner;
        let mut ready = lock.lock();
        *ready = true;
        cv.notify_all();
    }
}

fn splash_html() -> &'static str {
    r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Loading…</title>
    <style>
      html, body {
        margin: 0;
        padding: 0;
        width: 100%;
        height: 100%;
        background: #020617;
      }
      .wrap {
        position: fixed;
        inset: 0;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        background: radial-gradient(circle at top, #0f172a 0%, #020617 100%);
        color: #e5e7eb;
        font-family: system-ui, -apple-system, Segoe UI, sans-serif;
      }
      .logo {
        font-size: 64px;
        margin-bottom: 24px;
        animation: pulse 1.8s infinite;
        filter: drop-shadow(0 0 20px rgba(77, 163, 255, 0.35));
      }
      .bar-bg {
        width: 180px;
        height: 4px;
        background: rgba(148, 163, 184, 0.2);
        border-radius: 2px;
        overflow: hidden;
      }
      .bar {
        width: 50%;
        height: 100%;
        background: linear-gradient(90deg, #4da3ff, #22d3ee);
        animation: slide 1.4s infinite ease-in-out;
      }
      .hint {
        margin-top: 14px;
        font-size: 12px;
        color: rgba(148, 163, 184, 0.9);
      }
      @keyframes pulse {
        0%, 100% { opacity: 1; transform: scale(1); }
        50% { opacity: 0.78; transform: scale(0.95); }
      }
      @keyframes slide {
        0% { transform: translateX(-100%); }
        100% { transform: translateX(200%); }
      }
    </style>
  </head>
  <body>
    <div class="wrap">
      <div class="logo">🛡️</div>
      <div class="bar-bg"><div class="bar"></div></div>
      <div class="hint">Starting…</div>
    </div>
  </body>
</html>"#
}

pub fn show_splashscreen(app: &AppHandle) -> Result<(), String> {
    if app.get_webview_window("splashscreen").is_some() {
        return Ok(());
    }

    let encoded = urlencoding::encode(splash_html());
    let data_url = format!("data:text/html,{}", encoded);
    let url = url::Url::parse(&data_url).map_err(|e| e.to_string())?;

    WebviewWindowBuilder::new(app, "splashscreen", WebviewUrl::External(url))
        .title("Starting…")
        .decorations(false)
        .resizable(false)
        .always_on_top(true)
        .transparent(false)
        .center()
        .inner_size(420.0, 300.0)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn close_splashscreen_internal(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("splashscreen") {
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn app_ready(app: AppHandle, ready: tauri::State<'_, FrontendReady>) -> Result<(), String> {
    ready.set_ready();
    close_splashscreen_internal(&app)?;

    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.set_focus();
    }

    Ok(())
}
