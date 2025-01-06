use crate::gui::config::Config as AppConfig;
use crate::gui::socket;
use crate::setup::thread_init;
use cosmic::app::{context_drawer, Core, Task};
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::advanced::widget::{self};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length, Padding, Subscription};
use cosmic::iced_widget::{column, row};
use cosmic::prelude::CollectionWidget;
use cosmic::theme::Container;
use cosmic::widget::button::link;
use cosmic::widget::text;
use cosmic::widget::text::{title1, title3};
use cosmic::widget::{container, icon, menu, nav_bar, scrollable, vertical_space};
use cosmic::{cosmic_theme, theme, Application, ApplicationExt, Apply, Element, Theme};
use rosu_v2::prelude::{Score, UserExtended};
use std::collections::HashMap;
use tracing::debug;

use super::socket::{Connection, Event, Message};

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
#[derive(Default)]
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: Core,
    /// Display a context drawer with the designated page if defined.
    context_page: ContextPage,
    /// Contains items assigned to the nav bar panel.
    nav: nav_bar::Model,
    /// Key bindings for the application's menu bar.
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    // Configuration data that persists between application runs.
    config: AppConfig,
    // State of the websocket connection
    state: State,
    // Latest received user data
    user_extended: Option<UserExtended>,
    user_tops: Option<Vec<Score>>,
    user_firsts: Option<Vec<Score>>,
    // First received (a.k.a. initial) user data
    initial_user_extended: Option<UserExtended>,
    initial_user_tops: Option<Vec<Score>>,
    initial_user_firsts: Option<Vec<Score>>,
}

#[derive(Default)]
pub enum State {
    #[default]
    Disconnected,
    Connected(Connection),
}
/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum AppMessage {
    OpenRepositoryUrl,
    ToggleContextPage(ContextPage),
    UpdateConfig(AppConfig),
    LaunchUrl(String),
    // StartServer(Vec<AbortHandle>),
    StartServer,
    ReceiveMessage(Event),
}

