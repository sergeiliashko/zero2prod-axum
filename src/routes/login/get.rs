use axum::response::Html;
use axum_extra::extract::{cookie::Cookie, SignedCookieJar};

pub async fn login_form(signed_jar: SignedCookieJar) -> (SignedCookieJar, Html<String>) {
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
    <title>Login</title>
</head>
<body>
    {error_html}
    <form action="/login" method="post">
        <label>Username
            <input
type="text"
placeholder="Enter Username"
                name="username"
            >
        </label>
        <label>Password
            <input
                type="password"
                placeholder="Enter Password"
                name="password"
            >
</label>
        <button type="submit">Login</button>
    </form>
</body> </html>"#,
        )),
    )
}
