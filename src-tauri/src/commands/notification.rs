#[cfg(windows)]
pub mod windows_toast {
    use tauri::{AppHandle, Emitter, Manager, Runtime};
    use windows::{
        Data::Xml::Dom::XmlDocument,
        UI::Notifications::{ToastNotification, ToastNotificationManager},
        core::{HSTRING, IInspectable},
        Foundation::TypedEventHandler,
    };

    pub fn show_clickable_notification<R: Runtime>(
        app: &AppHandle<R>,
        title: &str,
        body: &str,
    ) -> Result<(), String> {
        let app_id = app.config().identifier.clone();

        let xml_str = format!(
            r#"<toast activationType="foreground">
                <visual>
                    <binding template="ToastGeneric">
                        <text>{}</text>
                        <text>{}</text>
                    </binding>
                </visual>
            </toast>"#,
            escape_xml(title),
            escape_xml(body),
        );

        let xml = XmlDocument::new().map_err(|e| format!("XmlDocument::new: {e}"))?;
        xml.LoadXml(&HSTRING::from(xml_str))
            .map_err(|e| format!("LoadXml: {e}"))?;

        let toast = ToastNotification::CreateToastNotification(&xml)
            .map_err(|e| format!("CreateToastNotification: {e}"))?;

        let app_handle = app.clone();
        let handler: TypedEventHandler<ToastNotification, IInspectable> =
            TypedEventHandler::new(move |_, _| {
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
                let _ = app_handle.emit("notification-clicked", ());
                Ok(())
            });

        toast
            .Activated(&handler)
            .map_err(|e| format!("Activated: {e}"))?;

        let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(&app_id))
            .or_else(|_| {
                // Fallback for dev/uninstalled: use PowerShell's registered AUMID.
                ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(
                    "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\WindowsPowerShell\\v1.0\\powershell.exe",
                ))
            })
            .map_err(|e| format!("CreateToastNotifierWithId: {e}"))?;

        notifier.Show(&toast).map_err(|e| format!("Show: {e}"))?;

        Ok(())
    }

    fn escape_xml(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }
}

#[tauri::command]
pub async fn send_clickable_notification(
    app: tauri::AppHandle,
    title: String,
    body: String,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        windows_toast::show_clickable_notification(&app, &title, &body)
    }
    #[cfg(not(windows))]
    {
        use tauri_plugin_notification::NotificationExt;
        app.notification()
            .builder()
            .title(title)
            .body(body)
            .show()
            .map_err(|e| e.to_string())
    }
}
