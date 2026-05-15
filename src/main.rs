use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use iced::alignment;
use iced::widget::{
    button, column, container, horizontal_rule, horizontal_space, opaque, pick_list, row,
    scrollable, stack, text, text_input, Column, Row,
};
use iced::{
    application, border, time, Border, Color, Element, Font, Length, Shadow, Subscription, Task,
    Theme, Vector,
};
use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;

mod i18n;

use i18n::{category_label, category_value_from_label, tr, Language, TextKey, CATEGORIES};

const DB_FILE: &str = "time_tracker.db";

fn main() -> iced::Result {
    application("MyTime", MyTimeApp::update, MyTimeApp::view)
        .theme(|_| Theme::Light)
        .font(include_bytes!("/System/Library/Fonts/PingFang.ttc").as_slice())
        .font(include_bytes!("/System/Library/Fonts/Hiragino Sans GB.ttc").as_slice())
        .default_font(Font::with_name("PingFang SC"))
        .subscription(MyTimeApp::subscription)
        .run_with(|| (MyTimeApp::new(), Task::none()))
}

#[derive(Debug, Error)]
enum AppError {
    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("开始时间必须早于结束时间")]
    InvalidTimeRange,
    #[error("活动名称不能为空")]
    EmptyActivity,
    #[error("类别不能为空")]
    EmptyCategory,
    #[error("没有正在进行的活动")]
    NoActiveActivity,
    #[error("日期格式应为 YYYY-MM-DD")]
    InvalidDate,
    #[error("时间格式应为 HH:MM")]
    InvalidClockTime,
}

type AppResult<T> = Result<T, AppError>;

#[derive(Clone, Debug)]
struct TimeEntry {
    id: i64,
    start_time: NaiveDateTime,
    end_time: NaiveDateTime,
    activity: String,
    category: String,
    description: String,
}

impl TimeEntry {
    fn hours(&self) -> f64 {
        (self.end_time - self.start_time).num_seconds() as f64 / 3600.0
    }
}

#[derive(Clone, Debug)]
struct CurrentActivity {
    start_time: NaiveDateTime,
    activity: String,
    category: String,
    description: String,
}

struct Repository {
    conn: Connection,
}

impl Repository {
    fn open() -> AppResult<Self> {
        let db_path = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(DB_FILE);
        let conn = Connection::open(db_path)?;
        let repo = Self { conn };
        repo.init()?;
        Ok(repo)
    }

    fn init(&self) -> AppResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS time_entry (
                id INTEGER NOT NULL PRIMARY KEY,
                start_time DATETIME NOT NULL,
                end_time DATETIME NOT NULL,
                activity VARCHAR(200) NOT NULL,
                category VARCHAR(50) NOT NULL,
                description TEXT
            );

