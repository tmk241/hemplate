use hemplate::{Hemplate, Url};

#[derive(Hemplate)]
struct UiDynamicTry {
    video_src: &'static str,
    video_media_type: &'static str,
}

fn view_url(value: &str) -> Result<Url, hemplate::UrlError> {
    Url::parse(value)
}

fn main() {}
