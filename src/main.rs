mod route;

use anyhow::Result;
use actix_cors::Cors;
use actix_web::{App as ActixApp, HttpServer};

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
            .configure(route::routes)
    })
    .bind("127.0.0.1:7600")?
    .run()
    .await?;
    Ok(())
}