mod route;
mod upload;
mod unique;
mod error;

use anyhow::Result;
use actix_cors::Cors;
use actix_web::{App as ActixApp, HttpServer};
use actix_web::web::{
    self, 
    // Data
};

use scylla::batch::Batch;
use scylla::{
    Session, 
    SessionBuilder, 
    transport::errors::NewSessionError
};
use scylla::{QueryResult, BatchResult};
use scylla::query::Query;
use scylla::frame::value::ValueList;
use scylla::frame::value::BatchValues;
use scylla::transport::errors::QueryError;

use log::error;
use std::sync::Arc;

#[derive(Clone)]
pub struct App {
    session: Arc<Session>,
}

impl App {
    fn new(session: Session) -> Self {
        Self {
            session: Arc::new(session),
        }
    }

    pub async fn query(&self, query: impl Into<Query>, values: impl ValueList) -> Result<QueryResult, QueryError>{
        self.session.query(query, values).await
    }

    pub async fn batch(&self, query: &Batch, values: impl BatchValues) -> Result<BatchResult, QueryError>{
        self.session.batch(query, values).await
    }
}

pub async fn get_db_session() -> Session {
    let uri = "127.0.0.1:9042";
    let session = SessionBuilder::new().known_node(uri).build().await;
    if let Err(err) = session {
        match err {
            NewSessionError::FailedToResolveAddress(e) => error!("FailedToResolveAddress, {}", e),
            NewSessionError::EmptyKnownNodesList => error!("EmptyKnownNodesList"),
            NewSessionError::DbError(e, er) => error!("DbError, {} {}", e, er),
            NewSessionError::BadQuery(e) => error!("BadQuery, {}", e),
            NewSessionError::IoError(e) => {
                error!("IoError, {}", e);
                println!("Would you mind to check if you have started scylladb service. Command is: \"sudo systemctl start scylla-server\" ");
            },
            NewSessionError::ProtocolError(e) => error!("ProtocolError, {}", e),
            NewSessionError::InvalidMessage(e) => error!("InvalidMessage, {}", e),
            NewSessionError::TimeoutError => error!("TimeoutError"),
        }
        panic!("Could not start server");
    }
    session.unwrap()
}

#[actix_web::main]
async fn main() -> Result<()> {
    let app = App::new(get_db_session().await);

    HttpServer::new(move || {
        let cors = Cors::default()
              .allow_any_origin()
              .allow_any_method()
              .allow_any_header()
              .supports_credentials();

        ActixApp::new()
            .wrap(cors)
            .app_data(web::Data::new(app.clone()))
            .configure(route::routes)
    })
    .bind("127.0.0.1:7600")?
    .run()
    .await?;
    Ok(())
}