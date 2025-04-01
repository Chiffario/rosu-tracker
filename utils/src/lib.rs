use ::image::DynamicImage;
use color_eyre::eyre::Result;
use crate::image::{form_url, fetch_url, parse_image};

mod image;
pub async fn try_get_image(beatmapset_id: u32) -> Result<DynamicImage> {
    let url = form_url(beatmapset_id);
    let image = fetch_url(url).await;
    let image = image.and_then(parse_image);
    image
}