/// Create a COSMIC application from the app model
impl Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = AppMessage;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "com.chiffa.rosuTracker";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        // Create a nav bar with three page items.
        let mut nav = nav_bar::Model::default();

        nav.insert()
            .text("User")
            .data::<Page>(Page::UserPage)
            // user-available is just a guy
            .icon(icon::from_name("user-available-symbolic"))
            .activate();

        nav.insert()
            .text("Tops")
            .data::<Page>(Page::TopsPage)
            // user-bookmarks is a star in cosmic icon theme
            .icon(icon::from_name("user-bookmarks-symbolic"));

        nav.insert()
            .text("Firsts")
            .data::<Page>(Page::FirstsPage)
            // text-html looks like Earth, makes sense for leaderboards, fight me
            .icon(icon::from_name("text-html-symbolic"));

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            nav,
            key_binds: HashMap::new(),
            // Optional configuration file for an application.
            config: cosmic_config::Config::new(Self::APP_ID, AppConfig::VERSION)
                .map(|context| match AppConfig::get_entry(&context) {
                    Ok(config) => config,
                    Err((_errors, config)) => config,
                })
                .unwrap_or_default(),
            ..Default::default()
        };

        let rename = app.update_title();

        // Create a startup command that starts the socket server.
        let command = Task::perform(thread_init(), |_| {
            cosmic::app::Message::App(AppMessage::StartServer)
        });

        let batch = Task::batch([rename, command]);
        (app, batch)
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<Self::Message>> {
        let menu_bar = menu::bar(vec![menu::Tree::with_children(
            menu::root("Menu"),
            menu::items(
                &self.key_binds,
                vec![menu::Item::Button("about", None, MenuAction::About)],
            ),
        )]);

        vec![menu_bar.into()]
    }

    /// Enables the COSMIC application to create a nav bar with this model.
    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    /// Display a context drawer if the context page is requested.
    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => context_drawer::context_drawer(
                self.about(),
                AppMessage::ToggleContextPage(ContextPage::About),
            )
            .title("about"),
        })
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<Self::Message> {
        match self.nav.active_data::<Page>() {
            Some(Page::UserPage) => self.user_view(),
            Some(Page::TopsPage) => self.tops_view(),
            Some(Page::FirstsPage) => self.firsts_view(),
            None => todo!(),
        }
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-running async tasks running in the background which
    /// emit messages to the application through a channel. They are started at the
    /// beginning of the application, and persist through its lifetime.
    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch(vec![
            // Create a subscription which emits updates through a channel.
            Subscription::run(socket::connect_user).map(|x| AppMessage::ReceiveMessage(x)),
            Subscription::run(socket::connect_tops).map(|x| AppMessage::ReceiveMessage(x)),
            Subscription::run(socket::connect_firsts).map(|x| AppMessage::ReceiveMessage(x)),
            // Watch for application configuration changes.
            self.core()
                .watch_config::<AppConfig>(Self::APP_ID)
                .map(|update| {
                    for why in update.errors {
                        tracing::error!(?why, "app config error");
                    }

                    Self::Message::UpdateConfig(update.config)
                }),
        ])
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            AppMessage::OpenRepositoryUrl => {
                _ = open::that_detached("");
            }

            AppMessage::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    // Close the context drawer if the toggled context page is the same.
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    // Open the context drawer to display the requested context page.
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }
            }

            AppMessage::UpdateConfig(config) => {
                self.config = config;
            }

            AppMessage::LaunchUrl(url) => match open::that_detached(&url) {
                Ok(()) => {}
                Err(err) => {
                    eprintln!("failed to open {url:?}: {err}");
                }
            },
            AppMessage::StartServer => tracing::debug!("Iced: Started websocket server"),
            AppMessage::ReceiveMessage(event) => match event {
                Event::Connected(connection) => {
                    self.state = State::Connected(connection);
                }
                Event::Disconnected => {
                    self.state = State::Disconnected;
                }
                Event::MessageReceived(message) => match message {
                    Message::Connected => {}
                    Message::Disconnected => {}
                    Message::User(user_extended) => {
                        debug!("User received: {}", user_extended.username);
                        self.user_extended = Some(user_extended);
                    }
                    Message::Tops(vec) => {
                        debug!("Top plays received: {}", vec.len());
                        self.user_tops = Some(vec);
                    }
                    Message::Firsts(vec) => {
                        debug!("Firsts received: {}", vec.len());
                        self.user_firsts = Some(vec);
                    }
                },
            },
        }
        Task::none()
    }

    /// Called when a nav item is selected.
    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<Self::Message> {
        // Activate the page in the model.
        self.nav.activate(id);

        self.update_title()
    }

    fn on_app_exit(&mut self) -> Option<Self::Message> {
        None
    }
}

