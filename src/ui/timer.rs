use iced::widget::{column, container, rule, text};
use iced::{Element, Length};

use crate::app::{Message, MyTimeApp};
use crate::i18n::{category_label, tr, TextKey};
use crate::model::{format_datetime, format_duration_minutes};

use super::{
    category_pick_list, danger_button, labeled_input, muted_color, panel, primary_button,
    primary_color,
};

pub(super) fn view(app: &MyTimeApp) -> Element<'_, Message> {
    container(realtime_panel(app)).width(Length::Fill).into()
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
                "{}: {} {}",
                tr(app.language, TextKey::Elapsed),
                format_duration_minutes(elapsed.num_seconds()),
                tr(app.language, TextKey::Minutes)
            ))
            .size(26)
            .color(primary_color()),
            danger_button(tr(app.language, TextKey::EndActivity), Message::EndActivity),
        ]
        .spacing(8)
    } else {
        column![text(tr(app.language, TextKey::NoCurrentActivity)).color(muted_color())].spacing(8)
    };

    panel(
        tr(app.language, TextKey::RealtimeTracking),
        column![
            current,
            rule::horizontal(1),
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
                tr(app.language, TextKey::Location),
                &app.realtime_form.location,
                Message::RealtimeLocationChanged
            ),
            labeled_input(
                tr(app.language, TextKey::Description),
                &app.realtime_form.description,
                Message::RealtimeDescriptionChanged
            ),
            primary_button(
                tr(app.language, TextKey::StartTracking),
                Message::StartActivity
            ),
        ]
        .spacing(10),
    )
}