            CREATE TABLE IF NOT EXISTS current_activity (
                id INTEGER NOT NULL PRIMARY KEY CHECK (id = 1),
                start_time DATETIME NOT NULL,
                activity VARCHAR(200) NOT NULL,
                category VARCHAR(50) NOT NULL,
                description TEXT
            );
            ",
        )?;
        Ok(())
    }

    fn list_between(&self, start: NaiveDateTime, end: NaiveDateTime) -> AppResult<Vec<TimeEntry>> {
        let mut stmt = self.conn.prepare(
            "
            SELECT id, start_time, end_time, activity, category, COALESCE(description, '')
            FROM time_entry
            WHERE start_time >= ?1 AND start_time < ?2
            ORDER BY start_time DESC
            ",
        )?;

        let entries = stmt
            .query_map(params![start, end], row_to_entry)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(entries)
    }

    fn list_all_recent(&self, limit: usize) -> AppResult<Vec<TimeEntry>> {
        let mut stmt = self.conn.prepare(
            "
            SELECT id, start_time, end_time, activity, category, COALESCE(description, '')
            FROM time_entry
            ORDER BY start_time DESC
            LIMIT ?1
            ",
        )?;

        let entries = stmt
            .query_map(params![limit as i64], row_to_entry)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(entries)
    }

    fn insert_entry(
        &self,
        start_time: NaiveDateTime,
        end_time: NaiveDateTime,
        activity: &str,
        category: &str,
        description: &str,
    ) -> AppResult<()> {
        validate_entry(start_time, end_time, activity, category)?;
        self.conn.execute(
            "
            INSERT INTO time_entry (start_time, end_time, activity, category, description)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ",
            params![
                start_time,
                end_time,
                activity.trim(),
                category.trim(),
                description.trim()
            ],
        )?;
        Ok(())
    }

    fn update_entry(&self, entry: &TimeEntry) -> AppResult<()> {
        validate_entry(
            entry.start_time,
            entry.end_time,
            &entry.activity,
            &entry.category,
        )?;
        self.conn.execute(
            "
            UPDATE time_entry
            SET start_time = ?1, end_time = ?2, activity = ?3, category = ?4, description = ?5
            WHERE id = ?6
            ",
            params![
                entry.start_time,
                entry.end_time,
                entry.activity.trim(),
                entry.category.trim(),
                entry.description.trim(),
                entry.id
            ],
        )?;
        Ok(())
    }

    fn delete_entry(&self, id: i64) -> AppResult<()> {
        self.conn
            .execute("DELETE FROM time_entry WHERE id = ?1", params![id])?;
        Ok(())
    }

    fn get_current_activity(&self) -> AppResult<Option<CurrentActivity>> {
        self.conn
            .query_row(
                "
                SELECT start_time, activity, category, COALESCE(description, '')
                FROM current_activity
                WHERE id = 1
                ",
                [],
                |row| {
                    Ok(CurrentActivity {
                        start_time: row.get(0)?,
                        activity: row.get(1)?,
                        category: row.get(2)?,
                        description: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(AppError::from)
    }

    fn start_activity(&self, activity: &str, category: &str, description: &str) -> AppResult<()> {
        let now = Local::now().naive_local();
        validate_text(activity, category)?;
        self.conn.execute(
            "
            INSERT INTO current_activity (id, start_time, activity, category, description)
            VALUES (1, ?1, ?2, ?3, ?4)
            ON CONFLICT(id) DO UPDATE SET
                start_time = excluded.start_time,
                activity = excluded.activity,
                category = excluded.category,
                description = excluded.description
            ",
            params![now, activity.trim(), category.trim(), description.trim()],
        )?;
        Ok(())
    }

    fn end_activity(&self) -> AppResult<()> {
        let current = self
            .get_current_activity()?
            .ok_or(AppError::NoActiveActivity)?;
        let now = Local::now().naive_local();
        self.insert_entry(
            current.start_time,
            now,
            &current.activity,
            &current.category,
            &current.description,
        )?;
        self.conn
            .execute("DELETE FROM current_activity WHERE id = 1", [])?;
        Ok(())
    }
}

fn row_to_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<TimeEntry> {
    Ok(TimeEntry {
        id: row.get(0)?,
        start_time: row.get(1)?,
        end_time: row.get(2)?,
        activity: row.get(3)?,
        category: row.get(4)?,
        description: row.get(5)?,
    })
}

fn validate_text(activity: &str, category: &str) -> AppResult<()> {
    if activity.trim().is_empty() {
        return Err(AppError::EmptyActivity);
    }
    if category.trim().is_empty() {
        return Err(AppError::EmptyCategory);
    }
    Ok(())
}

fn validate_entry(
    start_time: NaiveDateTime,
    end_time: NaiveDateTime,
    activity: &str,
    category: &str,
) -> AppResult<()> {
    validate_text(activity, category)?;
    if end_time <= start_time {
        return Err(AppError::InvalidTimeRange);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MainTab {
    Records,
    Statistics,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StatsView {
    Week,
    Month,
    Year,
}

impl StatsView {
    fn label(self, language: Language) -> &'static str {
        match self {
            Self::Week => tr(language, TextKey::Week),
            Self::Month => tr(language, TextKey::Month),
            Self::Year => tr(language, TextKey::Year),
        }
    }
}

fn error_message(language: Language, error: &AppError) -> String {
    match error {
        AppError::Database(err) => format!("{}: {err}", tr(language, TextKey::DatabaseError)),
        AppError::InvalidTimeRange => tr(language, TextKey::InvalidTimeRange).to_string(),
        AppError::EmptyActivity => tr(language, TextKey::EmptyActivity).to_string(),
        AppError::EmptyCategory => tr(language, TextKey::EmptyCategory).to_string(),
        AppError::NoActiveActivity => tr(language, TextKey::NoActiveActivity).to_string(),
        AppError::InvalidDate => tr(language, TextKey::InvalidDate).to_string(),
        AppError::InvalidClockTime => tr(language, TextKey::InvalidClockTime).to_string(),
    }
}

#[derive(Clone, Debug)]
struct EntryForm {
    id: Option<i64>,
    start_date: String,
    start_time: String,
    end_date: String,
    end_time: String,
    activity: String,
    category: String,
    description: String,
}

impl EntryForm {
    fn new_default() -> Self {
        let now = Local::now().naive_local();
        let next_hour = now + chrono::Duration::hours(1);
        Self {
            id: None,
            start_date: now.date().format("%Y-%m-%d").to_string(),
            start_time: format_hm(now.time()),
            end_date: next_hour.date().format("%Y-%m-%d").to_string(),
            end_time: format_hm(next_hour.time()),
            activity: String::new(),
            category: CATEGORIES[0].0.to_string(),
            description: String::new(),
        }
    }

    fn from_entry(entry: &TimeEntry) -> Self {
        Self {
            id: Some(entry.id),
            start_date: entry.start_time.date().format("%Y-%m-%d").to_string(),
            start_time: format_hm(entry.start_time.time()),
            end_date: entry.end_time.date().format("%Y-%m-%d").to_string(),
            end_time: format_hm(entry.end_time.time()),
            activity: entry.activity.clone(),
            category: entry.category.clone(),
            description: entry.description.clone(),
        }
    }

    fn to_entry(&self) -> AppResult<TimeEntry> {
        Ok(TimeEntry {
            id: self.id.unwrap_or_default(),
            start_time: parse_datetime(&self.start_date, &self.start_time)?,
            end_time: parse_datetime(&self.end_date, &self.end_time)?,
            activity: self.activity.clone(),
            category: self.category.clone(),
            description: self.description.clone(),
        })
    }
}

#[derive(Clone, Debug)]
struct ActivityForm {
    activity: String,
    category: String,
    description: String,
}

impl ActivityForm {
    fn new() -> Self {
        Self {
            activity: String::new(),
            category: CATEGORIES[0].0.to_string(),
            description: String::new(),
        }
    }
}

#[derive(Clone, Debug)]
struct StatsData {
    entries: Vec<TimeEntry>,
    category_hours: BTreeMap<String, f64>,
    period_hours: BTreeMap<String, f64>,
}

#[derive(Clone, Debug)]
enum Message {
    Tick,
    Refresh,
    ToggleLanguage,
    SwitchTab(MainTab),
    RealtimeActivityChanged(String),
    RealtimeCategoryChanged(String),
    RealtimeDescriptionChanged(String),
    StartActivity,
    EndActivity,
    ManualActivityChanged(String),
    ManualCategoryChanged(String),
    ManualDescriptionChanged(String),
    ManualStartDateChanged(String),
    ManualStartTimeChanged(String),
    ManualEndDateChanged(String),
    ManualEndTimeChanged(String),
    SaveManualEntry,
    RecordsDateChanged(String),
    Today,
    EditEntry(i64),
    DeleteEntry(i64),
    EditActivityChanged(String),
    EditCategoryChanged(String),
    EditDescriptionChanged(String),
    EditStartDateChanged(String),
    EditStartTimeChanged(String),
    EditEndDateChanged(String),
    EditEndTimeChanged(String),
    SaveEdit,
    CancelEdit,
    StatsDateChanged(String),
    ChangeStatsView(StatsView),
    DismissMessage,
}

#[derive(Clone, Copy, Debug)]
enum MessageKind {
    Info,
    Error,
}

struct MyTimeApp {
    repo: Result<Repository, String>,
    language: Language,
    active_tab: MainTab,
    manual_form: EntryForm,
    realtime_form: ActivityForm,
    editing_form: Option<EntryForm>,
    records_date: String,
    stats_date: String,
    stats_view: StatsView,
    today_entries: Vec<TimeEntry>,
    recent_entries: Vec<TimeEntry>,
    current_activity: Option<CurrentActivity>,
    stats: Option<StatsData>,
    message: Option<(String, MessageKind)>,
}

impl MyTimeApp {
    fn new() -> Self {
        let repo = Repository::open().map_err(|err| err.to_string());
        let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
        let mut app = Self {
            repo,
            language: Language::Chinese,
            active_tab: MainTab::Records,
            manual_form: EntryForm::new_default(),
            realtime_form: ActivityForm::new(),
            editing_form: None,
            records_date: today.clone(),
            stats_date: today,
            stats_view: StatsView::Week,
            today_entries: Vec::new(),
            recent_entries: Vec::new(),
            current_activity: None,
            stats: None,
            message: None,
        };
        app.refresh_all();
        app
    }

    fn subscription(&self) -> Subscription<Message> {
        if self.current_activity.is_some() {
            time::every(Duration::from_secs(1)).map(|_| Message::Tick)
        } else {
            Subscription::none()
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {}
            Message::Refresh => self.refresh_all(),
            Message::ToggleLanguage => self.language = self.language.toggled(),
            Message::SwitchTab(tab) => self.active_tab = tab,
            Message::RealtimeActivityChanged(value) => self.realtime_form.activity = value,
            Message::RealtimeCategoryChanged(value) => {
                self.realtime_form.category = category_value_from_label(&value);
            }
            Message::RealtimeDescriptionChanged(value) => self.realtime_form.description = value,
            Message::StartActivity => {
                let form = self.realtime_form.clone();
                self.with_repo(
                    |repo| repo.start_activity(&form.activity, &form.category, &form.description),
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
            Message::ManualDescriptionChanged(value) => self.manual_form.description = value,
            Message::ManualStartDateChanged(value) => self.manual_form.start_date = value,
            Message::ManualStartTimeChanged(value) => self.manual_form.start_time = value,
            Message::ManualEndDateChanged(value) => self.manual_form.end_date = value,
            Message::ManualEndTimeChanged(value) => self.manual_form.end_time = value,
            Message::SaveManualEntry => match self.manual_form.to_entry() {
                Ok(entry) => {
                    self.with_repo(
                        |repo| {
                            repo.insert_entry(
                                entry.start_time,
                                entry.end_time,
                                &entry.activity,
                                &entry.category,
                                &entry.description,
                            )
                        },
                        TextKey::EntrySaved,
                    );
                    if self.is_info_message() {
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
                if let Some(entry) = self
                    .today_entries
                    .iter()
                    .chain(self.recent_entries.iter())
                    .find(|entry| entry.id == id)
                {
                    self.editing_form = Some(EntryForm::from_entry(entry));
                }
            }
            Message::DeleteEntry(id) => {
                self.with_repo(|repo| repo.delete_entry(id), TextKey::EntryDeleted);
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
                if let Some(form) = &self.editing_form {
                    match form.to_entry() {
                        Ok(entry) => {
                            self.with_repo(|repo| repo.update_entry(&entry), TextKey::EntryUpdated);
                            if self.is_info_message() {
                                self.editing_form = None;
                            }
                        }
                        Err(err) => self.set_error(err.to_string()),
                    }
                }
            }
            Message::CancelEdit => self.editing_form = None,
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

    fn view(&self) -> Element<'_, Message> {
        let mut page = column![self.top_bar()].spacing(12).padding(16);

        if let Some(message) = self.info_message_view() {
            page = page.push(message);
        }

        let page: Element<_> = if let Err(err) = &self.repo {
            page.push(text(err.clone()).size(18)).into()
        } else {
            let body = match self.active_tab {
                MainTab::Records => self.records_view(),
                MainTab::Statistics => self.statistics_view(),
            };
            page.push(body).into()
        };

        let base = scrollable(page)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        if let Some(dialog) = self.error_dialog_view() {
            stack![base, dialog]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            base
        }
    }

    fn top_bar(&self) -> Element<'_, Message> {
        row![
            text("MyTime").size(32),
            button(tr(self.language, TextKey::Records))
                .on_press(Message::SwitchTab(MainTab::Records)),
            button(tr(self.language, TextKey::Statistics))
                .on_press(Message::SwitchTab(MainTab::Statistics)),
            button(tr(self.language, TextKey::Refresh)).on_press(Message::Refresh),
            horizontal_space(),
            button(tr(self.language, TextKey::LanguageSwitch)).on_press(Message::ToggleLanguage),
        ]
        .spacing(10)
        .width(Length::Fill)
        .align_y(alignment::Vertical::Center)
        .into()
    }

    fn info_message_view(&self) -> Option<Element<'_, Message>> {
        let Some((message, MessageKind::Info)) = &self.message else {
            return None;
        };

        Some(
            container(
                row![
                    text(format!("{}: {message}", tr(self.language, TextKey::Info)))
                        .width(Length::Fill),
                    button(tr(self.language, TextKey::Close)).on_press(Message::DismissMessage),
                ]
                .spacing(8)
                .align_y(alignment::Vertical::Center),
            )
            .padding(10)
            .width(Length::Fill)
            .into(),
        )
    }

    fn error_dialog_view(&self) -> Option<Element<'_, Message>> {
        let Some((message, MessageKind::Error)) = &self.message else {
            return None;
        };

        let dialog = container(
            column![
                text(tr(self.language, TextKey::Error)).size(24),
                horizontal_rule(1),
                text(message.clone()).width(Length::Fill),
                row![button(tr(self.language, TextKey::Close)).on_press(Message::DismissMessage)]
                    .width(Length::Fill)
                    .align_y(alignment::Vertical::Center),
            ]
            .spacing(12),
        )
        .padding(20)
        .width(Length::Fixed(360.0))
        .style(dialog_style);

        Some(
            opaque(
                container(dialog)
                    .center(Length::Fill)
                    .style(dialog_backdrop_style),
            )
            .into(),
        )
    }

    fn records_view(&self) -> Element<'_, Message> {
        let left = column![self.realtime_panel(), self.manual_panel()]
            .spacing(12)
            .width(Length::Fixed(360.0));

        let right = column![
            row![
                text(tr(self.language, TextKey::Date)),
                text_input("YYYY-MM-DD", &self.records_date)
                    .on_input(Message::RecordsDateChanged)
                    .width(Length::Fixed(130.0)),
                button(tr(self.language, TextKey::Today)).on_press(Message::Today),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
            self.entries_panel(TextKey::TodayEntries, &self.today_entries, true),
            self.entries_panel(TextKey::RecentEntries, &self.recent_entries, false),
            self.edit_panel(),
        ]
        .spacing(12)
        .width(Length::Fill);

        row![left, right].spacing(16).into()
    }

    fn realtime_panel(&self) -> Element<'_, Message> {
        let current = if let Some(current) = &self.current_activity {
            let elapsed = Local::now().naive_local() - current.start_time;
            column![
                text(format!(
                    "{}: {}",
                    tr(self.language, TextKey::Activity),
                    current.activity
                )),
                text(format!(
                    "{}: {}",
                    tr(self.language, TextKey::Category),
                    category_label(self.language, &current.category)
                )),
                text(format!(
                    "{}: {}",
                    tr(self.language, TextKey::Start),
                    format_datetime(current.start_time)
                )),
                text(format!(
                    "{}: {}",
                    tr(self.language, TextKey::Elapsed),
                    format_duration(elapsed.num_seconds())
                ))
                .size(20),
                button(tr(self.language, TextKey::EndActivity)).on_press(Message::EndActivity),
            ]
            .spacing(6)
        } else {
            column![text(tr(self.language, TextKey::NoCurrentActivity))].spacing(6)
        };

        panel(
            tr(self.language, TextKey::RealtimeTracking),
            column![
                current,
                horizontal_rule(1),
                labeled_input(
                    tr(self.language, TextKey::ActivityName),
                    &self.realtime_form.activity,
                    Message::RealtimeActivityChanged
                ),
                category_pick_list(
                    self.language,
                    self.realtime_form.category.clone(),
                    Message::RealtimeCategoryChanged
                ),
                labeled_input(
                    tr(self.language, TextKey::Description),
                    &self.realtime_form.description,
                    Message::RealtimeDescriptionChanged
                ),
                button(tr(self.language, TextKey::StartTracking)).on_press(Message::StartActivity),
            ]
            .spacing(8),
        )
    }

    fn manual_panel(&self) -> Element<'_, Message> {
        panel(
            tr(self.language, TextKey::ManualEntry),
            entry_form_view(
                self.language,
                &self.manual_form,
                EntryFormMessages {
                    activity: Message::ManualActivityChanged,
                    category: Message::ManualCategoryChanged,
                    description: Message::ManualDescriptionChanged,
                    start_date: Message::ManualStartDateChanged,
                    start_time: Message::ManualStartTimeChanged,
                    end_date: Message::ManualEndDateChanged,
                    end_time: Message::ManualEndTimeChanged,
                },
            )
            .push(button(tr(self.language, TextKey::SaveEntry)).on_press(Message::SaveManualEntry)),
        )
    }

    fn entries_panel<'a>(
        &'a self,
        title: TextKey,
        entries: &'a [TimeEntry],
        editable: bool,
    ) -> Element<'a, Message> {
        let mut table = column![table_header(self.language, editable)].spacing(4);
        for entry in entries {
            table = table.push(entry_row(self.language, entry, editable));
        }
        panel(
            tr(self.language, title),
            scrollable(table).height(Length::Fixed(230.0)),
        )
    }

    fn edit_panel(&self) -> Element<'_, Message> {
        if let Some(form) = &self.editing_form {
            panel(
                tr(self.language, TextKey::EditEntry),
                entry_form_view(
                    self.language,
                    form,
                    EntryFormMessages {
                        activity: Message::EditActivityChanged,
                        category: Message::EditCategoryChanged,
                        description: Message::EditDescriptionChanged,
                        start_date: Message::EditStartDateChanged,
                        start_time: Message::EditStartTimeChanged,
                        end_date: Message::EditEndDateChanged,
                        end_time: Message::EditEndTimeChanged,
                    },
                )
                .push(
                    row![
                        button(tr(self.language, TextKey::SaveChanges)).on_press(Message::SaveEdit),
                        button(tr(self.language, TextKey::Cancel)).on_press(Message::CancelEdit),
                    ]
                    .spacing(8),
                ),
            )
        } else {
            container(column![]).into()
        }
    }

    fn statistics_view(&self) -> Element<'_, Message> {
        let controls = row![
            text(tr(self.language, TextKey::StatsDate)),
            text_input("YYYY-MM-DD", &self.stats_date)
                .on_input(Message::StatsDateChanged)
                .width(Length::Fixed(130.0)),
            button(StatsView::Week.label(self.language))
                .on_press(Message::ChangeStatsView(StatsView::Week)),
            button(StatsView::Month.label(self.language))
                .on_press(Message::ChangeStatsView(StatsView::Month)),
            button(StatsView::Year.label(self.language))
                .on_press(Message::ChangeStatsView(StatsView::Year)),
            text(format!(
                "{}: {}",
                tr(self.language, TextKey::Current),
                self.stats_view.label(self.language)
            )),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center);

        let Some(stats) = &self.stats else {
            return column![controls, text(tr(self.language, TextKey::NoStats))]
                .spacing(12)
                .into();
        };

        let mut detail_rows = column![table_header(self.language, false)].spacing(4);
        for entry in &stats.entries {
            detail_rows = detail_rows.push(entry_row(self.language, entry, false));
        }

        column![
            controls,
            row![
                panel(
                    tr(self.language, TextKey::CategoryDistribution),
                    stats_bars(self.language, &stats.category_hours)
                ),
                panel(
                    tr(self.language, TextKey::TimeTrend),
                    stats_bars(self.language, &stats.period_hours)
                ),
            ]
            .spacing(12),
            panel(
                tr(self.language, TextKey::Details),
                scrollable(detail_rows).height(Length::Fixed(340.0))
            ),
        ]
        .spacing(12)
        .into()
    }

    fn refresh_all(&mut self) {
        self.refresh_records();
        self.refresh_stats();
    }

    fn refresh_records(&mut self) {
        let Ok(date) = parse_date(&self.records_date) else {
            return;
        };
        if let Ok(repo) = &self.repo {
            let (start, end) = day_range(date);
            match (
                repo.list_between(start, end),
                repo.list_all_recent(100),
                repo.get_current_activity(),
            ) {
                (Ok(today_entries), Ok(recent_entries), Ok(current_activity)) => {
                    self.today_entries = today_entries;
                    self.recent_entries = recent_entries;
                    self.current_activity = current_activity;
                }
                (today_result, recent_result, current_result) => {
                    let error = today_result
                        .err()
                        .or_else(|| recent_result.err())
                        .or_else(|| current_result.err())
                        .map(|err| error_message(self.language, &err))
                        .unwrap_or_else(|| tr(self.language, TextKey::RefreshFailed).to_string());
                    self.set_error(error);
                }
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
                let mut category_hours = BTreeMap::new();
                let mut period_hours = BTreeMap::new();

                for entry in &entries {
                    *category_hours.entry(entry.category.clone()).or_insert(0.0) += entry.hours();
                    let key = if group_by_month {
                        format!(
                            "{}-{:02}",
                            entry.start_time.year(),
                            entry.start_time.month()
                        )
                    } else {
                        entry.start_time.date().format("%Y-%m-%d").to_string()
                    };
                    *period_hours.entry(key).or_insert(0.0) += entry.hours();
                }

                self.stats = Some(StatsData {
                    entries,
                    category_hours,
                    period_hours,
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

struct EntryFormMessages {
    activity: fn(String) -> Message,
    category: fn(String) -> Message,
    description: fn(String) -> Message,
    start_date: fn(String) -> Message,
    start_time: fn(String) -> Message,
    end_date: fn(String) -> Message,
    end_time: fn(String) -> Message,
}

fn entry_form_view(
    language: Language,
    form: &EntryForm,
    messages: EntryFormMessages,
) -> Column<'_, Message> {
    column![
        labeled_input(
            tr(language, TextKey::ActivityName),
            &form.activity,
            messages.activity
        ),
        category_pick_list(language, form.category.clone(), messages.category),
        row![
            labeled_input(
                tr(language, TextKey::StartDate),
                &form.start_date,
                messages.start_date
            ),
            labeled_input(
                tr(language, TextKey::StartTime),
                &form.start_time,
                messages.start_time
            ),
        ]
        .spacing(8),
        row![
            labeled_input(
                tr(language, TextKey::EndDate),
                &form.end_date,
                messages.end_date
            ),
            labeled_input(
                tr(language, TextKey::EndTime),
                &form.end_time,
                messages.end_time
            ),
        ]
        .spacing(8),
        labeled_input(
            tr(language, TextKey::Description),
            &form.description,
            messages.description
        ),
    ]
    .spacing(8)
}

fn labeled_input<'a>(
    label: &'static str,
    value: &'a str,
    on_input: fn(String) -> Message,
) -> Element<'a, Message> {
    column![
        text(label),
        text_input(label, value).on_input(on_input).padding(8),
    ]
    .spacing(4)
    .into()
}

fn category_pick_list<'a>(
    language: Language,
    selected: String,
    on_select: fn(String) -> Message,
) -> Element<'a, Message> {
    let choices: Vec<String> = CATEGORIES
        .iter()
        .map(|(zh, en)| match language {
            Language::Chinese => (*zh).to_string(),
            Language::English => (*en).to_string(),
        })
        .collect();
    let selected = category_label(language, &selected);
    column![
        text(tr(language, TextKey::Category)),
        pick_list(choices, Some(selected), on_select).padding(8),
    ]
    .spacing(4)
    .into()
}

fn panel<'a>(
    title: &'static str,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(column![text(title).size(20), horizontal_rule(1), content.into()].spacing(8))
        .padding(12)
        .width(Length::Fill)
        .into()
}

fn dialog_backdrop_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Color::from_rgba8(0, 0, 0, 0.24).into()),
        ..container::Style::default()
    }
}

