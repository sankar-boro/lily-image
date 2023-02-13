use crate::upload::{upload_image, update_image};
use crate::postman;
use crate::unique::time_uuid;
use crate::delete_image::delete_image;
use crate::{PATH};

use std::fs;
use actix_files;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::middleware::Authentication;

use crate::error::Error;
use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::http::header::{ContentDisposition, DispositionType};
use anyhow::Result;

async fn index(req: HttpRequest) -> Result<actix_files::NamedFile, Error> {
    let mut images_dir = PathBuf::from(PATH);
    let file_name: std::path::PathBuf = req.match_info().query("filename").parse().unwrap();
    images_dir.push(file_name);

    let full_path = images_dir.to_str().map(|a| a.to_owned()).unwrap();
    let file = actix_files::NamedFile::open(full_path)?;
    Ok(file
        .use_last_modified(true)
        .set_content_disposition(ContentDisposition {
            disposition: DispositionType::Attachment,
            parameters: vec![],
        }))
}

async fn get_image_by_id(path: web::Path<(String, String)>) -> Result<actix_files::NamedFile, Error> {
    let full_path = format!("{}/{}/{}", PATH, path.0, path.1);
    let file = actix_files::NamedFile::open(&full_path)?;
    Ok(file
        .use_last_modified(true)
        .set_content_disposition(ContentDisposition {
            disposition: DispositionType::Attachment,
            parameters: vec![],
        }))
}

#[derive(Serialize, Deserialize)]
struct UserRequest {
    user_id: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    status: u16,
    message: String,
}

async fn create_user_dir(request: web::Json<UserRequest>) -> HttpResponse {
    let path = format!("{}{}", PATH, &request.user_id);
    match fs::create_dir(&path) {
        Ok(_) => {
            HttpResponse::Ok().body("Created dir.")
        },
        Err(e) => {
            HttpResponse::build(StatusCode::FORBIDDEN).json(ErrorResponse {
                status: 403,
                message: e.to_string()
            })
        }
    }
}

async fn home() -> HttpResponse {
    HttpResponse::Ok().body("Home!")
}

async fn new_uuid(path: web::Path<String>) -> HttpResponse {
    let l = str::parse::<u32>(path.as_ref()).unwrap();
    for _ in 0..l {
        let x = time_uuid();
        let y = x.to_string();
        println!("'{}'", y);
    }
    HttpResponse::Ok().body("Done.")
}

pub fn routes(config: &mut web::ServiceConfig) {
    config.route("/", web::get().to(home));
    config.route("/images/{filename:.*}", web::get().to(index));
    config.route("/getimage/{userid}/{filename}", web::get().to(get_image_by_id));
    config.route("/create_user_dir", web::post().to(create_user_dir));
    config.service(web::resource("/upload_image").route(web::post().to(upload_image)));
    config.service(web::resource("/update_image").route(web::post().to(update_image)));
    config.service(web::resource("/test_image").route(web::post().to(postman::upload_image)));
    config.route("/new_id/{id}", web::get().to(new_uuid));
    config.service(
        web::scope("/delete")
        .wrap(Authentication{})
        .route("/image/{userId}/{image_name}", web::post().to(delete_image))
    );
}