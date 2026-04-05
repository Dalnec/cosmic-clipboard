use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{time, Application, Command, Element, Length, Subscription, Theme};
use std::time::{Duration, Instant};
use crate::db::Database;
use crate::db::models::ClipboardEntry;

pub struct CopyousApp {
    entries: Vec<ClipboardEntry>,
    db_path: String,
    search_query: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    #[allow(dead_code)]
    Loaded(Vec<ClipboardEntry>),
    CopyToClipboard(String),
    DeleteEntry(i64),
    SearchChanged(String),
    #[allow(dead_code)]
    Tick(Instant),
}

impl Application for CopyousApp {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let db_path = format!("{}/.local/share/cosmic/com.boerdereinar.copyous/clipboard.db", home);
        
        let mut loaded_entries = Vec::new();
        // Cargar primera vez al iniciar la ventana
        if let Ok(db) = Database::new(&db_path) {
            if let Ok(entries) = db.get_entries() {
                loaded_entries = entries;
            }
        }

        (
            Self {
                entries: loaded_entries,
                db_path,
                search_query: String::new(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Copyous COSMIC - Portapapeles")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Loaded(loaded_entries) => {
                self.entries = loaded_entries;
            }
            Message::SearchChanged(query) => {
                self.search_query = query;
            }
            Message::CopyToClipboard(content) => {
                // Escribir el contenido explícitamente en el portapapeles activo usando la librería de sistema
                let _ = std::process::Command::new("wl-copy").arg(&content).output();
                println!("Acción Ejecutada: Bloque rebotado directamente al portapapeles local.");
            }
            Message::DeleteEntry(id) => {
                // Llamamos a nuestra función previamente construida para botarlo de SQLite
                if let Ok(db) = Database::new(&self.db_path) {
                    let _ = db.delete_entry(id);
                    // Refrescamos inmediatamente después de borrar
                    if let Ok(entries) = db.get_entries() {
                        self.entries = entries;
                    }
                }
            }
            Message::Tick(_) => {
                // Consultar en segundo plano para ver si el Daemon asincrónico copió algo nuevo
                if let Ok(db) = Database::new(&self.db_path) {
                    if let Ok(entries) = db.get_entries() {
                        self.entries = entries;
                    }
                }
            }
        }
        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        // Enlazar una suscripción de tiempo cada 1 segundo (Tick) para actualizar en vivo la vista
        time::every(Duration::from_millis(1000)).map(Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let search_box = text_input("🔍 Busca entre tus recortes...", &self.search_query)
            .on_input(Message::SearchChanged)
            .padding(10)
            .size(20);

        let mut list = column![
            text("📋 Historial del Portapapeles").size(30),
            search_box,
        ]
        .spacing(15);

        // Pre-filtramos las entradas según la búsqueda (ignorando las mayúsculas)
        let filtered_entries: Vec<&ClipboardEntry> = self.entries
            .iter()
            .filter(|e| e.content.to_lowercase().contains(&self.search_query.to_lowercase()))
            .collect();

        if filtered_entries.is_empty() {
             list = list.push(text("No hay elementos que coincidan con la búsqueda."));
        } else {
             for entry in filtered_entries {
                 let content_preview = text(entry.content.clone()).size(16);
                 
                 // Crear botones de acción atados a variables
                 let btn_copy = button("Copiar (Ctrl+C)")
                     .on_press(Message::CopyToClipboard(entry.content.clone()))
                     .padding(7);

                 let mut actions_row = row![btn_copy].spacing(10);
                 
                 // Extraemos el ID e inyectamos el botón de borrar si es un registro validado
                 if let Some(id) = entry.id {
                     let btn_delete = button("Borrar")
                         .on_press(Message::DeleteEntry(id))
                         .padding(7);
                     actions_row = actions_row.push(btn_delete);
                 }

                 let item_block = column![
                     content_preview,
                     actions_row
                 ].spacing(10);

                 let item = container(item_block)
                     .padding(15)
                     .width(Length::Fill);

                 list = list.push(item);
             }
        }

        container(scrollable(list))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .into()
    }
}