fn dialog_style(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        text_color: Some(palette.background.base.text),
        background: Some(palette.background.base.color.into()),
        border: Border {
            width: 1.0,
            radius: border::radius(8),
            color: palette.danger.strong.color,
        },
        shadow: Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.28),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 24.0,
        },
    }
}

fn table_header(language: Language, editable: bool) -> Element<'static, Message> {
    let mut row = row![
        text(tr(language, TextKey::Date)).width(Length::Fixed(95.0)),
        text(tr(language, TextKey::Activity)).width(Length::Fill),
        text(tr(language, TextKey::Category)).width(Length::Fixed(70.0)),
        text(tr(language, TextKey::Start)).width(Length::Fixed(55.0)),
        text(tr(language, TextKey::End)).width(Length::Fixed(55.0)),
        text(tr(language, TextKey::Hours)).width(Length::Fixed(55.0)),
    ]
    .spacing(8);
    if editable {
        row = row.push(text(tr(language, TextKey::Actions)).width(Length::Fixed(90.0)));
    }
    row.into()
}

fn entry_row(language: Language, entry: &TimeEntry, editable: bool) -> Element<'_, Message> {
    let mut row = row![
        text(entry.start_time.date().format("%Y-%m-%d").to_string()).width(Length::Fixed(95.0)),
        text(entry.activity.clone()).width(Length::Fill),
        text(category_label(language, &entry.category)).width(Length::Fixed(70.0)),
        text(entry.start_time.format("%H:%M").to_string()).width(Length::Fixed(55.0)),
        text(entry.end_time.format("%H:%M").to_string()).width(Length::Fixed(55.0)),
        text(format!("{:.1}", entry.hours())).width(Length::Fixed(55.0)),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);

    if editable {
        row = row.push(
            Row::new()
                .push(button(tr(language, TextKey::Edit)).on_press(Message::EditEntry(entry.id)))
                .push(
                    button(tr(language, TextKey::Delete)).on_press(Message::DeleteEntry(entry.id)),
                )
                .spacing(4)
                .width(Length::Fixed(120.0)),
        );
    }

    row.into()
}

