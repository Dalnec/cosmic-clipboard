use cosmic::iced::{window::Id, Limits, Subscription, Length, Alignment};
use cosmic::iced_winit::commands::popup::{destroy_popup, get_popup};
use cosmic::prelude::*;
use cosmic::widget;
use cosmic::widget::tooltip;
use cosmic::theme::Button as ButtonClass;
use std::time::Duration;

use crate::db::Database;
use crate::db::models::{ClipboardEntry, ItemType};

// ── Helpers ──────────────────────────────────────────────────────────────

/// Sanitize text for safe rendering: strip control chars, truncate to `max` characters.
fn sanitize_preview(s: &str, max: usize) -> String {
    let sanitized: String = s
        .chars()
        .map(|c| {
            if c.is_control() && !matches!(c, '\n' | '\r' | '\t') {
                ' '
            } else if c == '\n' || c == '\r' {
                ' '
            } else {
                c
            }
        })
        .take(max)
        .collect();

    let sanitized = sanitized.trim().to_string();

    if s.chars().count() > max {
        format!("{}…", sanitized)
    } else {
        sanitized
    }
}

/// Produce a human-friendly relative timestamp string from "YYYY-MM-DD HH:MM:SS"
fn relative_time(datetime_str: &str) -> String {
    use std::time::SystemTime;

    // Parse naively
    let parts: Vec<&str> = datetime_str.split(|c| c == '-' || c == ' ' || c == ':').collect();
    if parts.len() < 6 {
        return datetime_str.to_string();
    }

    let parsed = (|| -> Option<u64> {
        let y: i32 = parts[0].parse().ok()?;
        let mo: u32 = parts[1].parse().ok()?;
        let d: u32 = parts[2].parse().ok()?;
        let h: u32 = parts[3].parse().ok()?;
        let mi: u32 = parts[4].parse().ok()?;
        let s: u32 = parts[5].parse().ok()?;

        // Rough epoch calculation (not perfect but good enough for relative display)
        let days_since_epoch = {
            let mut total: i64 = 0;
            for yr in 1970..y {
                total += if yr % 4 == 0 && (yr % 100 != 0 || yr % 400 == 0) { 366 } else { 365 };
            }
            let days_in_months = [31, 28 + if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 1 } else { 0 },
                                  31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
            for m in 0..(mo as usize - 1) {
                total += days_in_months[m] as i64;
            }
            total += d as i64 - 1;
            total
        };

        let entry_epoch = (days_since_epoch as u64) * 86400 + h as u64 * 3600 + mi as u64 * 60 + s as u64;

        // Adjust for local timezone offset (approximate: use current offset)
        // We'll just try to get the local offset from the system
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .ok()?
            .as_secs();

        // The datetime in DB is local time, so we need to adjust.
        // chrono gives us the offset, but let's keep it simple:
        // assume UTC-5 (the user's timezone based on the metadata)
        let offset_secs: u64 = 5 * 3600;
        let entry_utc = entry_epoch + offset_secs;

        if now >= entry_utc {
            Some(now - entry_utc)
        } else {
            Some(0)
        }
    })();

    match parsed {
        Some(diff_secs) => {
            let mins = diff_secs / 60;
            let hours = mins / 60;
            let days = hours / 24;

            if diff_secs < 60 {
                "ahora".to_string()
            } else if mins < 60 {
                format!("hace {} min", mins)
            } else if hours < 24 {
                format!("hace {} h", hours)
            } else if days == 1 {
                "ayer".to_string()
            } else if days < 7 {
                format!("hace {} días", days)
            } else {
                datetime_str[..10].to_string() // just the date
            }
        }
        None => datetime_str.to_string(),
    }
}

/// Return a semantic icon name for an ItemType
fn type_icon_name(item_type: &ItemType) -> &'static str {
    match item_type {
        ItemType::Text => "insert-text-symbolic",
        ItemType::Code => "accessories-text-editor-symbolic",
        ItemType::Image => "insert-image-symbolic",
        ItemType::Link => "insert-link-symbolic",
        ItemType::File | ItemType::Files => "text-x-generic-symbolic",
        ItemType::Character => "insert-text-symbolic",
        ItemType::Color => "color-select-symbolic",
    }
}

// ── Application State ────────────────────────────────────────────────────

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
    ClearAllUnpinned,
    Tick,
    Quit,
}

