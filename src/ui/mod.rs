mod records;
mod statistics;
mod timer;

use iced::alignment;
use iced::widget::{
    button, column, container, mouse_area, opaque, pick_list, row, rule, scrollable, space, stack,
    text, text_input, tooltip, Column,
};
use iced::{border, mouse, Border, Color, Element, Length, Shadow, Theme, Vector};

use crate::app::{Message, MessageKind, MyTimeApp};
use crate::i18n::{category_label, tr, Language, TextKey, CATEGORIES};
use crate::model::{EntryForm, MainTab, TimeEntry};

pub(crate) fn main_window_view(app: &MyTimeApp) -> Element<'_, Message> {
    let mut page = column![top_bar(app)]
        .spacing(16)
        .padding(18)
        .width(Length::Fill);

    if let Some(message) = info_message_view(app) {
        page = page.push(message);
    }

    let page: Element<_> = if let Err(err) = &app.repo {
        page.push(text(err.clone()).size(18)).into()
    } else {
        let body = match app.active_tab {
            MainTab::Timer => timer::view(app),
            MainTab::Records => records::view(app),
            MainTab::Statistics => statistics::view(app),
        };
        page.push(body).into()
    };

    let base = container(
        scrollable(page)
            .style(scrollbar_style)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(page_style)
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
            text(tr(app.language, TextKey::EditEntry))
                .size(24)
                .color(title_color()),
            rule::horizontal(1),
            entry_form_view(
                app.language,
                form,
                EntryFormMessages {
                    activity: Message::EditActivityChanged,
                    category: Message::EditCategoryChanged,
                    location: Message::EditLocationChanged,
                    description: Message::EditDescriptionChanged,
                    start_date: Message::EditStartDateChanged,
                    start_time: Message::EditStartTimeChanged,
                    end_date: Message::EditEndDateChanged,
                    end_time: Message::EditEndTimeChanged,
                },
            ),
            row![
                primary_button(tr(app.language, TextKey::SaveChanges), Message::SaveEdit),
                danger_button(
                    tr(app.language, TextKey::Delete),
                    Message::DeleteEditingEntry
                ),
                secondary_button(tr(app.language, TextKey::Cancel), Message::CancelEdit),
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
        ]
        .spacing(16)
        .width(Length::Fill);

        container(
            scrollable(container(content).padding(20).style(panel_style))
                .style(scrollbar_style)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .padding(18)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(page_style)
        .into()
    } else {
        container(
            container(
                column![
                    text(tr(app.language, TextKey::NoData))
                        .color(muted_color())
                        .width(Length::Fill),
                    secondary_button(tr(app.language, TextKey::Close), Message::CancelEdit),
                ]
                .spacing(14),
            )
            .padding(20)
            .width(Length::Fill)
            .style(panel_style),
        )
        .padding(18)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(page_style)
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
    let timer = tab_button(
        tr(app.language, TextKey::Timer),
        app.active_tab == MainTab::Timer,
        Message::SwitchTab(MainTab::Timer),
    );
    let records = tab_button(
        tr(app.language, TextKey::Records),
        app.active_tab == MainTab::Records,
        Message::SwitchTab(MainTab::Records),
    );
    let statistics = tab_button(
        tr(app.language, TextKey::Statistics),
        app.active_tab == MainTab::Statistics,
        Message::SwitchTab(MainTab::Statistics),
    );

    container(
        row![
            text("MyTime").size(30).color(title_color()),
            timer,
            records,
            statistics,
            secondary_button(tr(app.language, TextKey::Refresh), Message::Refresh),
            space::horizontal(),
            secondary_button(
                tr(app.language, TextKey::LanguageSwitch),
                Message::ToggleLanguage
            ),
        ]
        .spacing(10)
        .padding([8, 10])
        .width(Length::Fill)
        .align_y(alignment::Vertical::Center),
    )
    .width(Length::Fill)
    .style(panel_style)
    .into()
}

fn info_message_view(app: &MyTimeApp) -> Option<Element<'_, Message>> {
    let Some((message, MessageKind::Info)) = &app.message else {
        return None;
    };

    Some(
        container(
            row![
                text(format!("{}: {message}", tr(app.language, TextKey::Info)))
                    .color(success_color())
                    .width(Length::Fill),
                secondary_button(tr(app.language, TextKey::Close), Message::DismissMessage),
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
        )
        .padding([10, 14])
        .width(Length::Fill)
        .style(info_message_style)
        .into(),
    )
}

fn error_dialog_view(app: &MyTimeApp) -> Option<Element<'_, Message>> {
    let Some((message, MessageKind::Error)) = &app.message else {
        return None;
    };

    let dialog = container(
        column![
            text(tr(app.language, TextKey::Error))
                .size(24)
                .color(danger_color()),
            rule::horizontal(1),
            text(message.clone()).width(Length::Fill),
            row![secondary_button(
                tr(app.language, TextKey::Close),
                Message::DismissMessage
            )]
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

pub(super) struct EntryFormMessages {
    pub(super) activity: fn(String) -> Message,
    pub(super) category: fn(String) -> Message,
    pub(super) location: fn(String) -> Message,
    pub(super) description: fn(String) -> Message,
    pub(super) start_date: fn(String) -> Message,
    pub(super) start_time: fn(String) -> Message,
    pub(super) end_date: fn(String) -> Message,
    pub(super) end_time: fn(String) -> Message,
}

pub(super) fn entry_form_view(
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
        labeled_input(
            tr(language, TextKey::Location),
            &form.location,
            messages.location
        ),
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

pub(super) fn labeled_input<'a>(
    label: &'static str,
    value: &'a str,
    on_input: fn(String) -> Message,
) -> Element<'a, Message> {
    column![
        text(label).size(13).color(muted_color()),
        styled_text_input(label, value, on_input).width(Length::Fill),
    ]
    .spacing(5)
    .width(Length::Fill)
    .into()
}

pub(super) fn category_pick_list<'a>(
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
        text(tr(language, TextKey::Category))
            .size(13)
            .color(muted_color()),
        pick_list(choices, Some(selected), on_select)
            .padding([9, 12])
            .style(pick_list_style)
            .width(Length::Fill),
    ]
    .spacing(5)
    .width(Length::Fill)
    .into()
}