fn stats_bars(language: Language, values: &BTreeMap<String, f64>) -> Element<'_, Message> {
    if values.is_empty() {
        return text(tr(language, TextKey::NoData)).into();
    }

    let max = values.values().copied().fold(0.0, f64::max).max(1.0);
    let total: f64 = values.values().sum();
    let mut list = column!().spacing(6);

    for (name, hours) in values {
        let width = ((*hours / max) * 240.0).max(4.0) as f32;
        list = list.push(
            row![
                text(category_label(language, name)).width(Length::Fixed(90.0)),
                container(text(""))
                    .width(Length::Fixed(width))
                    .height(Length::Fixed(12.0)),
                text(format!("{hours:.1}h")).width(Length::Fixed(70.0)),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
        );
    }

    list.push(text(format!(
        "{} {total:.1} {}",
        tr(language, TextKey::Total),
        tr(language, TextKey::Hours)
    )))
    .into()
}

fn parse_date(value: &str) -> AppResult<NaiveDate> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").map_err(|_| AppError::InvalidDate)
}

fn parse_time(value: &str) -> AppResult<NaiveTime> {
    NaiveTime::parse_from_str(value.trim(), "%H:%M").map_err(|_| AppError::InvalidClockTime)
}

fn parse_datetime(date: &str, time: &str) -> AppResult<NaiveDateTime> {
    Ok(parse_date(date)?.and_time(parse_time(time)?))
}

