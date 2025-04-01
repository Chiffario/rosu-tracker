use ::image::{DynamicImage, GenericImageView};
use cosmic::iced::advanced::widget::{self};
use cosmic::iced::alignment::Horizontal;
use cosmic::iced::wgpu::naga::FastHashMap;
use cosmic::iced::{Alignment, Background, Length, Padding};
use cosmic::iced_widget::image::Handle;
use cosmic::iced_widget::{column, row, stack};
use cosmic::prelude::CollectionWidget;
use cosmic::theme::Container;
use cosmic::widget::text::{title1, title3};
use cosmic::widget::{Image, container, scrollable, vertical_space};
use cosmic::widget::{image, text};
use cosmic::{Element, Theme, theme};
use rosu_v2::prelude::{Score, UserExtended};

use super::app::AppMessage;

pub fn draw_scores<'a>(
    scores: &'a [Score],
    background: &'a FastHashMap<u32, Option<DynamicImage>>,
) -> Element<'a, AppMessage> {
    let mut score_text = scores
        .iter()
        .map(|score| {
            let bg = background
                .get(&score.mapset.as_ref().unwrap().mapset_id)
                .unwrap_or(&None);
            draw_score(score, bg)
        })
        .collect::<Vec<_>>();
    if scores.is_empty() {
        score_text = vec![stack!(cosmic::widget::text("No scores!"))];
    }
    scrollable(
        cosmic::widget::column()
            .spacing(20)
            .append(&mut score_text)
            .width(Length::Fill)
            .max_width(800)
            .padding(Padding {
                top: 0.0,
                right: 20.0,
                bottom: 0.0,
                left: 10.0,
            }),
    )
    .into()
}
fn draw_score<'a>(
    score: &'a Score,
    background: &'a Option<DynamicImage>,
) -> cosmic::iced_widget::Stack<'a, AppMessage, Theme> {
    let mapset = score.mapset.as_ref().unwrap();
    let map = score.map.as_ref().unwrap();
    let title_diff: Element<AppMessage> = cosmic::widget::button::custom(
        text(format!("{} [{}]", mapset.title.clone(), &map.version))
            .wrapping(widget::text::Wrapping::None),
    )
    .class(theme::Button::Link)
    .on_press(AppMessage::LaunchUrl(format!(
        "https://osu.ppy.sh/scores/{}",
        score.id
    )))
    // TODO: Change to Length::Shrink and add horizontal_space()
    .width(Length::Fill)
    .padding(0)
    .into();
    let artist = text(mapset.artist.clone()).height(Length::Fill);
    let date = text(score.ended_at.date().to_string());
    let pp = text(format!(
        "{} pp",
        score.pp.unwrap_or_default().trunc() as u32
    ))
    .height(Length::Fill);
    let combo = text(format!("{} combo", score.max_combo)).height(Length::Fill);
    let spacing = vertical_space();
    let col = row![
        column![title_diff, artist, spacing, combo]
            .padding(10)
            .width(Length::FillPortion(2))
            .height(Length::Fill),
        column![pp, date]
            .padding(10)
            .width(Length::FillPortion(1))
            .height(Length::Fill)
    ];
    let bg: Option<Image> = background
        .clone()
        .map(<DynamicImage>::into_bytes)
        .map(Handle::from_bytes)
        .map(Image::new);

    let card = container(col)
        .class(Container::custom(|theme| {
            let cosmic = theme.cosmic();
            let corners = cosmic.corner_radii;
            container::Style {
                text_color: Some(cosmic.background.on.into()),
                background: Some(
                    cosmic::iced::Color::from(cosmic.background.component.base).into(),
                ),
                border: cosmic::iced::Border {
                    radius: corners.radius_m.into(),
                    width: 1.0,
                    color: cosmic.background.divider.into(),
                },
                shadow: cosmic::iced::Shadow::default(),
                icon_color: Some(cosmic.background.on.into()),
            }
        }))
        .width(Length::Fill)
        .height(Length::Fixed(100.0));
    let bg = cosmic::iced_widget::Stack::new().push_maybe(bg).push(card);
    bg.into()
}
/// LIFETIME: Realistically - shares one with `App`
pub(crate) fn draw_user<'u>(
    current: &'u UserExtended,
    initial: &'u UserExtended,
) -> Element<'u, AppMessage> {
    let title = centered_username(current);
    let data = cosmic::widget::container(user_extended_data(current, initial));
    let children = cosmic::widget::column()
        .push(title)
        .push(data)
        .width(Length::Fill);
    container(children)
        .center_x(Length::Fill)
        .center_y(Length::Shrink)
        .into()
}

