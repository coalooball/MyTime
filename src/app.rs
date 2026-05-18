use std::collections::BTreeMap;
use std::time::Duration;

use chrono::{Datelike, Local};
use iced::{time, window, Element, Size, Subscription, Task};

use crate::i18n::{category_value_from_label, tr, Language, TextKey};
use crate::model::{
    day_range, error_message, parse_date, stats_range, ActivityForm, AppResult, CurrentActivity,
    EntryForm, MainTab, StatsData, StatsView, TimeEntry,
};
use crate::repository::Repository;
use crate::ui;

#[derive(Clone, Debug)]
pub(crate) enum Message {
    MainWindowOpened(window::Id),
    EditWindowOpened(window::Id),
    WindowCloseRequested(window::Id),
    Tick,
    Refresh,
    ToggleLanguage,
    SwitchTab(MainTab),
    RealtimeActivityChanged(String),
    RealtimeCategoryChanged(String),
    RealtimeLocationChanged(String),
    RealtimeDescriptionChanged(String),
    StartActivity,
    EndActivity,
    ManualActivityChanged(String),
    ManualCategoryChanged(String),
    ManualLocationChanged(String),
    ManualDescriptionChanged(String),
    ManualStartDateChanged(String),
    ManualStartTimeChanged(String),
    ManualEndDateChanged(String),
    ManualEndTimeChanged(String),
    OpenManualEntry,
    CloseManualEntry,
    SaveManualEntry,
    RecordsDateChanged(String),
    Today,
    EditEntry(i64),
    EditActivityChanged(String),
    EditCategoryChanged(String),
    EditLocationChanged(String),
    EditDescriptionChanged(String),
    EditStartDateChanged(String),
    EditStartTimeChanged(String),
    EditEndDateChanged(String),
    EditEndTimeChanged(String),
    SaveEdit,
    DeleteEditingEntry,
    CancelEdit,
    StatsDateChanged(String),
    ChangeStatsView(StatsView),
    DismissMessage,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum MessageKind {
    Info,
    Error,
}

pub(crate) struct MyTimeApp {
    pub(crate) repo: Result<Repository, String>,
    pub(crate) main_window: Option<window::Id>,
    pub(crate) edit_window: Option<window::Id>,
    pub(crate) language: Language,
    pub(crate) active_tab: MainTab,
    pub(crate) manual_form: EntryForm,
    pub(crate) manual_entry_dialog_open: bool,
    pub(crate) realtime_form: ActivityForm,
    pub(crate) editing_form: Option<EntryForm>,
    pub(crate) records_date: String,
    pub(crate) stats_date: String,
    pub(crate) stats_view: StatsView,
    pub(crate) today_entries: Vec<TimeEntry>,
    pub(crate) current_activity: Option<CurrentActivity>,
    pub(crate) stats: Option<StatsData>,
    pub(crate) message: Option<(String, MessageKind)>,
}

impl MyTimeApp {
    pub(crate) fn new() -> (Self, Task<Message>) {
        let repo = Repository::open().map_err(|err| err.to_string());
        let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
        let (main_window, open_main_window) = window::open(window::Settings {
            position: window::Position::Centered,
            exit_on_close_request: false,
            ..window::Settings::default()
        });
        let mut app = Self {
            repo,
            main_window: Some(main_window),
            edit_window: None,
            language: Language::Chinese,
            active_tab: MainTab::Records,
            manual_form: EntryForm::new_default(),
            manual_entry_dialog_open: false,
            realtime_form: ActivityForm::new(),
            editing_form: None,
            records_date: today.clone(),
            stats_date: today,
            stats_view: StatsView::Week,
            today_entries: Vec::new(),
            current_activity: None,
            stats: None,
            message: None,
        };
        app.refresh_all();
        (app, open_main_window.map(Message::MainWindowOpened))
    }

    pub(crate) fn title(&self, window_id: window::Id) -> String {
        if self.edit_window == Some(window_id) {
            tr(self.language, TextKey::EditEntry).to_string()
        } else {
            "MyTime".to_string()
        }
    }

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![window::close_requests().map(Message::WindowCloseRequested)];

        if self.current_activity.is_some() {
            subscriptions.push(time::every(Duration::from_secs(1)).map(|_| Message::Tick));
        }