pub(super) fn panel<'a>(
    title: &'static str,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        column![
            text(title).size(19).color(title_color()),
            rule::horizontal(1),
            content.into()
        ]
        .spacing(10),
    )
    .padding(16)
    .width(Length::Fill)
    .style(panel_style)
    .into()
}

pub(super) fn dialog_backdrop_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Color::from_rgba8(0, 0, 0, 0.24).into()),
        ..container::Style::default()
    }
}

pub(super) fn dialog_style(theme: &Theme) -> container::Style {
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
        ..container::Style::default()
    }
}

pub(super) fn table_header(language: Language, editable: bool) -> Element<'static, Message> {
    let mut row = row![
        header_text(tr(language, TextKey::Date)).width(Length::Fixed(95.0)),
        header_text(tr(language, TextKey::Activity)).width(Length::Fill),
        header_text(tr(language, TextKey::Category)).width(Length::Fixed(70.0)),
        header_text(tr(language, TextKey::Start)).width(Length::Fixed(55.0)),
        header_text(tr(language, TextKey::End)).width(Length::Fixed(55.0)),
        header_text(tr(language, TextKey::Minutes)).width(Length::Fixed(55.0)),
    ]
    .spacing(8)
    .padding([0, 10]);
    if editable {
        row = row.push(header_text(tr(language, TextKey::Actions)).width(Length::Fixed(55.0)));
    }
    row.into()
}

