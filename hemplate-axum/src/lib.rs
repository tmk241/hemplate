//! axum integration for hemplate templates.
//!
//! Wrap any [`Hemplate`] type in [`HtmlTemplate`] (or [`StatusTemplate`]) to
//! return it from an axum handler:
//!
//! ```ignore
//! async fn home() -> HtmlTemplate<HomeView> {
//!     HomeView { title: "Welcome".into() }.into()
//! }
//! ```
//!
//! No blanket `impl<T: Hemplate> IntoResponse for T` exists — Rust's coherence
//! rules forbid it (foreign trait + foreign type + overlap with axum's existing
//! blanket impls). The wrapper is the canonical workaround, matching axum's own
//! `Json` / `Html` / `Form` newtypes.

use axum_core::response::{IntoResponse, Response};
use hemplate_runtime::Hemplate;
use http::{header, StatusCode};

/// Wrapper that converts a [`Hemplate`] type into a 200 OK axum [`Response`]
/// with `Content-Type: text/html; charset=utf-8`.
///
/// Render errors map to 500 Internal Server Error with a plain-text body —
/// applications wanting different error policy should wrap the template
/// themselves rather than reaching for configuration on this type.
///
/// ```ignore
/// async fn home() -> HtmlTemplate<HomePage> {
///     HomePage { title: "Welcome".into() }.into_html()
/// }
/// ```
pub struct HtmlTemplate<T: Hemplate>(pub T);

impl<T: Hemplate> From<T> for HtmlTemplate<T> {
    fn from(view: T) -> Self {
        Self(view)
    }
}

impl<T: Hemplate> IntoResponse for HtmlTemplate<T> {
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => {
                ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], html).into_response()
            }
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template render error: {err}"),
            )
                .into_response(),
        }
    }
}

/// Wrapper for pairing a [`Hemplate`] type with an HTTP status code.
///
/// ```ignore
/// async fn not_found() -> StatusTemplate<NotFoundPage> {
///     NotFoundPage { path: "/oops".into() }.with_status(StatusCode::NOT_FOUND)
/// }
/// ```
pub struct StatusTemplate<T: Hemplate>(pub StatusCode, pub T);

impl<T: Hemplate> IntoResponse for StatusTemplate<T> {
    fn into_response(self) -> Response {
        match self.1.render() {
            Ok(html) => (
                self.0,
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                html,
            )
                .into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template render error: {err}"),
            )
                .into_response(),
        }
    }
}

/// Ergonomic extension methods for any [`Hemplate`] type — chainable wrapper
/// construction without naming the wrapper type at the call site.
///
/// ```ignore
/// async fn home() -> impl IntoResponse {
///     HomePage { title: "Welcome".into() }.into_html()
/// }
///
/// async fn not_found() -> impl IntoResponse {
///     NotFoundPage { path: "/oops".into() }.with_status(StatusCode::NOT_FOUND)
/// }
/// ```
pub trait HemplateExt: Hemplate + Sized {
    /// Wrap this template as a 200 OK HTML response.
    fn into_html(self) -> HtmlTemplate<Self> {
        HtmlTemplate(self)
    }

    /// Wrap this template as an HTML response with an explicit status code.
    fn with_status(self, status: StatusCode) -> StatusTemplate<Self> {
        StatusTemplate(status, self)
    }
}

impl<T: Hemplate> HemplateExt for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use hemplate_runtime::RenderError;
    use http_body_util::BodyExt;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    struct TestView;

    impl Hemplate for TestView {
        fn render_into(&self, buf: &mut dyn core::fmt::Write) -> Result<(), RenderError> {
            buf.write_str("<h1>Hello</h1>")?;
            Ok(())
        }
    }

    fn content_type(resp: &Response) -> Option<&str> {
        resp.headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
    }

    #[tokio::test]
    async fn html_template_returns_200_with_html_content_type() -> TestResult {
        let resp = HtmlTemplate(TestView).into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(content_type(&resp), Some("text/html; charset=utf-8"));
        let body = resp.into_body().collect().await?.to_bytes();
        assert_eq!(body, "<h1>Hello</h1>");
        Ok(())
    }

    #[tokio::test]
    async fn status_template_returns_custom_status() -> TestResult {
        let resp = StatusTemplate(StatusCode::NOT_FOUND, TestView).into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        assert_eq!(content_type(&resp), Some("text/html; charset=utf-8"));
        let body = resp.into_body().collect().await?.to_bytes();
        assert_eq!(body, "<h1>Hello</h1>");
        Ok(())
    }

    // --- S89: Render error → 500 INTERNAL_SERVER_ERROR ---

    struct FailView;

    impl Hemplate for FailView {
        fn render_into(&self, _buf: &mut dyn core::fmt::Write) -> Result<(), RenderError> {
            Err(RenderError::Fmt(std::fmt::Error))
        }
    }

    #[tokio::test]
    async fn html_template_render_error_returns_500() -> TestResult {
        let resp = HtmlTemplate(FailView).into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = resp.into_body().collect().await?.to_bytes();
        let text = String::from_utf8_lossy(&body);
        assert!(
            text.contains("Template render error"),
            "error body should contain 'Template render error': {}",
            text
        );
        Ok(())
    }

    #[tokio::test]
    async fn status_template_render_error_returns_500() -> TestResult {
        let resp = StatusTemplate(StatusCode::NOT_FOUND, FailView).into_response();
        // Even though we asked for 404, render failure → 500
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = resp.into_body().collect().await?.to_bytes();
        let text = String::from_utf8_lossy(&body);
        assert!(
            text.contains("Template render error"),
            "error body should contain 'Template render error': {}",
            text
        );
        Ok(())
    }

    // --- Ergonomics: From + HemplateExt ---

    #[tokio::test]
    async fn from_view_constructs_html_template() -> TestResult {
        let wrapped: HtmlTemplate<TestView> = TestView.into();
        let resp = wrapped.into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await?.to_bytes();
        assert_eq!(body, "<h1>Hello</h1>");
        Ok(())
    }

    #[tokio::test]
    async fn ext_into_html_wraps_with_200_and_html_content_type() {
        let resp = TestView.into_html().into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(content_type(&resp), Some("text/html; charset=utf-8"));
    }

    #[test]
    fn adapter_leaves_application_http_policy_outside_its_api() {
        let source = include_str!("lib.rs").split("#[cfg(test)]").next().unwrap();
        for forbidden in [
            "axum::Router",
            "axum::middleware",
            "CACHE_CONTROL",
            "ETAG",
            "HeaderMap",
            "tower::",
        ] {
            assert!(
                !source.contains(forbidden),
                "adapter owns forbidden policy: {forbidden}"
            );
        }
    }

    #[tokio::test]
    async fn ext_with_status_pairs_view_with_given_status() -> TestResult {
        let resp = TestView
            .with_status(StatusCode::IM_A_TEAPOT)
            .into_response();
        assert_eq!(resp.status(), StatusCode::IM_A_TEAPOT);
        let body = resp.into_body().collect().await?.to_bytes();
        assert_eq!(body, "<h1>Hello</h1>");
        Ok(())
    }
}
