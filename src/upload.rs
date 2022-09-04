use crate::unique::time_uuid;

use actix_web::{HttpResponse, web};
use actix_multipart::{Multipart};
use futures::{StreamExt, TryStreamExt};
use std::io::Write;
use serde::{Deserialize, Serialize};
use image::{self, imageops};

static PATH: &str = "/home/sankar/Projects/lily-images/";

#[derive(Serialize, Deserialize)]
pub struct UserRequest {
    user_id: String,
}

// NOTE: image wont upload from postman if you set Content-Type: multipart/form-data
// Postman->Body->binary
pub async fn upload_image(mut payload: Multipart) -> HttpResponse {
    // iterate over multipart stream
    let mut paths: Vec<(String, String)> = Vec::new();

    while let Ok(Some(mut field)) = payload.try_next().await {
        let filename = time_uuid().to_string();
        let filepath = format!("{}{}.tmp.{}", PATH, filename, "png");
        let filepath1 = format!("{}{}.{}", PATH, filename, "png");

        println!("filepath: {}", filepath);
        paths.push((filepath.clone(), filepath1.clone()));

        // File::create is blocking operation, use threadpool
        let mut f = web::block(|| std::fs::File::create(filepath))
            .await
            .unwrap();

        // Field in turn is stream of *Bytes* object
        while let Some(chunk) = field.next().await {
            let data = chunk.unwrap();
            // filesystem operations are blocking, we have to use threadpool
            f = web::block(move || {
                let mut g = f.unwrap(); 
                g.write_all(&data).unwrap();
                Ok(g)
            }).await.unwrap();
        }

    }
    for path in paths.iter() {
        let mut img = image::open(&path.0).unwrap();
        let subimg = imageops::crop(&mut img, 0, 0, 100, 100);
        let d = subimg.to_image();
        d.save(&path.1).unwrap();
    }

    HttpResponse::Ok().body("Image uploaded!")
}