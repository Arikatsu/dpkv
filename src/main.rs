use warp::Filter;

mod models;
mod parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api = warp::path("api")
        .and(warp::path("data"))
        .and(warp::get())
        .map(move || {
            warp::reply::json(&serde_json::json!({
                "message": "This is where the extracted data would be returned."
            }))
        });

    let static_files = warp::fs::dir("ui/dist")
        .or(warp::path::end().and(warp::fs::file("ui/dist/index.html")));

    let routes = api.or(static_files);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
    Ok(())
}