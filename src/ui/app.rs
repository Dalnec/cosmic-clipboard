use cosmic::iced::{window::Id, Limits, Subscription};
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::prelude::*;
use cosmic::widget;
use std::time::Duration;

use crate::db::Database;
use crate::db::models::ClipboardEntry;

pub struct CopyousApp {
    core: cosmic::Core,
    popup: Option<Id>,
    entries: Vec<ClipboardEntry>,
    db_path: String,
    search_query: String,
    editing_target: Option<i64>,
    editing_title: String,
    editing_tag: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    CopyToClipboard(String),
    DeleteEntry(i64),
    TogglePin(i64),
    StartEditing(i64, Option<String>, Option<String>),
    InputTitleChanged(String),
    InputTagChanged(String),
    SaveEdits(i64),
    CancelEdits,
    SearchChanged(String),
    Tick,
    Quit,
}

impl cosmic::Application for CopyousApp {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "com.boerdereinar.copyous";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(core: cosmic::Core, _flags: Self::Flags) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let db_path = format!("{}/.local/share/cosmic/com.boerdereinar.copyous/clipboard.db", home);
        
        let mut loaded_entries = Vec::new();
        if let Ok(db) = Database::new(&db_path) {
            if let Ok(entries) = db.get_entries() {
                loaded_entries = entries;
            }
        }

        (
            Self {
                core,
                popup: None,
                entries: loaded_entries,
                db_path,
                search_query: String::new(),
                editing_target: None,
                editing_title: String::new(),
                editing_tag: String::new(),
            },
            Task::none(),
        )
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn view(&self) -> Element<'_, Self::Message> {
        self.core
            .applet
            .icon_button("edit-copy-symbolic")
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        let search_box = widget::text_input("🔍 Busca entre tus recortes...", &self.search_query)
            .on_input(Message::SearchChanged)
            .padding(10);

        let mut list_items: Vec<Element<'_, Message>> = Vec::new();
        list_items.push(widget::text("📋 Historial del Portapapeles").size(24).into());
        list_items.push(search_box.into());

        let filtered_entries: Vec<&ClipboardEntry> = self.entries
            .iter()
            .filter(|e| {
                let search_lower = self.search_query.to_lowercase();
                let txt_match = e.content.to_lowercase().contains(&search_lower);
                let tag_match = e.tag.as_ref().map_or(false, |t| t.to_lowercase().contains(&search_lower));
                let title_match = e.title.as_ref().map_or(false, |t| t.to_lowercase().contains(&search_lower));
                txt_match || tag_match || title_match
            })
            .collect();

        if filtered_entries.is_empty() {
             list_items.push(widget::text("No hay elementos que coincidan con la búsqueda.").into());
        } else {
             for entry in filtered_entries {
                 if Some(entry.id.unwrap_or(-1)) == self.editing_target {
                     // MODO EDICIÓN
                     let title_input = widget::text_input("Ingresa un Título...", &self.editing_title)
                         .on_input(Message::InputTitleChanged).padding(7);
                     let tag_input = widget::text_input("Ingresa un Tag...", &self.editing_tag)
                         .on_input(Message::InputTagChanged).padding(7);
                         
                     let btn_save = widget::button::standard("✅")
                         .on_press(Message::SaveEdits(entry.id.unwrap())).padding(7);
                     let btn_cancel = widget::button::destructive("❌")
                         .on_press(Message::CancelEdits).padding(7);

                     let edit_block = widget::column(vec![
                          widget::text("Editando Título:").size(14).into(),
                          title_input.into(),
                          widget::text("Editando Tag:").size(14).into(),
                          tag_input.into(),
                          widget::row(vec![btn_save.into(), btn_cancel.into()]).spacing(10).into(),
                     ]).spacing(8);
                     
                     let item = widget::container(edit_block).padding(15).width(cosmic::iced::Length::Fill);
                     list_items.push(item.into());
                 } else {
                     // MODO VISUALIZACIÓN NORMAL
                     let mut content_elements: Vec<Element<'_, Message>> = Vec::new();
                     
                     // Metadata Line (Pinned, Tag, Type, Metadata)
                     let mut meta_infos: Vec<Element<'_, Message>> = Vec::new();
                     if entry.pinned {
                         meta_infos.push(widget::text("📌 Pinned").size(12).into());
                     }
                     if let Some(ref tag) = entry.tag {
                         let tag_badge = widget::text(format!("🏷 {}", tag)).size(12);
                         meta_infos.push(tag_badge.into());
                     }
                     meta_infos.push(widget::text(format!("(Tipo: {:?})", entry.item_type)).size(10).into());
                     
                     if let Some(ref meta) = entry.metadata {
                         meta_infos.push(widget::text(format!("Meta: {}", meta)).size(10).into());
                     }

                     if !meta_infos.is_empty() {
                         content_elements.push(widget::row(meta_infos).spacing(10).into());
                     }

                     // Título visible si existe
                     if let Some(ref title) = entry.title {
                         content_elements.push(widget::text(title.clone()).size(18).into());
                     }

                     // Texto parcial original copiado
                     let content_preview = widget::text(entry.content.clone()).size(15);
                     content_elements.push(content_preview.into());

                     // Botones de acción
                     let btn_copy = widget::button::standard("📋")
                         .on_press(Message::CopyToClipboard(entry.content.clone()))
                         .padding(7);

                     let mut actions: Vec<Element<'_, Message>> = vec![btn_copy.into()];
                     
                     if let Some(id) = entry.id {
                         let pin_icon = if entry.pinned { "📌" } else { "📍" };
                         let btn_pin = widget::button::standard(pin_icon)
                             .on_press(Message::TogglePin(id))
                             .padding(7);
                         actions.push(btn_pin.into());

                         let btn_edit = widget::button::standard("✏️")
                             .on_press(Message::StartEditing(id, entry.title.clone(), entry.tag.clone()))
                             .padding(7);
                         actions.push(btn_edit.into());

                         let btn_delete = widget::button::destructive("🗑️")
                             .on_press(Message::DeleteEntry(id))
                             .padding(7);
                         actions.push(btn_delete.into());
                     }

                     content_elements.push(widget::row(actions).spacing(10).into());

                     let item_block = widget::column(content_elements).spacing(10);
                     let item = widget::container(item_block)
                         .padding(15)
                         .width(cosmic::iced::Length::Fill);

                     list_items.push(item.into());
                 }
             }
        }