pub(super) fn entry_row(
    language: Language,
    entry: &TimeEntry,
    editable: bool,
) -> Element<'_, Message> {
    let mut row = row![
        text(entry.start_time.date().format("%Y-%m-%d").to_string())
            .color(muted_color())
            .width(Length::Fixed(95.0)),
        activity_cell(&entry.activity),
        text(category_label(language, &entry.category))
            .color(primary_color())
            .width(Length::Fixed(70.0)),
        text(entry.start_time.format("%H:%M").to_string())
            .color(muted_color())
            .width(Length::Fixed(55.0)),
        text(entry.end_time.format("%H:%M").to_string())
            .color(muted_color())
            .width(Length::Fixed(55.0)),
        text(format!("{:.1}", entry.minutes()))
            .color(title_color())
            .width(Length::Fixed(55.0)),
    ]
    .spacing(8)
    .padding([8, 10])
    .align_y(alignment::Vertical::Center);

    if editable {
        row = row.push(
            text(tr(language, TextKey::Edit))
                .color(primary_color())
                .width(Length::Fixed(55.0)),
        );
    }

    if editable {
        mouse_area(container(row).width(Length::Fill).style(table_row_style))
            .on_press(Message::EditEntry(entry.id))
            .interaction(mouse::Interaction::Pointer)
            .into()
    } else {
        container(row)
            .width(Length::Fill)
            .style(table_row_style)
            .into()
    }
}

fn activity_cell(activity: &str) -> Element<'_, Message> {
    const MAX_ACTIVITY_WIDTH: usize = 18;

    let display = truncate_with_ellipsis(activity, MAX_ACTIVITY_WIDTH);
    let activity_text = text(display)
        .color(title_color())
        .wrapping(iced::widget::text::Wrapping::None)
        .width(Length::Fill);

    container(
        tooltip(
            container(activity_text).width(Length::Fill),
            container(text(activity.to_string()).color(Color::WHITE))
                .padding([8, 10])
                .style(tooltip_style),
            tooltip::Position::FollowCursor,
        )
        .gap(8)
        .style(tooltip_style),
    )
    .width(Length::Fill)
    .into()
}

fn truncate_with_ellipsis(value: &str, max_width: usize) -> String {
    if display_width(value) <= max_width {
        return value.to_string();
    }

    let target_width = max_width.saturating_sub(3);
    let mut text = String::new();
    let mut width = 0;

    for character in value.chars() {
        let character_width = character_display_width(character);
        if width + character_width > target_width {
            break;
        }

        text.push(character);
        width += character_width;
    }

    text.push_str("...");
    text
}

fn display_width(value: &str) -> usize {
    value.chars().map(character_display_width).sum()
}

fn character_display_width(character: char) -> usize {
    if character.is_ascii() {
        1
    } else {
        2
    }
}

pub(super) fn styled_text_input<'a>(
    placeholder: &'static str,
    value: &'a str,
    on_input: fn(String) -> Message,
) -> iced::widget::TextInput<'a, Message> {
    text_input(placeholder, value)
        .on_input(on_input)
        .padding([9, 12])
        .style(text_input_style)
}

pub(super) fn primary_button<'a>(
    label: &'static str,
    message: Message,
) -> iced::widget::Button<'a, Message> {
    button(text(label).size(14))
        .padding([8, 14])
        .style(primary_button_style)
        .on_press(message)
}

pub(super) fn secondary_button<'a>(
    label: &'static str,
    message: Message,
) -> iced::widget::Button<'a, Message> {
    button(text(label).size(14))
        .padding([8, 14])
        .style(secondary_button_style)
        .on_press(message)
}

pub(super) fn danger_button<'a>(
    label: &'static str,
    message: Message,
) -> iced::widget::Button<'a, Message> {
    button(text(label).size(14))
        .padding([8, 14])
        .style(danger_button_style)
        .on_press(message)
}

pub(super) fn tab_button(
    label: &'static str,
    selected: bool,
    message: Message,
) -> iced::widget::Button<'static, Message> {
    button(text(label).size(14))
        .padding([8, 14])
        .style(move |theme, status| {
            if selected {
                primary_button_style(theme, status)
            } else {
                secondary_button_style(theme, status)
            }
        })
        .on_press(message)
}

pub(super) fn toolbar<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .padding([10, 12])
        .width(Length::Fill)
        .style(toolbar_style)
        .into()
}

fn header_text(label: &'static str) -> iced::widget::Text<'static, Theme> {
    text(label).size(13).color(muted_color())
}

fn page_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(background_color().into()),
        text_color: Some(text_color()),
        ..container::Style::default()
    }
}

pub(super) fn panel_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(surface_color().into()),
        text_color: Some(text_color()),
        border: Border {
            width: 1.0,
            radius: border::radius(8),
            color: border_color(),
        },
        shadow: soft_shadow(),
        ..container::Style::default()
    }
}

