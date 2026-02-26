use axum::response::Html;
use axum::routing::get;
use axum::Router;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new().route(
        "/",
        get(|| async {
            Html(
                r#"<!doctype html>
<html>
  <head>
    <meta charset=\"utf-8\" />
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />
    <title>Litedocs</title>
    <style>
      body { font-family: ui-sans-serif, system-ui, -apple-system, Segoe UI, sans-serif; margin: 0; padding: 48px; background: #0d0e10; color: #f4f4f5; }
      .card { max-width: 720px; margin: 0 auto; border: 1px solid #27272a; border-radius: 16px; padding: 24px; background: #111318; }
      h1 { margin: 0 0 8px; font-size: 34px; }
      p { color: #a1a1aa; line-height: 1.6; margin: 0; }
    </style>
  </head>
  <body>
    <div class=\"card\">
      <h1>Litedocs</h1>
      <p>Landing site placeholder crate. Marketing pages will be implemented here.</p>
    </div>
  </body>
</html>"#,
            )
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("litedocs-website listening on http://127.0.0.1:3000");
    axum::serve(listener, app).await?;
    Ok(())
}
