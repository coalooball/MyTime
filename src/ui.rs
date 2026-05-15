use iced::alignment;
use iced::widget::{
    button, column, container, horizontal_rule, horizontal_space, mouse_area, opaque, pick_list,
    row, scrollable, stack, text, text_input, Column,
};
use iced::{border, mouse, Border, Color, Element, Length, Shadow, Theme, Vector};

use crate::app::{Message, MessageKind, MyTimeApp};
use crate::i18n::{category_label, tr, Language, TextKey, CATEGORIES};
use crate::model::{format_datetime, format_duration, EntryForm, MainTab, StatsView, TimeEntry};

pub(crate) fn main_window_view(app: &MyTimeApp) -> Element<'_, Message> {
    let mut page = column![top_bar(app)].spacing(12).padding(16);

    if let Some(message) = info_message_view(app) {
        page = page.push(message);
    }

    let page: Element<_> = if let Err(err) = &app.repo {
        page.push(text(err.clone()).size(18)).into()
    } else {
        let body = match app.active_tab {
            MainTab::Records => records_view(app),
            MainTab::Statistics => statistics_view(app),
        };
        page.push(body).into()
    };

    let base = scrollable(page)
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

    if let Some(dialog) = error_dialog_view(app) {
        stack![base, dialog]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        base
    }
}

pub(crate) fn edit_window_view(app: &MyTimeApp) -> Element<'_, Message> {
    let base: Element<_> = if let Some(form) = &app.editing_form {
        let content = column![
            text(tr(app.language, TextKey::EditEntry)).size(24),
            horizontal_rule(1),
            entry_form_view(
                app.language,
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
            ),
            row![
                button(tr(app.language, TextKey::SaveChanges)).on_press(Message::SaveEdit),
                button(tr(app.language, TextKey::Delete))
                    .on_press(Message::DeleteEditingEntry)
                    .style(iced::widget::button::danger),
                button(tr(app.language, TextKey::Cancel)).on_press(Message::CancelEdit),
            ]
            .spacing(8),
        ]
        .spacing(12);

        container(scrollable(content).width(Length::Fill).height(Length::Fill))
            .padding(16)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        container(
            column![
                text(tr(app.language, TextKey::NoData)).width(Length::Fill),
                button(tr(app.language, TextKey::Close)).on_press(Message::CancelEdit),
            ]
            .spacing(12),
        )
        .padding(16)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    };

    if let Some(dialog) = error_dialog_view(app) {
        stack![base, dialog]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        base
    }
}

fn top_bar(app: &MyTimeApp) -> Element<'_, Message> {
    row![
        text("MyTime").size(32),
        button(tr(app.language, TextKey::Records)).on_press(Message::SwitchTab(MainTab::Records)),
        button(tr(app.language, TextKey::Statistics))
            .on_press(Message::SwitchTab(MainTab::Statistics)),
        button(tr(app.language, TextKey::Refresh)).on_press(Message::Refresh),
        horizontal_space(),
        button(tr(app.language, TextKey::LanguageSwitch)).on_press(Message::ToggleLanguage),
    ]
    .spacing(10)
    .width(Length::Fill)
    .align_y(alignment::Vertical::Center)
    .into()
}

fn info_message_view(app: &MyTimeApp) -> Option<Element<'_, Message>> {
    let Some((message, MessageKind::Info)) = &app.message else {
        return None;
    };

    Some(
        container(
            row![
                text(format!("{}: {message}", tr(app.language, TextKey::Info))).width(Length::Fill),
                button(tr(app.language, TextKey::Close)).on_press(Message::DismissMessage),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
        )
        .padding(10)
        .width(Length::Fill)
        .into(),
    )
}

fn error_dialog_view(app: &MyTimeApp) -> Option<Element<'_, Message>> {
    let Some((message, MessageKind::Error)) = &app.message else {
        return None;
    };

    let dialog = container(
        column![
            text(tr(app.language, TextKey::Error)).size(24),
            horizontal_rule(1),
            text(message.clone()).width(Length::Fill),
            row![button(tr(app.language, TextKey::Close)).on_press(Message::DismissMessage)]
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

fn records_view(app: &MyTimeApp) -> Element<'_, Message> {
    let left = column![realtime_panel(app), manual_panel(app)]
        .spacing(12)
        .width(Length::Fixed(360.0));

    let right = column![
        row![
            text(tr(app.language, TextKey::Date)),
            text_input("YYYY-MM-DD", &app.records_date)
                .on_input(Message::RecordsDateChanged)
                .width(Length::Fixed(130.0)),
            button(tr(app.language, TextKey::Today)).on_press(Message::Today),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
        entries_panel(
            app.language,
            TextKey::TodayEntries,
            &app.today_entries,
            true
        ),
        entries_panel(
            app.language,
            TextKey::RecentEntries,
            &app.recent_entries,
            true
        ),
    ]
    .spacing(12)
    .width(Length::Fill);

    row![left, right].spacing(16).into()
}

fn realtime_panel(app: &MyTimeApp) -> Element<'_, Message> {
    let current = if let Some(current) = &app.current_activity {
        let elapsed = chrono::Local::now().naive_local() - current.start_time;
        column![
            text(format!(
                "{}: {}",
                tr(app.language, TextKey::Activity),
                current.activity
            )),
            text(format!(
                "{}: {}",
                tr(app.language, TextKey::Category),
                category_label(app.language, &current.category)
            )),
            text(format!(
                "{}: {}",
                tr(app.language, TextKey::Start),
                format_datetime(current.start_time)
            )),
            text(format!(
                "{}: {}",
                tr(app.language, TextKey::Elapsed),
                format_duration(elapsed.num_seconds())
            ))
            .size(20),
            button(tr(app.language, TextKey::EndActivity)).on_press(Message::EndActivity),
        ]
        .spacing(6)
    } else {
        column![text(tr(app.language, TextKey::NoCurrentActivity))].spacing(6)
    };

    panel(
        tr(app.language, TextKey::RealtimeTracking),
        column![
            current,
            horizontal_rule(1),
            labeled_input(
                tr(app.language, TextKey::ActivityName),
                &app.realtime_form.activity,
                Message::RealtimeActivityChanged
            ),
            category_pick_list(
                app.language,
                app.realtime_form.category.clone(),
                Message::RealtimeCategoryChanged
            ),
            labeled_input(
                tr(app.language, TextKey::Description),
                &app.realtime_form.description,
                Message::RealtimeDescriptionChanged
            ),
            button(tr(app.language, TextKey::StartTracking)).on_press(Message::StartActivity),
        ]
        .spacing(8),
    )
}

fn manual_panel(app: &MyTimeApp) -> Element<'_, Message> {
    panel(
        tr(app.language, TextKey::ManualEntry),
        entry_form_view(
            app.language,
            &app.manual_form,
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
        .push(button(tr(app.language, TextKey::SaveEntry)).on_press(Message::SaveManualEntry)),
    )
}

fn entries_panel<'a>(
    language: Language,
    title: TextKey,
    entries: &'a [TimeEntry],
    editable: bool,
) -> Element<'a, Message> {
    let mut table = column![table_header(language, editable)].spacing(4);
    for entry in entries {
        table = table.push(entry_row(language, entry, editable));
    }
    panel(
        tr(language, title),
        scrollable(table).height(Length::Fixed(230.0)),
    )
}

