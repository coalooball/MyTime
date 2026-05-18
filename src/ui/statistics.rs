use iced::alignment;
use iced::widget::{column, container, row, scrollable, text};
use iced::{Element, Length};

use crate::app::{Message, MyTimeApp};
use crate::i18n::{category_label, tr, Language, TextKey};
use crate::model::StatsView;

use super::{
    bar_style, entry_row, muted_color, panel, panel_style, scrollbar_style, styled_text_input,
    tab_button, table_header, title_color, toolbar,
};

pub(super) fn view(app: &MyTimeApp) -> Element<'_, Message> {
    let controls = toolbar(
        row![
            text(tr(app.language, TextKey::StatsDate)).color(muted_color()),
            styled_text_input("YYYY-MM-DD", &app.stats_date, Message::StatsDateChanged)
                .width(Length::Fixed(136.0)),
            tab_button(
                StatsView::Week.label(app.language),
                app.stats_view == StatsView::Week,
                Message::ChangeStatsView(StatsView::Week)
            ),
            tab_button(
                StatsView::Month.label(app.language),
                app.stats_view == StatsView::Month,
                Message::ChangeStatsView(StatsView::Month)
            ),
            tab_button(
                StatsView::Year.label(app.language),
                app.stats_view == StatsView::Year,
                Message::ChangeStatsView(StatsView::Year)
            ),
            text(format!(
                "{}: {}",
                tr(app.language, TextKey::Current),
                app.stats_view.label(app.language)
            ))
            .color(muted_color()),
        ]
        .spacing(10)
        .align_y(alignment::Vertical::Center),
    );

    let Some(stats) = &app.stats else {
        return column![
            controls,
            container(text(tr(app.language, TextKey::NoStats)).color(muted_color()))
                .padding(18)
                .width(Length::Fill)
                .style(panel_style)
        ]
        .spacing(14)
        .into();
    };

    let mut detail_rows = column![table_header(app.language, false)].spacing(6);
    for entry in &stats.entries {
        detail_rows = detail_rows.push(entry_row(app.language, entry, false));
    }

    column![
        controls,
        row![
            panel(
                tr(app.language, TextKey::CategoryDistribution),
                stats_bars(app.language, &stats.category_minutes)
            ),
            panel(
                tr(app.language, TextKey::TimeTrend),
                stats_bars(app.language, &stats.period_minutes)
            ),
        ]
        .spacing(14),
        panel(
            tr(app.language, TextKey::Details),
            scrollable(detail_rows)
                .style(scrollbar_style)
                .height(Length::Fixed(340.0))
        ),
    ]
    .spacing(14)
    .into()
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

    for (name, minutes) in values {
        let width = ((*minutes / max) * 240.0).max(6.0) as f32;
        list = list.push(
            row![
                text(category_label(language, name))
                    .color(title_color())
                    .width(Length::Fixed(90.0)),
                container(text(""))
                    .width(Length::Fixed(width))
                    .height(Length::Fixed(12.0))
                    .style(bar_style),
                text(format_minutes_hours(*minutes))
                    .color(muted_color())
                    .width(Length::Fixed(70.0)),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
        );
    }

    list.push(
        text(format!(
            "{} {}",
            tr(language, TextKey::Total),
            format_minutes_hours(total)
        ))
        .color(title_color()),
    )
    .into()
}

fn format_minutes_hours(minutes: f64) -> String {
    let total_minutes = minutes.round().max(0.0) as i64;
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;

    if hours > 0 {
        format!("{hours}h{minutes}m")
    } else {
        format!("{minutes}m")
    }
}