fn day_range(date: NaiveDate) -> (NaiveDateTime, NaiveDateTime) {
    let start = date.and_time(NaiveTime::MIN);
    let end = (date + chrono::Duration::days(1)).and_time(NaiveTime::MIN);
    (start, end)
}

fn stats_range(date: NaiveDate, view: StatsView) -> (NaiveDateTime, NaiveDateTime, bool) {
    match view {
        StatsView::Week => {
            let end_date = date + chrono::Duration::days(1);
            let start_date = end_date - chrono::Duration::days(7);
            (
                start_date.and_time(NaiveTime::MIN),
                end_date.and_time(NaiveTime::MIN),
                false,
            )
        }
        StatsView::Month => {
            let start_date = NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap_or(date);
            let end_date = if date.month() == 12 {
                NaiveDate::from_ymd_opt(date.year() + 1, 1, 1).unwrap_or(start_date)
            } else {
                NaiveDate::from_ymd_opt(date.year(), date.month() + 1, 1).unwrap_or(start_date)
            };
            (
                start_date.and_time(NaiveTime::MIN),
                end_date.and_time(NaiveTime::MIN),
                false,
            )
        }
        StatsView::Year => {
            let start_date = NaiveDate::from_ymd_opt(date.year(), 1, 1).unwrap_or(date);
            let end_date = NaiveDate::from_ymd_opt(date.year() + 1, 1, 1).unwrap_or(start_date);
            (
                start_date.and_time(NaiveTime::MIN),
                end_date.and_time(NaiveTime::MIN),
                true,
            )
        }
    }
}

fn format_hm(time: NaiveTime) -> String {
    format!("{:02}:{:02}", time.hour(), time.minute())
}

fn format_datetime(time: NaiveDateTime) -> String {
    time.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn format_duration(seconds: i64) -> String {
    let seconds = seconds.max(0);
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}
