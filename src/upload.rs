use crate::unique::time_uuid;
use crate::error::Error;

use actix_web::{HttpResponse, web};
use actix_multipart::{Multipart};
use futures::{StreamExt, TryStreamExt};
use std::{io::Write, path::Path};
use serde::{Deserialize, Serialize};
use image::{self, imageops};
// use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
struct Claims {
   userId: String,
   contextId: String,
   exp: usize,
}

static PATH: &str = "/home/sankar/Projects/lily-images/";

#[derive(Serialize, Deserialize)]
pub struct UserRequest {
    user_id: String,
}

// NOTE: image wont upload from postman if you set Content-Type: multipart/form-data
// Postman->Body->binary
pub async fn upload_image(mut payload: Multipart, token: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    
    // let decode_token = decode::<Claims>(&token, &DecodingKey::from_secret("secret".as_ref()), &Validation::new(Algorithm::HS512))?;
    let user_dir = format!("{}/{}", PATH, &token.0);
    let is_user_dir: bool = Path::new(&user_dir).is_dir();
    let post_dir = format!("{}/{}", user_dir, &token.1);
    let is_post_dir: bool = Path::new(&post_dir).is_dir();

    if !is_user_dir {
        std::fs::create_dir(&user_dir)?;
    }
    if !is_post_dir {
        std::fs::create_dir(&post_dir)?;
    }
    // iterate over multipart stream
    let mut paths: Vec<(String, String)> = Vec::new();

    while let Ok(Some(mut field)) = payload.try_next().await {
        // let con = field.content_disposition();
        // let ext = con.get_filename_ext();
        let content_type = field.content_disposition();
        let fileext = content_type.get_filename();
        let fileext = match fileext {
            Some(r) => r,
            None => {
                return Err(Error::from("image processing failed.").into());
            }
        };

        let filename = time_uuid().to_string();
        let ext = &fileext[fileext.len() - 3..];
        let filepath = format!("{}/{}.tmp.{}", post_dir, filename, ext);
        let filepath1 = format!("{}/{}.{}", post_dir, filename, &ext);
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
        let subimg = imageops::crop(&mut img, 120, 305, 1080, 607);
        let d = subimg.to_image();
        d.save(&path.1).unwrap();
    }

    Ok(HttpResponse::Ok().body("Image uploaded!"))
}