fn toolbar_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Color::from_rgb8(242, 245, 247).into()),
        text_color: Some(text_color()),
        border: Border {
            width: 1.0,
            radius: border::radius(8),
            color: border_color(),
        },
        ..container::Style::default()
    }
}

fn table_row_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Color::from_rgb8(250, 251, 252).into()),
        text_color: Some(text_color()),
        border: Border {
            width: 1.0,
            radius: border::radius(6),
            color: Color::from_rgb8(236, 240, 244),
        },
        ..container::Style::default()
    }
}

fn info_message_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Color::from_rgb8(236, 253, 245).into()),
        text_color: Some(success_color()),
        border: Border {
            width: 1.0,
            radius: border::radius(8),
            color: Color::from_rgb8(167, 243, 208),
        },
        ..container::Style::default()
    }
}

fn tooltip_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Color::from_rgb8(15, 23, 42).into()),
        text_color: Some(Color::WHITE),
        border: Border {
            width: 1.0,
            radius: border::radius(7),
            color: Color::from_rgba8(15, 23, 42, 0.92),
        },
        shadow: Shadow {
            color: Color::from_rgba8(15, 23, 42, 0.22),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 14.0,
        },
        ..container::Style::default()
    }
}

pub(super) fn bar_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(primary_color().into()),
        border: Border {
            width: 0.0,
            radius: border::radius(6),
            color: Color::TRANSPARENT,
        },
        ..container::Style::default()
    }
}

fn text_input_style(
    _theme: &Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    let border_color = match status {
        iced::widget::text_input::Status::Focused { .. } => primary_color(),
        iced::widget::text_input::Status::Hovered => Color::from_rgb8(148, 163, 184),
        iced::widget::text_input::Status::Active => border_color(),
        iced::widget::text_input::Status::Disabled => Color::from_rgb8(226, 232, 240),
    };

    iced::widget::text_input::Style {
        background: Color::WHITE.into(),
        border: Border {
            width: 1.0,
            radius: border::radius(7),
            color: border_color,
        },
        icon: muted_color(),
        placeholder: Color::from_rgb8(148, 163, 184),
        value: text_color(),
        selection: Color::from_rgba8(15, 118, 110, 0.22),
    }
}

fn pick_list_style(
    _theme: &Theme,
    status: iced::widget::pick_list::Status,
) -> iced::widget::pick_list::Style {
    let border_color = match status {
        iced::widget::pick_list::Status::Active => border_color(),
        iced::widget::pick_list::Status::Hovered
        | iced::widget::pick_list::Status::Opened { .. } => primary_color(),
    };

    iced::widget::pick_list::Style {
        text_color: text_color(),
        placeholder_color: Color::from_rgb8(148, 163, 184),
        handle_color: muted_color(),
        background: Color::WHITE.into(),
        border: Border {
            width: 1.0,
            radius: border::radius(7),
            color: border_color,
        },
    }
}

fn primary_button_style(
    _theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let background = match status {
        iced::widget::button::Status::Hovered => Color::from_rgb8(13, 148, 136),
        iced::widget::button::Status::Pressed => Color::from_rgb8(17, 94, 89),
        iced::widget::button::Status::Disabled => Color::from_rgb8(148, 163, 184),
        iced::widget::button::Status::Active => primary_color(),
    };

    iced::widget::button::Style {
        background: Some(background.into()),
        text_color: Color::WHITE,
        border: Border {
            width: 1.0,
            radius: border::radius(7),
            color: background,
        },
        shadow: button_shadow(status),
        ..iced::widget::button::Style::default()
    }
}

fn secondary_button_style(
    _theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let background = match status {
        iced::widget::button::Status::Hovered => Color::from_rgb8(241, 245, 249),
        iced::widget::button::Status::Pressed => Color::from_rgb8(226, 232, 240),
        iced::widget::button::Status::Disabled => Color::from_rgb8(248, 250, 252),
        iced::widget::button::Status::Active => Color::WHITE,
    };

    iced::widget::button::Style {
        background: Some(background.into()),
        text_color: if matches!(status, iced::widget::button::Status::Disabled) {
            Color::from_rgb8(148, 163, 184)
        } else {
            text_color()
        },
        border: Border {
            width: 1.0,
            radius: border::radius(7),
            color: border_color(),
        },
        shadow: Shadow::default(),
        ..iced::widget::button::Style::default()
    }
}

