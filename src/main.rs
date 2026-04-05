mod db;
mod clipboard;
mod ui; // Importamos el módulo visual

fn main() -> cosmic::iced::Result {
    println!("Copyous COSMIC - Background Daemon Initializing...");
    
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let db_path = format!("{}/.local/share/cosmic/com.boerdereinar.copyous/clipboard.db", home);
    
    println!("Conectando a base de datos en: {}", db_path);

    let database = db::Database::new(&db_path).unwrap_or_else(|e| {
        panic!("Error initializing SQLite database at {}: {:?}", db_path, e);
    });
    
    let entries = database.get_entries().expect("Error al leer el historial");
    println!("Database ready! Found {} clipboard entries.", entries.len());
    
    // Lanzamos el monitor de portapapeles en un hilo estándar de C/Rust
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            clipboard::monitor::start_monitoring(db_path.clone()).await;
            // Para mantener el hilo vivo infinitamente
            loop { tokio::time::sleep(std::time::Duration::from_secs(60)).await; }
        });
    });

    println!("Daemon iniciado internamente. Lanzando Applet de COSMIC Panel...");
    
    // Bloquear el hilo inicial dibujando la ventana gráfica tipo Applet
    cosmic::applet::run::<ui::app::CopyousApp>(())
}