        let scroll_content = widget::scrollable(widget::column(list_items).spacing(15).padding(10))
            .width(cosmic::iced::Length::Fill)
            .height(cosmic::iced::Length::Fill);

        let btn_quit = widget::button::destructive("⏻")
            .on_press(Message::Quit)
            .width(cosmic::iced::Length::Fill);

        let popup_layout = widget::column(vec![
            scroll_content.into(),
            btn_quit.into()
        ]).spacing(10);

        self.core.applet.popup_container(popup_layout).into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        cosmic::iced::time::every(Duration::from_millis(1000)).map(|_| Message::Tick)
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::SearchChanged(query) => {
                self.search_query = query;
            }
            Message::InputTitleChanged(t) => {
                self.editing_title = t;
            }
            Message::InputTagChanged(t) => {
                self.editing_tag = t;
            }
            Message::StartEditing(id, title, tag) => {
                self.editing_target = Some(id);
                self.editing_title = title.unwrap_or_default();
                self.editing_tag = tag.unwrap_or_default();
            }
            Message::CancelEdits => {
                self.editing_target = None;
            }
            Message::SaveEdits(id) => {
                if let Ok(db) = Database::new(&self.db_path) {
                    if let Some(entry) = self.entries.iter_mut().find(|e| e.id == Some(id)) {
                        entry.title = if self.editing_title.trim().is_empty() { None } else { Some(self.editing_title.clone()) };
                        entry.tag = if self.editing_tag.trim().is_empty() { None } else { Some(self.editing_tag.clone()) };
                        
                        let _ = db.update_entry_metadata(id, entry.pinned, entry.tag.clone(), entry.title.clone());
                    }
                    if let Ok(entries) = db.get_entries() {
                        self.entries = entries;
                    }
                }
                self.editing_target = None;
            }
            Message::TogglePin(id) => {
                if let Ok(db) = Database::new(&self.db_path) {
                    if let Some(entry) = self.entries.iter_mut().find(|e| e.id == Some(id)) {
                        entry.pinned = !entry.pinned;
                        let _ = db.update_entry_metadata(id, entry.pinned, entry.tag.clone(), entry.title.clone());
                    }
                    if let Ok(entries) = db.get_entries() {
                        self.entries = entries;
                    }
                }
            }
            Message::CopyToClipboard(content) => {
                let _ = std::process::Command::new("wl-copy").arg(&content).spawn();
                println!("Acción Ejecutada: Bloque rebotado directamente al portapapeles local.");
                
                if let Some(p) = self.popup.take() {
                    return destroy_popup(p);
                }
            }
            Message::DeleteEntry(id) => {
                if let Ok(db) = Database::new(&self.db_path) {
                    let _ = db.delete_entry(id);
                    if let Ok(entries) = db.get_entries() {
                        self.entries = entries;
                    }
                }
            }
            Message::Tick => {
                if let Ok(db) = Database::new(&self.db_path) {
                    if let Ok(entries) = db.get_entries() {
                        self.entries = entries;
                    }
                }
            }
            Message::TogglePopup => {
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );
                    popup_settings.positioner.size_limits = Limits::NONE
                        .max_width(450.0)
                        .min_width(350.0)
                        .min_height(300.0)
                        .max_height(800.0);
                    get_popup(popup_settings)
                }
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
            Message::Quit => {
                println!("Cerrando voluntariamente el Applet y Daemon...");
                std::process::exit(0);
            }
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}