        Subscription::batch(subscriptions)
    }

    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MainWindowOpened(id) => self.main_window = Some(id),
            Message::EditWindowOpened(id) => {
                self.edit_window = Some(id);
                return window::gain_focus(id);
            }
            Message::WindowCloseRequested(id) => {
                if self.edit_window == Some(id) {
                    self.editing_form = None;
                    self.edit_window = None;
                    return window::close(id);
                }

                if self.main_window == Some(id) {
                    return iced::exit();
                }

                return window::close(id);
            }
            Message::Tick => {}
            Message::Refresh => self.refresh_all(),
            Message::ToggleLanguage => self.language = self.language.toggled(),
            Message::SwitchTab(tab) => self.active_tab = tab,
            Message::RealtimeActivityChanged(value) => self.realtime_form.activity = value,
            Message::RealtimeCategoryChanged(value) => {
                self.realtime_form.category = category_value_from_label(&value);
            }
            Message::RealtimeLocationChanged(value) => self.realtime_form.location = value,
            Message::RealtimeDescriptionChanged(value) => self.realtime_form.description = value,
            Message::StartActivity => {
                let form = self.realtime_form.clone();
                self.with_repo(
                    |repo| {
                        repo.start_activity(
                            &form.activity,
                            &form.category,
                            &form.location,
                            &form.description,
                        )
                    },
                    TextKey::ActivityStarted,
                );
                if self.is_info_message() {
                    self.realtime_form = ActivityForm::new();
                }
            }
            Message::EndActivity => {
                self.with_repo(|repo| repo.end_activity(), TextKey::ActivityEnded);
            }
            Message::ManualActivityChanged(value) => self.manual_form.activity = value,
            Message::ManualCategoryChanged(value) => {
                self.manual_form.category = category_value_from_label(&value);
            }
            Message::ManualLocationChanged(value) => self.manual_form.location = value,
            Message::ManualDescriptionChanged(value) => self.manual_form.description = value,
            Message::ManualStartDateChanged(value) => self.manual_form.start_date = value,
            Message::ManualStartTimeChanged(value) => self.manual_form.start_time = value,
            Message::ManualEndDateChanged(value) => self.manual_form.end_date = value,
            Message::ManualEndTimeChanged(value) => self.manual_form.end_time = value,
            Message::OpenManualEntry => {
                self.manual_form = EntryForm::new_default();
                self.manual_entry_dialog_open = true;
            }
            Message::CloseManualEntry => {
                self.manual_entry_dialog_open = false;
                self.manual_form = EntryForm::new_default();
            }
            Message::SaveManualEntry => match self.manual_form.to_entry() {
                Ok(entry) => {
                    self.with_repo(
                        |repo| {
                            repo.insert_entry(
                                entry.start_time,
                                entry.end_time,
                                &entry.activity,
                                &entry.category,
                                &entry.location,
                                &entry.description,
                            )
                        },
                        TextKey::EntrySaved,
                    );
                    if self.is_info_message() {
                        self.manual_entry_dialog_open = false;
                        self.manual_form = EntryForm::new_default();
                    }
                }
                Err(err) => self.set_error(err.to_string()),
            },
            Message::RecordsDateChanged(value) => {
                self.records_date = value;
                self.refresh_records();
            }
            Message::Today => {
                self.records_date = Local::now().date_naive().format("%Y-%m-%d").to_string();
                self.refresh_records();
            }
            Message::EditEntry(id) => {
                if let Some(entry) = self.today_entries.iter().find(|entry| entry.id == id) {
                    self.editing_form = Some(EntryForm::from_entry(entry));
                    return self.open_edit_window();
                }
            }
            Message::EditActivityChanged(value) => {
                if let Some(form) = &mut self.editing_form {
                    form.activity = value;
                }
            }
            Message::EditCategoryChanged(value) => {
                if let Some(form) = &mut self.editing_form {
                    form.category = category_value_from_label(&value);
                }
            }
            Message::EditLocationChanged(value) => {
                if let Some(form) = &mut self.editing_form {
                    form.location = value;
                }
            }
            Message::EditDescriptionChanged(value) => {
                if let Some(form) = &mut self.editing_form {
                    form.description = value;
                }
            }
            Message::EditStartDateChanged(value) => {
                if let Some(form) = &mut self.editing_form {
                    form.start_date = value;
                }
            }
            Message::EditStartTimeChanged(value) => {
                if let Some(form) = &mut self.editing_form {
                    form.start_time = value;
                }
            }
            Message::EditEndDateChanged(value) => {
                if let Some(form) = &mut self.editing_form {
                    form.end_date = value;
                }
            }
            Message::EditEndTimeChanged(value) => {
                if let Some(form) = &mut self.editing_form {
                    form.end_time = value;
                }
            }
            Message::SaveEdit => {
                if let Some(form) = self.editing_form.clone() {
                    match form.to_entry() {
                        Ok(entry) => {
                            self.with_repo(|repo| repo.update_entry(&entry), TextKey::EntryUpdated);
                            if self.is_info_message() {
                                return self.close_edit_window();
                            }
                        }
                        Err(err) => self.set_error(err.to_string()),
                    }
                }
            }
            Message::DeleteEditingEntry => {
                if let Some(id) = self.editing_form.as_ref().and_then(|form| form.id) {
                    self.with_repo(|repo| repo.delete_entry(id), TextKey::EntryDeleted);
                    if self.is_info_message() {
                        return self.close_edit_window();
                    }
                }
            }
            Message::CancelEdit => return self.close_edit_window(),
            Message::StatsDateChanged(value) => {
                self.stats_date = value;
                self.refresh_stats();
            }
            Message::ChangeStatsView(view) => {
                self.stats_view = view;
                self.refresh_stats();
            }
            Message::DismissMessage => self.message = None,
        }
        Task::none()
    }

    pub(crate) fn view(&self, window_id: window::Id) -> Element<'_, Message> {
        if self.edit_window == Some(window_id) {
            return ui::edit_window_view(self);
        }

        ui::main_window_view(self)
    }

    fn refresh_all(&mut self) {
        self.refresh_current_activity();
        self.refresh_records();
        self.refresh_stats();
    }

    fn refresh_current_activity(&mut self) {
        let Ok(repo) = &self.repo else {
            return;
        };

        match repo.get_current_activity() {
            Ok(current_activity) => self.current_activity = current_activity,
            Err(err) => self.set_error(error_message(self.language, &err)),
        }
    }

    fn refresh_records(&mut self) {
        let Ok(date) = parse_date(&self.records_date) else {
            return;
        };
        if let Ok(repo) = &self.repo {
            let (start, end) = day_range(date);
            match repo.list_between(start, end) {
                Ok(today_entries) => self.today_entries = today_entries,
                Err(err) => self.set_error(error_message(self.language, &err)),
            }
        }
    }

    fn refresh_stats(&mut self) {
        let Ok(date) = parse_date(&self.stats_date) else {
            return;
        };
        let Ok(repo) = &self.repo else {
            return;
        };

        let (start, end, group_by_month) = stats_range(date, self.stats_view);
        match repo.list_between(start, end) {
            Ok(entries) => {
                let mut category_minutes = BTreeMap::new();
                let mut period_minutes = BTreeMap::new();

                for entry in &entries {
                    *category_minutes
                        .entry(entry.category.clone())
                        .or_insert(0.0) += entry.minutes();
                    let key = if group_by_month {
                        format!(
                            "{}-{:02}",
                            entry.start_time.year(),
                            entry.start_time.month()
                        )
                    } else {
                        entry.start_time.date().format("%Y-%m-%d").to_string()
                    };
                    *period_minutes.entry(key).or_insert(0.0) += entry.minutes();
                }

                self.stats = Some(StatsData {
                    entries,
                    category_minutes,
                    period_minutes,
                });
            }
            Err(err) => self.set_error(error_message(self.language, &err)),
        }
    }

    fn with_repo(&mut self, action: impl FnOnce(&Repository) -> AppResult<()>, ok_msg: TextKey) {
        match &self.repo {
            Ok(repo) => match action(repo) {
                Ok(()) => {
                    self.set_info(tr(self.language, ok_msg));
                    self.refresh_all();
                }
                Err(err) => self.set_error(error_message(self.language, &err)),
            },
            Err(err) => self.set_error(err.clone()),
        }
    }

    fn open_edit_window(&mut self) -> Task<Message> {
        if let Some(id) = self.edit_window {
            return window::gain_focus(id);
        }

        let (id, open) = window::open(window::Settings {
            size: Size::new(460.0, 420.0),
            min_size: Some(Size::new(420.0, 360.0)),
            position: window::Position::Centered,
            exit_on_close_request: false,
            level: window::Level::AlwaysOnTop,
            ..window::Settings::default()
        });
        self.edit_window = Some(id);

        open.map(Message::EditWindowOpened)
    }

    fn close_edit_window(&mut self) -> Task<Message> {
        self.editing_form = None;

        if let Some(id) = self.edit_window.take() {
            window::close(id)
        } else {
            Task::none()
        }
    }

    fn set_info(&mut self, message: impl Into<String>) {
        self.message = Some((message.into(), MessageKind::Info));
    }

    fn set_error(&mut self, message: impl Into<String>) {
        self.message = Some((message.into(), MessageKind::Error));
    }

    fn is_info_message(&self) -> bool {
        self.message
            .as_ref()
            .is_some_and(|(_, kind)| matches!(kind, MessageKind::Info))
    }
}