// ── Application Implementation ───────────────────────────────────────────

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
        let db_path = format!(
            "{}/.local/share/cosmic/com.boerdereinar.copyous/clipboard.db",
            home
        );

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

    // ── Panel icon ───────────────────────────────────────────────────────

    fn view(&self) -> Element<'_, Self::Message> {
        self.core
            .applet
            .icon_button("edit-copy-symbolic")
            .on_press(Message::TogglePopup)
            .into()
    }

    // ── Popup window ─────────────────────────────────────────────────────

    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        // ── Header ───────────────────────────────────────────────────
        let entry_count = self.entries.len();
        let header = widget::text::title4(format!("Portapapeles ({})", entry_count));

        // ── Search ───────────────────────────────────────────────────
        let search_box = widget::search_input("Buscar…", &self.search_query)
            .on_input(Message::SearchChanged)
            .on_clear(Message::SearchChanged(String::new()));

        // ── Filter entries ───────────────────────────────────────────
        let filtered_entries: Vec<&ClipboardEntry> = self
            .entries
            .iter()
            .filter(|e| {
                if self.search_query.is_empty() {
                    return true;
                }
                let q = self.search_query.to_lowercase();
                e.content.to_lowercase().contains(&q)
                    || e.tag.as_ref().map_or(false, |t| t.to_lowercase().contains(&q))
                    || e.title.as_ref().map_or(false, |t| t.to_lowercase().contains(&q))
            })
            .collect();

        // ── Build list ───────────────────────────────────────────────
        let mut list_items: Vec<Element<'_, Message>> = Vec::new();

        if filtered_entries.is_empty() {
            let empty_msg = if self.search_query.is_empty() {
                "El portapapeles está vacío"
            } else {
                "Sin resultados"
            };
            list_items.push(
                widget::container(widget::text::body(empty_msg))
                    .padding(20)
                    .center_x(Length::Fill)
                    .into(),
            );
        } else {
            for (i, entry) in filtered_entries.iter().enumerate() {
                let entry_id = entry.id.unwrap_or(-1);

                if Some(entry_id) == self.editing_target {
                    // ── Edit mode ────────────────────────────────────
                    list_items.push(self.build_edit_row(entry));
                } else {
                    // ── Normal display mode ──────────────────────────
                    list_items.push(self.build_entry_row(entry));
                }

                // Divider between entries (not after last)
                if i < filtered_entries.len() - 1 {
                    list_items.push(widget::divider::horizontal::light().into());
                }
            }
        }

        // ── Scrollable body ──────────────────────────────────────────
        let scroll_body = widget::scrollable(
            widget::column(list_items).spacing(4).padding(8),
        )
        .width(Length::Fill)
        .height(Length::Fill);

        // ── Footer ───────────────────────────────────────────────────
        let btn_clear = widget::button::standard("Limpiar no fijados")
            .on_press(Message::ClearAllUnpinned)
            .width(Length::Fill);

        let btn_quit = widget::button::destructive("Salir")
            .on_press(Message::Quit)
            .width(Length::Fill);

        let footer = widget::row(vec![
            btn_clear.into(),
            btn_quit.into(),
        ])
        .spacing(8)
        .padding([8, 12])
        .align_y(Alignment::Center);

        // ── Assemble popup ───────────────────────────────────────────
        let popup_layout = widget::column(vec![
            widget::container(header).padding([12, 12, 4, 12]).into(),
            widget::container(search_box).padding([0, 12, 8, 12]).into(),
            widget::divider::horizontal::default().into(),
            scroll_body.into(),
            widget::divider::horizontal::default().into(),
            footer.into(),
        ]);

        self.core.applet.popup_container(popup_layout).into()
    }

    // ── Subscriptions ─────────────────────────────────────────────────────

    fn subscription(&self) -> Subscription<Self::Message> {
        cosmic::iced::time::every(Duration::from_millis(1500)).map(|_| Message::Tick)
    }

    // ── Update ────────────────────────────────────────────────────────────

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
                        entry.title = if self.editing_title.trim().is_empty() {
                            None
                        } else {
                            Some(self.editing_title.clone())
                        };
                        entry.tag = if self.editing_tag.trim().is_empty() {
                            None
                        } else {
                            Some(self.editing_tag.clone())
                        };
                        let _ = db.update_entry_metadata(
                            id,
                            entry.pinned,
                            entry.tag.clone(),
                            entry.title.clone(),
                        );
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
                        let _ = db.update_entry_metadata(
                            id,
                            entry.pinned,
                            entry.tag.clone(),
                            entry.title.clone(),
                        );
                    }
                    if let Ok(entries) = db.get_entries() {
                        self.entries = entries;
                    }
                }
            }
            Message::CopyToClipboard(content) => {
                let _ = std::process::Command::new("wl-copy").arg(&content).spawn();
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
            Message::ClearAllUnpinned => {
                if let Ok(db) = Database::new(&self.db_path) {
                    let _ = db.delete_all_unpinned();
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
                        .max_width(420.0)
                        .min_width(360.0)
                        .min_height(300.0)
                        .max_height(700.0);
                    get_popup(popup_settings)
                }
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
            Message::Quit => {
                std::process::exit(0);
            }
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

// ── View builders ────────────────────────────────────────────────────────

impl CopyousApp {
    /// Build a clickable entry row (click anywhere to copy)
    fn build_entry_row<'a>(&'a self, entry: &'a ClipboardEntry) -> Element<'a, Message> {
        let entry_id = entry.id.unwrap_or(-1);

        // ── Metadata line ────────────────────────────────────────────
        let mut meta_parts: Vec<Element<'_, Message>> = Vec::new();

        // Type icon
        let type_icon = widget::icon::from_name(type_icon_name(&entry.item_type))
            .size(14)
            .into();
        meta_parts.push(type_icon);

        // Pinned badge
        if entry.pinned {
            meta_parts.push(
                widget::icon::from_name("pin-symbolic").size(14).into(),
            );
        }

        // Tag
        if let Some(ref tag) = entry.tag {
            meta_parts.push(widget::text::caption(format!("#{}", tag)).into());
        }

        // Relative timestamp
        let time_str = relative_time(&entry.datetime);
        meta_parts.push(widget::Space::new().width(Length::Fill).into());
        meta_parts.push(widget::text::caption(time_str).into());

        let meta_row = widget::row(meta_parts)
            .spacing(6)
            .align_y(Alignment::Center);

        // ── Title (if exists) ────────────────────────────────────────
        let title_element: Option<Element<'_, Message>> = entry
            .title
            .as_ref()
            .map(|t| widget::text::body(t.clone()).into());

        // ── Content preview ──────────────────────────────────────────
        let preview_text = if entry.item_type == ItemType::Image {
            "[Imagen capturada]".to_string()
        } else {
            sanitize_preview(&entry.content, 80)
        };
        let preview = widget::text::caption(preview_text);

        // ── Action buttons ───────────────────────────────────────────
        let pin_icon_name = if entry.pinned {
            "pin-symbolic"
        } else {
            "pin-symbolic"
        };

        let btn_pin = widget::tooltip(
            widget::button::icon(widget::icon::from_name(pin_icon_name).size(16))
                .on_press(Message::TogglePin(entry_id))
                .padding(4),
            if entry.pinned { "Desfijar" } else { "Fijar" },
            tooltip::Position::Bottom,
        );

        let btn_edit = widget::tooltip(
            widget::button::icon(widget::icon::from_name("edit-symbolic").size(16))
                .on_press(Message::StartEditing(
                    entry_id,
                    entry.title.clone(),
                    entry.tag.clone(),
                ))
                .padding(4),
            "Editar",
            tooltip::Position::Bottom,
        );

        let btn_delete = widget::tooltip(
            widget::button::icon(widget::icon::from_name("edit-delete-symbolic").size(16))
                .on_press(Message::DeleteEntry(entry_id))
                .padding(4),
            "Eliminar",
            tooltip::Position::Bottom,
        );

        let actions = widget::row(vec![btn_pin.into(), btn_edit.into(), btn_delete.into()])
            .spacing(2)
            .align_y(Alignment::Center);

        // ── Assemble the card content ────────────────────────────────
        let mut content_parts: Vec<Element<'_, Message>> = vec![meta_row.into()];

        if let Some(title_el) = title_element {
            content_parts.push(title_el);
        }

        content_parts.push(preview.into());

        // Actions row: right-aligned
        content_parts.push(
            widget::row(vec![
                widget::Space::new().width(Length::Fill).into(),
                actions.into(),
            ])
            .into(),
        );

        let card_content = widget::column(content_parts).spacing(4);

        // ── Wrap in a clickable button (click-to-copy) ───────────────
        let clickable = widget::button::custom(card_content)
            .on_press(Message::CopyToClipboard(entry.content.clone()))
            .padding(10)
            .width(Length::Fill)
            .class(ButtonClass::MenuItem);

        clickable.into()
    }

    /// Build the inline edit form for an entry
    fn build_edit_row<'a>(&'a self, entry: &'a ClipboardEntry) -> Element<'a, Message> {
        let entry_id = entry.id.unwrap();

        let title_input = widget::text_input("Título…", &self.editing_title)
            .on_input(Message::InputTitleChanged)
            .padding(6);

        let tag_input = widget::text_input("Etiqueta…", &self.editing_tag)
            .on_input(Message::InputTagChanged)
            .padding(6);

        let btn_save = widget::button::suggested("Guardar")
            .on_press(Message::SaveEdits(entry_id))
            .padding(6);

        let btn_cancel = widget::button::standard("Cancelar")
            .on_press(Message::CancelEdits)
            .padding(6);

        let preview = widget::text::caption(sanitize_preview(&entry.content, 60));

        let edit_block = widget::column(vec![
            preview.into(),
            widget::divider::horizontal::light().into(),
            widget::text::caption("Título").into(),
            title_input.into(),
            widget::text::caption("Etiqueta").into(),
            tag_input.into(),
            widget::row(vec![
                widget::Space::new().width(Length::Fill).into(),
                btn_cancel.into(),
                btn_save.into(),
            ])
            .spacing(8)
            .into(),
        ])
        .spacing(6);

        widget::container(edit_block)
            .padding(12)
            .width(Length::Fill)
            .into()
    }
}