impl AppModel
where
    Self: cosmic::Application,
{
    /// The about page for this app.
    pub fn about(&self) -> Element<AppMessage> {
        let cosmic_theme::Spacing { space_xxs, .. } = theme::active().cosmic().spacing;

        let title = title3("rosu-tracker");

        let link = link("Source code")
            .on_press(AppMessage::OpenRepositoryUrl)
            .padding(0);

        cosmic::widget::column()
            // .push(icon)
            .push(title)
            .push(link)
            .align_x(Alignment::Center)
            .spacing(space_xxs)
            .into()
    }

    /// Updates the header and window titles.
    pub fn update_title(&mut self) -> Task<AppMessage> {
        let mut window_title = "rosu-tracker".to_owned();

        if let Some(page) = self.nav.text(self.nav.active()) {
            window_title.push_str(" — ");
            window_title.push_str(page);
        }

        if let Some(id) = self.core.main_window_id() {
            self.set_window_title(window_title, id)
        } else {
            Task::none()
        }
    }
    fn user_view(&self) -> Element<AppMessage> {
        let user_current = self.user_extended.as_ref();
        match user_current {
            Some(user_inner) => self.draw_user(
                user_inner,
                self.initial_user_extended.as_ref().unwrap_or(user_inner),
            ),
            None => title1("Waiting")
                .apply(container)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .into(),
        }
    }
    fn tops_view(&self) -> Element<AppMessage> {
        self.draw_scores(&self.user_tops.as_ref().unwrap_or(&vec![]))
    }
    fn firsts_view(&self) -> Element<AppMessage> {
        self.draw_scores(&self.user_firsts.as_ref().unwrap_or(&vec![]))
    }
    fn draw_scores(&self, scores: &[Score]) -> Element<AppMessage> {
        let mut score_text = scores
            .iter()
            .map(|score| self.draw_score(score))
            .collect::<Vec<_>>();
        if scores.len() == 0 {
            score_text = vec![container(cosmic::widget::text("No scores!"))];
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
    fn draw_user(&self, current: &UserExtended, initial: &UserExtended) -> Element<AppMessage> {
        let title = self.centered_username(current);
        let data = cosmic::widget::container(self.user_extended_data(current, initial));
        let children = cosmic::widget::column()
            .push(title)
            .push(data)
            .width(Length::Fill);
        container(children)
            .center_x(Length::Fill)
            .center_y(Length::Shrink)
            .into()
    }

    fn centered_username(&self, user: &UserExtended) -> Element<AppMessage> {
        let username = cosmic::widget::container(
            title1(user.username.clone().into_string()).align_x(Alignment::Center),
        )
        .align_x(Horizontal::Center)
        .center_x(Length::Fill);
        username.into()
    }

    fn user_extended_data(
        &self,
        current: &UserExtended,
        initial: &UserExtended,
    ) -> Element<AppMessage> {
        let items = cosmic::widget::column()
            .width(Length::Fill)
            .align_x(Horizontal::Center)
            .width(Length::Fill)
            .padding(20);
        let current_statistics = current.statistics.as_ref().unwrap();
        let initial_statistics = initial.statistics.as_ref().unwrap();
        let children = [
            self.make_pair::<f32>(
                "pp",
                current_statistics.pp,
                initial_statistics.pp,
                None::<fn(f32) -> String>,
            ),
            self.make_pair(
                "rank",
                current_statistics.global_rank.unwrap_or(0),
                initial_statistics.global_rank.unwrap_or(0),
                None::<fn(u32) -> String>,
            ),
            self.make_pair::<u32>(
                "country rank",
                current_statistics.country_rank.unwrap_or(0),
                initial_statistics.country_rank.unwrap_or(0),
                None::<fn(u32) -> String>,
            ),
            self.make_pair::<u32>(
                "peak rank",
                current.highest_rank.as_ref().unwrap().rank,
                initial.highest_rank.as_ref().unwrap().rank,
                None::<fn(u32) -> String>,
            ),
            self.make_pair::<f32>(
                "accuracy",
                current_statistics.accuracy,
                initial_statistics.accuracy,
                Some(format_accuracy),
            ),
            self.make_pair(
                "ranked score",
                current_statistics.ranked_score,
                initial_statistics.ranked_score,
                Some(format_number),
            ),
        ];
        let children = children.into_iter();

        let items = items.extend(children);
        items.into()
    }

    fn make_pair<'a, T>(
        &'a self,
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

    fn draw_score<'a>(&self, score: &Score) -> container::Container<'_, AppMessage, Theme> {
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
        container(col)
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
            .height(Length::Fixed(100.0))
    }
}

/// The page to display in the application.
pub enum Page {
    UserPage,
    TopsPage,
    FirstsPage,
}

/// The context page to display in the context drawer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    About,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
    About,
}

impl menu::action::MenuAction for MenuAction {
    type Message = AppMessage;

    fn message(&self) -> Self::Message {
        match self {
            MenuAction::About => AppMessage::ToggleContextPage(ContextPage::About),
        }
    }
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