fn danger_button_style(
    _theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let background = match status {
        iced::widget::button::Status::Hovered => Color::from_rgb8(220, 38, 38),
        iced::widget::button::Status::Pressed => Color::from_rgb8(153, 27, 27),
        iced::widget::button::Status::Disabled => Color::from_rgb8(252, 165, 165),
        iced::widget::button::Status::Active => danger_color(),
    };

    iced::widget::button::Style {
        background: Some(background.into()),
        text_color: Color::WHITE,
        border: Border {
            width: 1.0,
            radius: border::radius(7),
            color: background,
        },
        shadow: button_shadow(status),
        ..iced::widget::button::Style::default()
    }
}

pub(super) fn scrollbar_style(
    _theme: &Theme,
    status: iced::widget::scrollable::Status,
) -> iced::widget::scrollable::Style {
    let active = scroll_rail(Color::from_rgb8(203, 213, 225));
    let active_hover = scroll_rail(primary_color());

    let rail = match status {
        iced::widget::scrollable::Status::Active { .. } => active,
        iced::widget::scrollable::Status::Hovered {
            is_vertical_scrollbar_hovered,
            is_horizontal_scrollbar_hovered,
            ..
        } => {
            if is_vertical_scrollbar_hovered || is_horizontal_scrollbar_hovered {
                active_hover
            } else {
                active
            }
        }
        iced::widget::scrollable::Status::Dragged { .. } => active_hover,
    };

    iced::widget::scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: scrollable_auto_scroll(),
    }
}

fn scroll_rail(color: Color) -> iced::widget::scrollable::Rail {
    iced::widget::scrollable::Rail {
        background: Some(Color::from_rgba8(226, 232, 240, 0.7).into()),
        border: Border {
            width: 0.0,
            radius: border::radius(4),
            color: Color::TRANSPARENT,
        },
        scroller: iced::widget::scrollable::Scroller {
            background: color.into(),
            border: Border {
                width: 0.0,
                radius: border::radius(4),
                color: Color::TRANSPARENT,
            },
        },
    }
}

fn scrollable_auto_scroll() -> iced::widget::scrollable::AutoScroll {
    iced::widget::scrollable::AutoScroll {
        background: Color::from_rgba8(255, 255, 255, 0.92).into(),
        border: Border {
            width: 1.0,
            radius: border::radius(u32::MAX),
            color: Color::from_rgba8(100, 116, 139, 0.35),
        },
        shadow: Shadow::default(),
        icon: muted_color(),
    }
}

fn button_shadow(status: iced::widget::button::Status) -> Shadow {
    if matches!(
        status,
        iced::widget::button::Status::Active | iced::widget::button::Status::Hovered
    ) {
        Shadow {
            color: Color::from_rgba8(15, 23, 42, 0.12),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 8.0,
        }
    } else {
        Shadow::default()
    }
}

fn soft_shadow() -> Shadow {
    Shadow {
        color: Color::from_rgba8(15, 23, 42, 0.08),
        offset: Vector::new(0.0, 4.0),
        blur_radius: 18.0,
    }
}

fn background_color() -> Color {
    Color::from_rgb8(247, 248, 250)
}

fn surface_color() -> Color {
    Color::WHITE
}

fn border_color() -> Color {
    Color::from_rgb8(226, 232, 240)
}

fn text_color() -> Color {
    Color::from_rgb8(31, 41, 55)
}

pub(super) fn title_color() -> Color {
    Color::from_rgb8(15, 23, 42)
}

pub(super) fn muted_color() -> Color {
    Color::from_rgb8(100, 116, 139)
}

pub(super) fn primary_color() -> Color {
    Color::from_rgb8(15, 118, 110)
}

fn success_color() -> Color {
    Color::from_rgb8(22, 101, 52)
}

fn danger_color() -> Color {
    Color::from_rgb8(185, 28, 28)
}
