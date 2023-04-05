use axum::response::{Html, IntoResponse};
use axum_extra::extract::SignedCookieJar;
use cookie::Cookie;

pub async fn send_newsletter(signed_jar: SignedCookieJar) -> impl IntoResponse {
    let error_html = match signed_jar.get("_flash") {
        None => "".into(),
        Some(cookie) => {
            format!("<p><i>{}</i></p>", cookie.value())
        }
    };

    (
        signed_jar.remove(Cookie::named("_flash")),
        Html(format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Send Newsletter</title>
</head>
<body>
    {error_html}
    <form action="/admin/newsletter" method="post">
        <label>Title
                <input
                type="text"
                placeholder="Enter title of your email"
                name="title"
            >
        </label>
        <br>
        <label>HTML content
            <input
                type="text"
                placeholder="Enter html content"
                name="html"
            >
        </label>
        <br>
        <label>TEXT content
            <input
                type="text"
                placeholder="Enter text content"
                name="text"
            >
        </label>
        <br>
        <button type="submit">Send newsletter</button>
</form>
    <p><a href="/admin/dashboard">&lt;- Back</a></p>
</body>
</html>"#,
        )),
    )
        .into_response()
}
