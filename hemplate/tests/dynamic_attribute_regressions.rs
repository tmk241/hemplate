use std::cell::Cell;

use hemplate::{Hemplate, Url};

#[derive(Hemplate)]
struct PreparedVideoUrl {
    video_src: Url,
    media_type_calls: Cell<usize>,
}

impl PreparedVideoUrl {
    fn media_type(&self) -> &'static str {
        self.media_type_calls.set(self.media_type_calls.get() + 1);
        "video/mp4"
    }
}

struct VideoSource {
    url: Url,
    calls: Cell<usize>,
}

fn view_url(source: &VideoSource) -> &Url {
    source.calls.set(source.calls.get() + 1);
    &source.url
}

#[derive(Hemplate)]
struct HelperVideoUrl {
    video_src: VideoSource,
    media_type_calls: Cell<usize>,
}

impl HelperVideoUrl {
    fn media_type(&self) -> &'static str {
        self.media_type_calls.set(self.media_type_calls.get() + 1);
        "video/mp4"
    }
}

const EXPECTED: &str = "<video><source src=\"/video.mp4?x=1&amp;y=&quot;two&quot;\" type=\"video/mp4\">Video unavailable.</video><p>After video.</p>";

#[test]
fn prepared_url_keeps_source_fallback_closing_tag_and_following_content() {
    let template = PreparedVideoUrl {
        video_src: Url::parse("/video.mp4?x=1&y=\"two\"").unwrap(),
        media_type_calls: Cell::new(0),
    };
    assert_eq!(template.render().unwrap(), EXPECTED);
    assert_eq!(template.media_type_calls.get(), 1);
}

#[test]
fn supported_helper_is_called_once_per_render_without_losing_markup() {
    let template = HelperVideoUrl {
        video_src: VideoSource {
            url: Url::parse("/video.mp4?x=1&y=\"two\"").unwrap(),
            calls: Cell::new(0),
        },
        media_type_calls: Cell::new(0),
    };
    for calls in 1..=2 {
        assert_eq!(template.render().unwrap(), EXPECTED);
        assert_eq!(template.video_src.calls.get(), calls);
        assert_eq!(template.media_type_calls.get(), calls);
    }
}
