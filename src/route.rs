use actix_files;
use std::path::PathBuf;
use actix_web::{web, HttpRequest, Result, Error, HttpResponse};
use actix_web::http::header::{ContentDisposition, DispositionType};

async fn index(req: HttpRequest) -> Result<actix_files::NamedFile, Error> {
    let mut images_dir = PathBuf::from("/home/sankar/Projects/lily-images/");
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

async fn home() -> HttpResponse {
    HttpResponse::Ok().body("Home!")
}

pub fn routes(config: &mut web::ServiceConfig) {
    config.route("/images/{filename:.*}", web::get().to(index));
    config.route("/", web::get().to(home));
}