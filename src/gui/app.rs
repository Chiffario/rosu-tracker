// SPDX-License-Identifier: MIT
use crate::gui::config::Config as AppConfig;
use crate::gui::socket;
use crate::setup::thread_init;
use cosmic::app::{context_drawer, Core, Task};
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length, Subscription};
use cosmic::widget::{self, icon, menu, nav_bar};
use cosmic::{cosmic_theme, theme, Application, ApplicationExt, Apply, Element};
use futures_util::SinkExt;
use rosu_v2::prelude::{Score, SmallString, UserExtended};
use std::collections::HashMap;
use tokio::task::AbortHandle;
use tracing::debug;

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
    // Join handles for the server
    server_handles: Option<Vec<AbortHandle>>,
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
    SubscriptionChannel,
    ToggleContextPage(ContextPage),
    UpdateConfig(AppConfig),
    LaunchUrl(String),
    StartServer(Vec<AbortHandle>),
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
            .data::<Page>(Page::Page1("Balls".to_owned()))
            .icon(icon::from_name("applications-science-symbolic"))
            .activate();

        nav.insert()
            .text("Tops")
            .data::<Page>(Page::Page2("Dicks".to_owned()))
            .icon(icon::from_name("applications-system-symbolic"));

        nav.insert()
            .text("Firsts")
            .data::<Page>(Page::Page3("Boobs".to_owned()))
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
            server_handles: None,
            state: State::Disconnected,
            user_extended: None,
            user_tops: None,
            user_firsts: None,
        };

        let rename = app.update_title();

        // Create a startup command that starts the socket server.
        let command = Task::perform(thread_init(), |handles| {
            cosmic::app::Message::App(AppMessage::StartServer(handles.unwrap()))
        });
        (app, command)
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
        let title = self
            .user_extended
            .as_ref()
            .map(|user| user.username.clone().into_string())
            .unwrap_or("awo".to_owned());
        widget::text::title1(title)
            .apply(widget::container)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .into()
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-running async tasks running in the background which
    /// emit messages to the application through a channel. They are started at the
    /// beginning of the application, and persist through its lifetime.
    fn subscription(&self) -> Subscription<Self::Message> {
        struct MySubscription;

        Subscription::batch(vec![
            // Create a subscription which emits updates through a channel.
            Subscription::run_with_id(
                std::any::TypeId::of::<MySubscription>(),
                cosmic::iced::stream::channel(4, move |mut channel| async move {
                    _ = channel.send(AppMessage::SubscriptionChannel).await;

                    futures_util::future::pending().await
                }),
            ),
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

            AppMessage::SubscriptionChannel => {
                // For example purposes only.
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
            AppMessage::StartServer(handles) => self.server_handles = Some(handles),
            AppMessage::ReceiveMessage(event) => match event {
                Event::Connected(connection) => {
                    self.state = State::Connected(connection);
                    println!("Connected")
                }
                Event::Disconnected => {
                    self.state = State::Disconnected;
                    println!("Disconnected")
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
        if let Some(h) = &self.server_handles {
            for head in h {
                head.abort();
            }
        }
        if let State::Connected(_connection) = &self.state {
            debug!("Trying to drop connection")
        }
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

        let title = widget::text::title3("rosu-tracker");

        let link = widget::button::link("")
            .on_press(AppMessage::OpenRepositoryUrl)
            .padding(0);

        widget::column()
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
}

/// The page to display in the application.
pub enum Page {
    Page1(String),
    Page2(String),
    Page3(String),
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
