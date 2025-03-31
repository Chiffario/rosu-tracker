use crate::config::Config as AppConfig;
use crate::gui::socket;
use crate::setup::thread_init;
use crate::utils::image::fetch_multiple;
use cosmic::app::{context_drawer, Core, Task};
use cosmic::cosmic_config::{self, Config, CosmicConfigEntry};
use cosmic::iced::advanced::widget::{self};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::wgpu::naga::FastHashMap;
use cosmic::iced::{Alignment, Length, Padding, Subscription};
use cosmic::iced_widget::{column, row};
use cosmic::prelude::CollectionWidget;
use cosmic::theme::Container;
use cosmic::widget::button::link;
use cosmic::widget::text;
use cosmic::widget::text::{title1, title3};
use cosmic::widget::{container, icon, menu, nav_bar, scrollable, vertical_space};
use cosmic::{cosmic_theme, theme, Application, ApplicationExt, Apply, Element, Theme};
use image::DynamicImage;
use rosu_v2::prelude::{Score, UserExtended};
use std::collections::HashMap;
use tracing::{debug, error};

use super::components::{draw_scores, draw_user};
use super::socket::{Event, Message};

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
    #[allow(
        dead_code,
        reason = "The config isn't editable in runtime yet, but the handler is necessary"
    )]
    config_handler: Option<Config>,
    config: AppConfig,
    // State of the websocket connection
    state: State,
    // Latest received user data
    user_extended: Option<Box<UserExtended>>,
    user_tops: Option<Vec<Score>>,
    user_firsts: Option<Vec<Score>>,
    user_recent: Option<Vec<Score>>,
    // First received (a.k.a. initial) user data
    initial_user_extended: Option<Box<UserExtended>>,
    initial_user_tops: Option<Vec<Score>>,
    initial_user_firsts: Option<Vec<Score>>,
    // In-memory background cover cache
    backgrounds: FastHashMap<u32, Option<DynamicImage>>,
}

#[derive(Default)]
pub enum State {
    #[default]
    Disconnected,
    Connected,
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
    ReceiveBackground(u32, Option<DynamicImage>),
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
    const APP_ID: &'static str = crate::constants::APP_ID;

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
        nav.insert()
            .text("Recent")
            .data::<Page>(Page::RecentPage)
            // text-html looks like Earth, makes sense for leaderboards, fight me
            .icon(icon::from_name("text-html-symbolic"));

        let (config_handler, config) =
            match cosmic_config::Config::new(Self::APP_ID, AppConfig::VERSION) {
                Ok(config_handler) => {
                    let config = match AppConfig::get_entry(&config_handler) {
                        Ok(ok) => ok,
                        Err((errs, config)) => {
                            error!("errors loading config: {:?}", errs);
                            config
                        }
                    };
                    (Some(config_handler), config)
                }
                Err(err) => {
                    error!("failed to create config handler: {}", err);
                    (None, AppConfig::default())
                }
            };

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            nav,
            key_binds: HashMap::new(),
            // Optional configuration file for an application.
            config_handler,
            config,
            ..Default::default()
        };

        let rename = app.update_title();

        // Create a startup command that starts the socket server.
        let command = Task::perform(thread_init(None), |_| {
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
            Some(Page::RecentPage) => self.recent_view(),
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
            Subscription::run(socket::connect_user).map(AppMessage::ReceiveMessage),
            Subscription::run(socket::connect_tops).map(AppMessage::ReceiveMessage),
            Subscription::run(socket::connect_firsts).map(AppMessage::ReceiveMessage),
            Subscription::run(socket::connect_recent).map(AppMessage::ReceiveMessage),
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
                Event::Connected(_connection) => {
                    self.state = State::Connected;
                }
                Event::Disconnected => {
                    self.state = State::Disconnected;
                }
                Event::MessageReceived(message) => match message {
                    Message::Connected => {}
                    Message::Disconnected => {}
                    Message::User(user_extended) => {
                        debug!("User received: {}", user_extended.username);
                        self.user_extended = Some(user_extended.clone());
                        if self.initial_user_extended.is_none() {
                            self.initial_user_extended = Some(user_extended);
                        }
                    }
                    Message::Tops(vec) => {
                        debug!("Top plays received: {}", vec.len());
                        self.user_tops = Some(vec.clone());
                        let ids: Box<[u32]> = vec
                            .iter()
                            .map(|map| map.mapset.as_ref().unwrap().mapset_id)
                            .filter(|id| self.backgrounds.contains_key(id))
                            .collect();
                        if self.initial_user_tops.is_none() {
                            self.initial_user_tops = Some(vec);
                        }
                        let stream = fetch_multiple(ids);
                        return Task::stream(stream);
                    }
                    Message::Firsts(vec) => {
                        debug!("Firsts received: {}", vec.len());
                        self.user_firsts = Some(vec.clone());
                        if self.initial_user_firsts.is_none() {
                            self.initial_user_firsts = Some(vec);
                        }
                    }
                    Message::Recent(vec) => {
                        debug!("Recent received: {}", vec.len());
                        self.user_recent = Some(vec);
                    }
                },
            },
            AppMessage::ReceiveBackground(id, image) => {
                self.backgrounds.insert(id, image);
            }
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
            Some(user_inner) => draw_user(
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
        draw_scores(
            self.user_tops.as_ref().unwrap().as_slice(),
            &self.backgrounds,
        )
    }
    fn firsts_view(&self) -> Element<AppMessage> {
        draw_scores(
            self.user_firsts.as_ref().unwrap().as_slice(),
            &self.backgrounds,
        )
    }
    fn recent_view(&self) -> Element<AppMessage> {
        draw_scores(
            self.user_recent.as_ref().unwrap().as_slice(),
            &self.backgrounds,
        )
    }
}

/// The page to display in the application.
pub enum Page {
    UserPage,
    TopsPage,
    FirstsPage,
    RecentPage,
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