fn centered_username(user: &UserExtended) -> Element<AppMessage> {
    let username = cosmic::widget::container(
        title1(user.username.clone().into_string()).align_x(Alignment::Center),
    )
    .align_x(Horizontal::Center)
    .center_x(Length::Fill);
    username.into()
}

/// Widget for displaying a tri-column of user data
/// LIFETIME: Realistically - shares one with `App`
fn user_extended_data<'u>(
    current: &'u UserExtended,
    initial: &'u UserExtended,
) -> Element<'u, AppMessage> {
    let items = cosmic::widget::column()
        .width(Length::Fill)
        .align_x(Horizontal::Center)
        .width(Length::Fill)
        .padding(20);
    let current_statistics = current.statistics.as_ref().unwrap();
    let initial_statistics = initial.statistics.as_ref().unwrap();
    let children = [
        make_pair::<f32>(
            "pp",
            current_statistics.pp,
            initial_statistics.pp,
            None::<fn(f32) -> String>,
        ),
        make_pair(
            "rank",
            current_statistics.global_rank.unwrap_or(0),
            initial_statistics.global_rank.unwrap_or(0),
            None::<fn(u32) -> String>,
        ),
        make_pair::<u32>(
            "country rank",
            current_statistics.country_rank.unwrap_or(0),
            initial_statistics.country_rank.unwrap_or(0),
            None::<fn(u32) -> String>,
        ),
        make_pair::<u32>(
            "peak rank",
            current.highest_rank.as_ref().unwrap().rank,
            initial.highest_rank.as_ref().unwrap().rank,
            None::<fn(u32) -> String>,
        ),
        make_pair::<f32>(
            "accuracy",
            current_statistics.accuracy,
            initial_statistics.accuracy,
            Some(format_accuracy),
        ),
        make_pair(
            "ranked score",
            current_statistics.ranked_score,
            initial_statistics.ranked_score,
            Some(format_number),
        ),
        make_pair(
            "A ranks",
            current_statistics.grade_counts.a,
            initial_statistics.grade_counts.a,
            None::<fn(i32) -> String>,
        ),
        make_pair(
            "S ranks",
            current_statistics.grade_counts.s,
            initial_statistics.grade_counts.s,
            None::<fn(i32) -> String>,
        ),
        make_pair(
            "SS ranks",
            current_statistics.grade_counts.ss,
            initial_statistics.grade_counts.ss,
            None::<fn(i32) -> String>,
        ),
        make_pair(
            "SH ranks",
            current_statistics.grade_counts.sh,
            initial_statistics.grade_counts.sh,
            None::<fn(i32) -> String>,
        ),
        make_pair(
            "SSH ranks",
            current_statistics.grade_counts.ssh,
            initial_statistics.grade_counts.ssh,
            None::<fn(i32) -> String>,
        ),
    ];
    let children = children.into_iter();

    let items = items.extend(children);
    items.into()
}

fn make_pair<'a, T>(
    title: &'a str,
    current: T,
    initial: T,
    fmt: Option<impl Fn(T) -> String>,
) -> Element<'a, AppMessage>
where
    T: std::ops::Sub<Output = T> + Copy + std::fmt::Display,
{
    let current_string = match fmt.as_ref() {
        Some(f) => f(current),
        None => current.to_string(),
    };
    let delta_string = match fmt {
        Some(f) => {
            let tmp = current - initial;
            f(tmp)
        }
        None => format!("{}", current - initial),
    };
    container(
        row![
            widget::text::Text::new(title)
                .align_x(Horizontal::Left)
                .size(16)
                .width(Length::FillPortion(1)),
            // cosmic::widget::divider::vertical::default(),
            widget::text::Text::new(current_string)
                .align_x(Horizontal::Right)
                .size(16)
                .width(Length::FillPortion(1)),
            cosmic::widget::text(delta_string)
                .align_x(Horizontal::Right)
                .size(16)
                .width(Length::FillPortion(1))
        ]
        .width(Length::Fill)
        // .spacing(20)
        .height(Length::Shrink),
    )
    .center_x(Length::Fill)
    .center_y(Length::Shrink)
    .into()
}
fn format_number(int: impl Into<u64>) -> String {
    let num = int
        .into()
        .to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(std::str::from_utf8)
        .collect::<Result<Vec<&str>, _>>()
        .unwrap()
        .join(",");
    num
}
fn format_accuracy(acc: f32) -> String {
    format!("{:.2}%", acc)
}
