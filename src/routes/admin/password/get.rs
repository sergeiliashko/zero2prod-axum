use axum::response::{Html, IntoResponse};
use axum_extra::extract::SignedCookieJar;
use cookie::Cookie;

pub async fn change_password_form(
    signed_jar: SignedCookieJar,
) -> impl IntoResponse  {

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
    <title>Change Password</title>
</head>
<body>
    {error_html}
    <form action="/admin/password" method="post">
        <label>Current password
                <input
                type="password"
                placeholder="Enter current password"
                name="current_password"
            >
        </label>
        <br>
        <label>New password
            <input
                type="password"
                placeholder="Enter new password"
                name="new_password"
            >
        </label>
        <br>
        <label>Confirm new password
            <input
                type="password"
                placeholder="Type the new password again"
                name="new_password_check"
            >
        </label>
        <br>
        <button type="submit">Change password</button>
</form>
    <p><a href="/admin/dashboard">&lt;- Back</a></p>
</body>
</html>"#,
    ))).into_response()
}
