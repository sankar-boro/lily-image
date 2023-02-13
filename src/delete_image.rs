use crate::error::Error;
use crate::PATH;
use std::fs;

use actix_web::{HttpResponse, web};

pub async fn delete_image(payload: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    let full_path = format!("{}/{}/{}", PATH, &payload.0, &payload.1);
    fs::remove_file(&full_path)?;
    Ok(HttpResponse::Ok().body("Deleted."))
}