fn statistics_view(app: &MyTimeApp) -> Element<'_, Message> {
    let controls = row![
        text(tr(app.language, TextKey::StatsDate)),
        text_input("YYYY-MM-DD", &app.stats_date)
            .on_input(Message::StatsDateChanged)
            .width(Length::Fixed(130.0)),
        button(StatsView::Week.label(app.language))
            .on_press(Message::ChangeStatsView(StatsView::Week)),
        button(StatsView::Month.label(app.language))
            .on_press(Message::ChangeStatsView(StatsView::Month)),
        button(StatsView::Year.label(app.language))
            .on_press(Message::ChangeStatsView(StatsView::Year)),
        text(format!(
            "{}: {}",
            tr(app.language, TextKey::Current),
            app.stats_view.label(app.language)
        )),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center);

    let Some(stats) = &app.stats else {
        return column![controls, text(tr(app.language, TextKey::NoStats))]
            .spacing(12)
            .into();
    };

    let mut detail_rows = column![table_header(app.language, false)].spacing(4);
    for entry in &stats.entries {
        detail_rows = detail_rows.push(entry_row(app.language, entry, false));
    }

    column![
        controls,
        row![
            panel(
                tr(app.language, TextKey::CategoryDistribution),
                stats_bars(app.language, &stats.category_hours)
            ),
            panel(
                tr(app.language, TextKey::TimeTrend),
                stats_bars(app.language, &stats.period_hours)
            ),
        ]
        .spacing(12),
        panel(
            tr(app.language, TextKey::Details),
            scrollable(detail_rows).height(Length::Fixed(340.0))
        ),
    ]
    .spacing(12)
    .into()
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
            container(labeled_input(
                tr(language, TextKey::StartDate),
                &form.start_date,
                messages.start_date
            ))
            .width(Length::FillPortion(1)),
            container(labeled_input(
                tr(language, TextKey::StartTime),
                &form.start_time,
                messages.start_time
            ))
            .width(Length::FillPortion(1)),
        ]
        .spacing(8)
        .width(Length::Fill),
        row![
            container(labeled_input(
                tr(language, TextKey::EndDate),
                &form.end_date,
                messages.end_date
            ))
            .width(Length::FillPortion(1)),
            container(labeled_input(
                tr(language, TextKey::EndTime),
                &form.end_time,
                messages.end_time
            ))
            .width(Length::FillPortion(1)),
        ]
        .spacing(8)
        .width(Length::Fill),
        labeled_input(
            tr(language, TextKey::Description),
            &form.description,
            messages.description
        ),
    ]
    .spacing(8)
    .width(Length::Fill)
}

fn labeled_input<'a>(
    label: &'static str,
    value: &'a str,
    on_input: fn(String) -> Message,
) -> Element<'a, Message> {
    column![
        text(label),
        text_input(label, value)
            .on_input(on_input)
            .padding(8)
            .width(Length::Fill),
    ]
    .spacing(4)
    .width(Length::Fill)
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
        pick_list(choices, Some(selected), on_select)
            .padding(8)
            .width(Length::Fill),
    ]
    .spacing(4)
    .width(Length::Fill)
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
        row = row.push(text(tr(language, TextKey::Actions)).width(Length::Fixed(55.0)));
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
        row = row.push(text(tr(language, TextKey::Edit)).width(Length::Fixed(55.0)));
    }

    if editable {
        mouse_area(container(row).padding([4, 0]).width(Length::Fill))
            .on_press(Message::EditEntry(entry.id))
            .interaction(mouse::Interaction::Pointer)
            .into()
    } else {
        row.into()
    }
}

fn stats_bars(
    language: Language,
    values: &std::collections::BTreeMap<String, f64>,
) -> Element<'_, Message> {
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
