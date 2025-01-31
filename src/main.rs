use clap::{Arg, ArgGroup, ArgMatches, Command};
use color_eyre::eyre::Result;
use setup::Api;

#[cfg(feature = "gui")]
mod gui;

mod config;
mod constants;
mod setup;
mod websocket;

fn generate_commands() -> Command {
    let mut command = Command::new("rosu-tracker").subcommand(
        Command::new("init")
            .about(
                "Runs initial configuration. 
Will override your previously saved settings if rerun! If ran without any flags, runs in an interactive mode",
            ).args([
                Arg::new("username")
                    .short('n')
                    .long("name")
                    .help("Tracked user's name")
                    .long_help("Username of the user you want to track"),
                Arg::new("client_id").short('i').long("id").help("Your osu!api client ID").long_help("Client ID for osu!api v2. If you don't know where to get one, visit https://osu.ppy.sh/home/account/edit"),
                Arg::new("client_secret").short('s').long("secret").help("Your osu!api client secret").long_help("Client secret for osu!api v2. If you don't know where to get one, visit https://osu.ppy.sh/home/account/edit"),
            ])
            .group(ArgGroup::new("setup_flags").args(["username", "client_id", "client_secret"]).multiple(true).requires_all(["setup_flags"])),
    );

    #[cfg(feature = "gui")]
    {
        command = command.arg(Arg::new("gui"));
    }
    command
}

fn cli_flag_handler(matches: ArgMatches) -> Option<Api> {
    if let Some(init) = matches.subcommand_matches("init") {
        if init.get_one::<String>("username").is_some() {
            let mut api = Api::default();
            api.id = match init.get_one::<String>("client_id") {
                Some(id) => id.to_owned(),
                None => {
                    eprintln!("ID not found, please provide --id <id>");
                    return None;
                }
            };
            api.secret = match init.get_one::<String>("client_secret") {
                Some(secret) => secret.to_owned(),
                None => {
                    eprintln!("Secret not found, please provide --secret <secret>");
                    return None;
                }
            };
            api.username = match init.get_one::<String>("username") {
                Some(username) => username.to_owned(),
                None => {
                    eprintln!("Username not found, please provide --name <username>");
                    return None;
                }
            };
            return Some(api);
        }
        if !matches.args_present() {
            return None;
        }
    }
    None
}

#[cfg(not(feature = "gui"))]
#[tokio::main]
async fn main() -> Result<()> {
    let command = generate_commands();
    let matches = command.get_matches();

    let config = cli_flag_handler(matches);

    tui_init(config).await
}

#[cfg(feature = "gui")]
fn main() -> Result<()> {
    let command = generate_commands();
    let matches = command.get_matches();

    let config = cli_flag_handler(matches);

    gui_init(config)
}

#[cfg(not(feature = "gui"))]
async fn tui_init(config: Option<Api>) -> Result<()> {
    thread_init(config).await?;
    Ok(())
}

#[cfg(feature = "gui")]
fn gui_init(config: Option<Api>) -> Result<()> {
    gui::init_with_flags(config)?;
    Ok(())
}
