use iced::alignment;
use iced::widget::{column, container, opaque, row, rule, scrollable, space, stack, text};
use iced::{Element, Length};

use crate::app::{Message, MyTimeApp};
use crate::i18n::{tr, Language, TextKey};
use crate::model::TimeEntry;

use super::{
    dialog_backdrop_style, dialog_style, entry_form_view, entry_row, muted_color, panel_style,
    primary_button, scrollbar_style, secondary_button, styled_text_input, table_header, toolbar,
    EntryFormMessages,
};

pub(super) fn view(app: &MyTimeApp) -> Element<'_, Message> {
    let content: Element<_> = column![
        toolbar(
            row![
                text(tr(app.language, TextKey::Date)).color(muted_color()),
                styled_text_input("YYYY-MM-DD", &app.records_date, Message::RecordsDateChanged)
                    .width(Length::Fixed(136.0)),
                secondary_button(tr(app.language, TextKey::Today), Message::Today),
                space::horizontal(),
                primary_button(
                    tr(app.language, TextKey::AddEntry),
                    Message::OpenManualEntry
                ),
            ]
            .spacing(10)
            .width(Length::Fill)
            .align_y(alignment::Vertical::Center),
        ),
        entries_panel(app.language, &app.today_entries, true),
    ]
    .spacing(14)
    .width(Length::Fill)
    .into();

    if app.manual_entry_dialog_open {
        stack![content, manual_entry_dialog(app)]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        content
    }
}

fn manual_entry_dialog(app: &MyTimeApp) -> Element<'_, Message> {
    let dialog = container(
        column![
            text(tr(app.language, TextKey::ManualEntry)).size(24),
            rule::horizontal(1),
            entry_form_view(
                app.language,
                &app.manual_form,
                EntryFormMessages {
                    activity: Message::ManualActivityChanged,
                    category: Message::ManualCategoryChanged,
                    location: Message::ManualLocationChanged,
                    description: Message::ManualDescriptionChanged,
                    start_date: Message::ManualStartDateChanged,
                    start_time: Message::ManualStartTimeChanged,
                    end_date: Message::ManualEndDateChanged,
                    end_time: Message::ManualEndTimeChanged,
                },
            ),
            row![
                primary_button(
                    tr(app.language, TextKey::SaveEntry),
                    Message::SaveManualEntry
                ),
                secondary_button(tr(app.language, TextKey::Cancel), Message::CloseManualEntry),
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
        ]
        .spacing(14),
    )
    .padding(20)
    .width(Length::Fixed(460.0))
    .style(dialog_style);

    opaque(
        container(dialog)
            .center(Length::Fill)
            .style(dialog_backdrop_style),
    )
    .into()
}

fn entries_panel<'a>(
    language: Language,
    entries: &'a [TimeEntry],
    editable: bool,
) -> Element<'a, Message> {
    let mut table = column![table_header(language, editable)].spacing(6);
    for entry in entries {
        table = table.push(entry_row(language, entry, editable));
    }
    container(
        scrollable(table)
            .style(scrollbar_style)
            .height(Length::Fixed(520.0)),
    )
    .padding(16)
    .width(Length::Fill)
    .style(panel_style)
    .into()
}
