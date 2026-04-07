use crate::db::{Database, models::{ClipboardEntry, ItemType}};
use chrono::Local;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};

pub async fn start_monitoring(db_path: String) {
    println!("Iniciando hilo para escuchar cambios en Wayland nativo vía wl-clipboard...");
    
    tokio::task::spawn_blocking(move || {
        let database = Database::new(&db_path).expect("Daemon worker couldn't open db");
        let mut last_clipboard_text = String::new();

        println!("[DEBUG MONITOR] Levantando demonio interno: wl-paste --watch");
        
        let child_proc = Command::new("wl-paste")
            .args(["--watch", "echo", "COPY_TRIGGER"])
            .stdout(Stdio::piped())
            .spawn();

        let mut child = match child_proc {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[ERROR CRÍTICO] Hubo un error arrancando wl-paste nativo: {:?}", e);
                return;
            }
        };

        let stdout = child.stdout.take().expect("No se capturó salida estándar de Watch");
        let reader = BufReader::new(stdout);

        println!("[DEBUG MONITOR] Suspendido exitosamente. Esperando cortes sin consumir CPU...");

        // Aquí el proceso se congela eternamente en espera. Solo despertará si el OS nos arroja texto.
        for _ in reader.lines() {
            // "wl-paste" nos avisa que el portapapeles tuvo un cambio
            // 1. Detectamos el tipo de contenido
            let list_types = Command::new("wl-paste").arg("--list-types").output();
            let mut is_image = false;
            let mut is_text = false;

            if let Ok(output) = list_types {
                let types = String::from_utf8_lossy(&output.stdout);
                is_image = types.contains("image/");
                is_text = types.contains("text/plain") || types.contains("UTF8_STRING") || types.contains("text/html");
            }

            if is_text {
                if let Ok(output) = Command::new("wl-paste").arg("--no-newline").arg("--type").arg("text/plain").output() {
                    let text = String::from_utf8_lossy(&output.stdout).to_string();
                    
                    if text != last_clipboard_text && !text.trim().is_empty() {
                        println!("  > ¡BINGO! Nuevo bloque nativo (Texto): {}", text.chars().take(40).collect::<String>());
                        
                        let now = Local::now();
                        let datetime_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

                        let item_type = if text.starts_with("http://") || text.starts_with("https://") {
                            ItemType::Link
                        } else {
                            ItemType::Text
                        };

                        let entry = ClipboardEntry {
                            id: None,
                            item_type,
                            content: text.clone(),
                            pinned: false,
                            tag: None,
                            datetime: datetime_str,
                            metadata: None,
                            title: None,
                        };

                        match database.insert_entry(&entry) {
                            Ok(id) => println!("[DEBUG MONITOR] Insertado en DB (RowID: {})", id),
                            Err(e) => eprintln!("[ERROR DB] Fallo insertando SQLite: {:?}", e),
                        }

                        last_clipboard_text = text;
                    }
                }
            } else if is_image {
                // Por ahora no guardamos el blob de la imagen en el campo de texto, solo registramos que hubo una
                println!("  > Detectada imagen en el portapapeles. Registrando como metadato...");
                
                let now = Local::now();
                let datetime_str = now.format("%Y-%m-%d %H:%M:%S").to_string();

                let entry = ClipboardEntry {
                    id: None,
                    item_type: ItemType::Image,
                    content: "[Imagen capturada]".to_string(),
                    pinned: false,
                    tag: None,
                    datetime: datetime_str,
                    metadata: Some("Mime: image/*".to_string()),
                    title: Some("Nueva Imagen".to_string()),
                };

                match database.insert_entry(&entry) {
                    Ok(id) => println!("[DEBUG MONITOR] Imagen insertada en DB (RowID: {})", id),
                    Err(e) => eprintln!("[ERROR DB] Fallo insertando Imagen: {:?}", e),
                }
                
                // Actualizamos last_clipboard_text para no repetir si re-capturamos lo mismo
                last_clipboard_text = "[Imagen capturada]".to_string();
            }
        }
    });
}
