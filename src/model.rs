use std::collections::BTreeMap;

use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use thiserror::Error;

use crate::i18n::{tr, Language, TextKey, CATEGORIES};

pub(crate) type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub(crate) enum AppError {
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

#[derive(Clone, Debug)]
pub(crate) struct TimeEntry {
    pub(crate) id: i64,
    pub(crate) start_time: NaiveDateTime,
    pub(crate) end_time: NaiveDateTime,
    pub(crate) activity: String,
    pub(crate) category: String,
    pub(crate) location: String,
    pub(crate) description: String,
}

impl TimeEntry {
    pub(crate) fn minutes(&self) -> f64 {
        (self.end_time - self.start_time).num_seconds() as f64 / 60.0
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CurrentActivity {
    pub(crate) start_time: NaiveDateTime,
    pub(crate) activity: String,
    pub(crate) category: String,
    pub(crate) location: String,
    pub(crate) description: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainTab {
    Timer,
    Records,
    Statistics,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StatsView {
    Week,
    Month,
    Year,
}

impl StatsView {
    pub(crate) fn label(self, language: Language) -> &'static str {
        match self {
            Self::Week => tr(language, TextKey::Week),
            Self::Month => tr(language, TextKey::Month),
            Self::Year => tr(language, TextKey::Year),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct EntryForm {
    pub(crate) id: Option<i64>,
    pub(crate) start_date: String,
    pub(crate) start_time: String,
    pub(crate) end_date: String,
    pub(crate) end_time: String,
    pub(crate) activity: String,
    pub(crate) category: String,
    pub(crate) location: String,
    pub(crate) description: String,
}

impl EntryForm {
    pub(crate) fn new_default() -> Self {
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
            location: String::new(),
            description: String::new(),
        }
    }

    pub(crate) fn from_entry(entry: &TimeEntry) -> Self {
        Self {
            id: Some(entry.id),
            start_date: entry.start_time.date().format("%Y-%m-%d").to_string(),
            start_time: format_hm(entry.start_time.time()),
            end_date: entry.end_time.date().format("%Y-%m-%d").to_string(),
            end_time: format_hm(entry.end_time.time()),
            activity: entry.activity.clone(),
            category: entry.category.clone(),
            location: entry.location.clone(),
            description: entry.description.clone(),
        }
    }

    pub(crate) fn to_entry(&self) -> AppResult<TimeEntry> {
        Ok(TimeEntry {
            id: self.id.unwrap_or_default(),
            start_time: parse_datetime(&self.start_date, &self.start_time)?,
            end_time: parse_datetime(&self.end_date, &self.end_time)?,
            activity: self.activity.clone(),
            category: self.category.clone(),
            location: self.location.clone(),
            description: self.description.clone(),
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ActivityForm {
    pub(crate) activity: String,
    pub(crate) category: String,
    pub(crate) location: String,
    pub(crate) description: String,
}

impl ActivityForm {
    pub(crate) fn new() -> Self {
        Self {
            activity: String::new(),
            category: CATEGORIES[0].0.to_string(),
            location: String::new(),
            description: String::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct StatsData {
    pub(crate) entries: Vec<TimeEntry>,
    pub(crate) category_minutes: BTreeMap<String, f64>,
    pub(crate) period_minutes: BTreeMap<String, f64>,
}

pub(crate) fn error_message(language: Language, error: &AppError) -> String {
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

pub(crate) fn validate_text(activity: &str, category: &str) -> AppResult<()> {
    if activity.trim().is_empty() {
        return Err(AppError::EmptyActivity);
    }
    if category.trim().is_empty() {
        return Err(AppError::EmptyCategory);
    }
    Ok(())
}

pub(crate) fn validate_entry(
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

pub(crate) fn parse_date(value: &str) -> AppResult<NaiveDate> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").map_err(|_| AppError::InvalidDate)
}

fn parse_time(value: &str) -> AppResult<NaiveTime> {
    NaiveTime::parse_from_str(value.trim(), "%H:%M").map_err(|_| AppError::InvalidClockTime)
}

fn parse_datetime(date: &str, time: &str) -> AppResult<NaiveDateTime> {
    Ok(parse_date(date)?.and_time(parse_time(time)?))
}

pub(crate) fn day_range(date: NaiveDate) -> (NaiveDateTime, NaiveDateTime) {
    let start = date.and_time(NaiveTime::MIN);
    let end = (date + chrono::Duration::days(1)).and_time(NaiveTime::MIN);
    (start, end)
}

pub(crate) fn stats_range(
    date: NaiveDate,
    view: StatsView,
) -> (NaiveDateTime, NaiveDateTime, bool) {
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

pub(crate) fn format_datetime(time: NaiveDateTime) -> String {
    time.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub(crate) fn format_duration_minutes(seconds: i64) -> String {
    let minutes = seconds.max(0) as f64 / 60.0;
    format!("{minutes:.1}")
}
