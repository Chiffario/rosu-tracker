#[cfg(feature = "gui")]
pub mod gui;

mod constants;
mod setup;
mod websocket;
#[cfg(feature = "gui")]
fn main() -> eyre::Result<()> {
    gui::init()?;
    Ok(())
}

#[cfg(not(feature = "gui"))]
#[tokio::main]
async fn main() -> eyre::Result<()> {
    use setup::thread_init;

    thread_init().await?;
    Ok(())
}
