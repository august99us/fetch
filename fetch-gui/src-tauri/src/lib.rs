use std::error::Error;

use camino::Utf8PathBuf;
use fetch_core::{init_indexing, init_ort, init_querying};
use tauri::{menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem}, tray::{MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent}, App, AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Get the resource directory where models are bundled
            let resource_dir = Utf8PathBuf::try_from(app.path().resource_dir()
                .expect("Failed to get resource directory"))
                .expect("Resource directory path is not valid UTF-8");

            // Initialize ort first
            init_ort(Some(&resource_dir)).expect("Failed initializing ort");

            // Convert to Utf8PathBuf and set as the base model directory
            let models_dir = resource_dir.join("models");

            // Set the resource directory with the first init call
            init_indexing(Some(&models_dir));
            // Second call doesn't need to set it again since fetch-core defines this as static setup
            init_querying(None);

            // Initialize system tray functionality
            let tray = build_tray(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            crate::commands::index::index,
            crate::commands::open::open,
            crate::commands::open_location::open_location,
            crate::commands::preview::preview,
            crate::commands::query::query,
        ])
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                if window.label() == "full" {
                    // Hide the window instead of closing
                    window.hide().expect("Could not hide full search window");
                    // Prevent the application from closing
                    api.prevent_close();
                }
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn build_tray(app: &mut App) -> Result<TrayIcon, Box<dyn Error>> {
    let menu_items: Vec<Box<dyn IsMenuItem<_>>> = vec![
        Box::new(MenuItem::with_id(app, "fetch", "Fetch", true, Some("CmdOrCtrl+Shift+Space"))?),
        Box::new(MenuItem::with_id(app, "search", "Search and Index", true, None::<&str>)?),
        Box::new(PredefinedMenuItem::separator(app)?),
        Box::new(MenuItem::with_id(app, "settings", "Settings", false, None::<&str>)?),
        Box::new(PredefinedMenuItem::separator(app)?),
        Box::new(MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?),
    ];
    let menu = Menu::with_items(app, 
        menu_items.iter()
            .map(Box::as_ref)
            .collect::<Vec<_>>()
            .as_slice())?;
    Ok(TrayIconBuilder::new()
        .icon(app.default_window_icon().expect("App should have an icon").clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } => {
                println!("Tray icon double clicked");
                // in this example, let's show and focus the main window when the tray is double clicked
                let app = tray.app_handle();
                summon_full_window(app).expect("Unable to instantiate full search window");
            }
            _ => {},
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "fetch" => {
                let _webview_window = WebviewWindowBuilder::new(
                    app, 
                    "quick", 
                    WebviewUrl::App("/".into())
                )
                .inner_size(800.0, 69.0)
                .transparent(true)
                .decorations(false)
                .always_on_top(true)
                .resizable(false)
                .center()
                .focused(true)
                .build()
                .expect("Unable to instantiate fetch window");
            },
            "search" => {
                summon_full_window(app).expect("Unable to instantiate full search window");
            },
            "settings" => {
                println!("settings menu item was clicked. Not yet implemented!");
            },
            "quit" => {
                if let Some(main_window) = app.get_webview_window("full") {
                    main_window.destroy().unwrap_or_else(|e| 
                        eprintln!("Error while trying to destroy full search window before closing: {:?}", e));
                }
                app.exit(0);
            },
            _ => {}
        })
        .build(app)?)
}

fn summon_full_window(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    if let Some(window) = app.get_webview_window("full") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    } else {
        let _webview_window = WebviewWindowBuilder::new(
            app, 
            "full", 
            WebviewUrl::App("/search".into())
        )
        .resizable(true)
        .center()
        .focused(true)
        .build()?;
    }

    Ok(())
}

pub mod utility;
pub mod commands;