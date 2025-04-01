use cosmic::{app, iced_futures};
use cosmic::iced::futures::Stream;
use futures_util::SinkExt;
use crate::app::AppMessage;
use utils::try_get_image;

pub fn fetch_multiple(id_list: Box<[u32]>) -> impl Stream<Item = cosmic::action::Action<AppMessage>> {
    iced_futures::stream::channel(100, move |mut output| async move {
        for id in id_list {
            let image = try_get_image(id).await.ok();
            let _ = output
                .send(cosmic::action::Action::App(AppMessage::ReceiveBackground(
                    id, image,
                )))
                .await;
        }
    })
}