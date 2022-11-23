mod route;
mod upload;
mod unique;
mod error;
mod auth;
mod middleware;
mod postman;
mod utils;
mod delete_image;

use anyhow::Result;
use actix_cors::Cors;
use actix_web::{App as ActixApp, HttpServer};
use actix_redis::RedisSession;

pub(crate) static PATH: &str = "/home/sankar/Projects/lily-images";
pub(crate) static TRASH: &str = "/home/sankar/trash";

#[actix_web::main]
async fn main() -> Result<()> {

    HttpServer::new(move || {
        let cors = Cors::default()
              .allow_any_origin()
              .allow_any_method()
              .allow_any_header()
              .supports_credentials();

        ActixApp::new()
            .wrap(cors)
            .wrap(
                RedisSession::new("127.0.0.1:6379", &[0; 32])
                .cookie_name("lily-session")
                .cookie_http_only(true)
                .ttl(86400)
            )
            .configure(route::routes)
    })
    .bind("127.0.0.1:7600")?
    .run()
    .await?;
    Ok(())
}