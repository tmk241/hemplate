use hemplate::{
    AnimationValue, DocumentValue, NavigationValue, ResourceValue, ScriptValue, SrcsetValue,
    StyleValue, TrustedUrl, Url,
};

fn script(_: impl ScriptValue) {}
fn style(_: impl StyleValue) {}
fn document(_: impl DocumentValue) {}
fn navigation(_: impl NavigationValue) {}
fn resource(_: impl ResourceValue) {}
fn animation(_: impl AnimationValue) {}
fn srcset(_: impl SrcsetValue) {}

fn main() {
    let url = Url::parse("/page").unwrap();
    script(url.clone());
    style(url.clone());
    document(url.clone());
    navigation(url.clone());
    resource(url.clone());
    animation(url.clone());
    srcset(url);
    // A value vouched for as a URL is not vouched for as anything else.
    script(TrustedUrl::new_unchecked("x"));
}
