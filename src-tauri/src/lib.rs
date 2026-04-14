use crate::kirc::manager::KircManager;
use crate::kirc::persistence::KircStateSnapshot;
use crate::memento::Memento;
use anyhow::Context;
use std::sync::Arc;
use tauri::menu::{Menu, MenuBuilder, MenuEvent, MenuItem, SubmenuBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Window, WindowEvent};
use tracing::warn;

mod error;
mod fs;
mod kirc;
mod logging;
mod memento;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logging::init_logging();

    tauri::Builder::default()
        .setup(|app| {
            // Menu
            let file_menu = SubmenuBuilder::new(app, "File")
                // .submenu_icon(menu_image) // Optional: Add an icon to the submenu
                .text("open", "Open")
                .text("quit", "Quit")
                .build()?;

            let app_handle = app.handle();
            let menu = MenuBuilder::new(app_handle).items(&[&file_menu]).build()?;

            app.set_menu(menu).context("Can not initialize menu")?;

            // System tray
            let tray_quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let tray_show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&tray_show, &tray_quit])?;
            let _tray = TrayIconBuilder::new()
                .menu(&tray_menu)
                .on_menu_event(on_menu_event)
                .show_menu_on_left_click(false)
                .icon(app.default_window_icon().unwrap().clone())
                .build(app)?;

            // kirc
            let mut app_data_dir = app.path().app_data_dir().unwrap();
            if cfg!(debug_assertions) {
                app_data_dir.push("dev");
            }

            if !app_data_dir.exists() {
                std::fs::create_dir_all(&app_data_dir)?;
            }
            let config_path = app_data_dir.join("config.json");
            let snapshot: KircStateSnapshot = fs::load(&config_path).unwrap();

            let mut state = snapshot.restore();
            state.set_persistence_path(&config_path);
            let state = Arc::new(state);

            app.manage(KircManager::new(state.clone(), app.handle().clone()));
            app.manage(state);

            Ok(())
        })
        .on_window_event(on_window_event)
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            kirc::commands::init_servers,
            kirc::commands::get_servers,
            kirc::commands::connect_server,
            kirc::commands::join_channel,
            kirc::commands::leave_channel,
            kirc::commands::send_message,
            kirc::commands::cancel_connect,
            kirc::commands::disconnect_server,
            kirc::commands::lock_channel,
            kirc::commands::unlock_channel,
            kirc::commands::is_channel_locked,
            kirc::commands::change_nickname
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn on_window_event(window: &Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close(); // 실제 종료 막기
        let _ = window.hide(); // 창만 숨김
    }
}

fn on_menu_event(app_handle: &AppHandle, event: MenuEvent) {
    match event.id.as_ref() {
        "show" => {
            if let Some(window) = app_handle.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }
        "quit" => {
            let async_app_handle = app_handle.clone();

            tauri::async_runtime::spawn(async move {
                if let Some(manager) = async_app_handle.try_state::<KircManager>() {
                    manager.shutdown().await;
                }

                async_app_handle.exit(0);
            });
        }
        _ => {
            warn!("Unhandled event: {:?}", event.id);
        }
    }
}
