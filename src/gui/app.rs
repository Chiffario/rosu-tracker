use crate::gui::config::Config as AppConfig;
use crate::gui::socket;
use crate::setup::thread_init;
use cosmic::app::{context_drawer, Core, Task};
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::advanced::widget::{self};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length, Subscription};
use cosmic::iced_widget::row;
use cosmic::widget::button::link;
use cosmic::widget::text::{title1, title3};
use cosmic::widget::{container, icon, menu, nav_bar};
use cosmic::{cosmic_theme, theme, Application, ApplicationExt, Apply, Element};
use rosu_v2::prelude::{Score, UserExtended};
use std::collections::HashMap;

use super::socket::{Connection, Event, Message};

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
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
}

pub enum State {
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
            .data::<Page>(Page::UserPage("Balls".to_owned()))
            .icon(icon::from_name("applications-science-symbolic"))
            .activate();

        nav.insert()
            .text("Tops")
            .data::<Page>(Page::TopsPage("Dicks".to_owned()))
            .icon(icon::from_name("applications-system-symbolic"));

        nav.insert()
            .text("Firsts")
            .data::<Page>(Page::FirstsPage("Boobs".to_owned()))
            .icon(icon::from_name("applications-games-symbolic"));

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
                    Err((_errors, config)) => {
                        // for why in errors {
                        //     tracing::error!(%why, "error loading app config");
                        // }

                        config
                    }
                })
                .unwrap_or_default(),
            state: State::Disconnected,
            user_extended: None,
            user_tops: None,
            user_firsts: None,
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
        let user = self.user_extended.as_ref();
        match user {
            Some(user_inner) => self.draw_user(user_inner),
            None => title1("Waiting")
                .apply(container)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .into(),
        }
        // .map(|user| user.username.clone().into_string())
        // .unwrap_or("Waiting...".to_owned());
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
                    println!("Connected")
                }
                Event::Disconnected => {
                    self.state = State::Disconnected;
                    println!("Disconnected");
                }
                Event::MessageReceived(message) => match message {
                    Message::Connected => {}
                    Message::Disconnected => {}
                    Message::User(user_extended) => {
                        println!("User received: {}", user_extended.username);
                        self.user_extended = Some(user_extended);
                    }
                    Message::Tops(vec) => {
                        println!("Top plays received: ");
                        self.user_tops = Some(vec);
                    }
                    Message::Firsts(vec) => {
                        println!("Firsts received: {:?}", vec.iter().next());
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

        // let icon = widget::svg(widget::svg::Handle::from_memory(""));

        let title = title3("rosu-tracker");

        let link = link("").on_press(AppMessage::OpenRepositoryUrl).padding(0);

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
    fn draw_user(&self, user: &UserExtended) -> Element<AppMessage> {
        let title = self.centered_username(user);

        // let mut items = vec![];
        // if let Some(statistics) = user.statistics.as_ref() {
        //     items.push(self.make_pair("pp", statistics.pp.to_string()));
        //     items.push(self.make_pair("rank", statistics.global_rank.unwrap_or(0).to_string()));
        //     items.push(self.make_pair(
        //         "country rank",
        //         statistics.country_rank.unwrap_or(0).to_string(),
        //     ));
        //     items.push(self.make_pair("score", statistics.ranked_score.to_string()));
        // } else {
        //     items.push(widget::text::Text::new("You have no pp!").into());
        //     items.push(widget::text::Text::new("You have no global rank!").into());
        // };
        // let items = cosmic::widget::column()
        //     .width(Length::Fill)
        //     .align_x(Horizontal::Center)
        //     .padding(20)
        //     .append(&mut items);
        let data = cosmic::widget::container(self.user_extended_data(user));
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

    fn user_extended_data(&self, user: &UserExtended) -> Element<AppMessage> {
        let items = cosmic::widget::column()
            .width(Length::Fill)
            .align_x(Horizontal::Center)
            .width(Length::Fill)
            .padding(20);
        let stats = user.statistics.as_ref();
        let children = vec![
            stats.map(|x| self.make_pair("pp", x.pp.to_string())),
            stats.map(|x| self.make_pair("rank", x.global_rank.unwrap_or(0).to_string())),
            stats.map(|x| self.make_pair("country rank", x.country_rank.unwrap_or(0).to_string())),
            Some(self.make_pair(
                "First places",
                user.scores_first_count.unwrap_or(0).to_string(),
            )),
            stats.map(|x| self.make_pair("score", x.ranked_score.apply(format_number))),
        ];
        let children = children.into_iter().flatten();

        let items = items.extend(children);
        items.into()
    }

    fn make_pair<'a>(&'a self, title: &'a str, data: impl Into<String>) -> Element<'a, AppMessage> {
        container(
            row![
                widget::text::Text::new(title)
                    .align_x(Horizontal::Left)
                    .size(16)
                    .width(Length::FillPortion(1)),
                // cosmic::widget::divider::vertical::default(),
                widget::text::Text::new(data.into())
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
}

/// The page to display in the application.
pub enum Page {
    UserPage(String),
    TopsPage(String),
    FirstsPage(String),
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
