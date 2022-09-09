use crate::unique::time_uuid;
use crate::error::Error;

use actix_web::{HttpResponse, web};
use actix_multipart::{Multipart, Field};
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

#[derive(Serialize, Deserialize)]
struct UploadResponse {
    image_url: String,
}

async fn create_file(p: &String) -> Result<std::fs::File, std::io::Error> {
    std::fs::File::create(p)
}

async fn save_image(field: &mut Field, token: &web::Path<(String, String)>) -> Result<(String,String,String), Error> {
    let user_dir = format!("{}{}", PATH, &token.0);
    let is_user_dir: bool = Path::new(&user_dir).is_dir();
    let post_dir = format!("{}/{}", user_dir, &token.1);
    let is_post_dir: bool = Path::new(&post_dir).is_dir();

    if !is_user_dir {
        std::fs::create_dir(&user_dir)?;
    }
    if !is_post_dir {
        std::fs::create_dir(&post_dir)?;
    }

    let content_type = field.content_disposition();
    let filename = content_type.get_filename();
    let filename = match filename {
        Some(r) => r,
        None => {
            return Err(Error::from("image processing failed.").into());
        }
    };
    let new_filename = time_uuid().to_string();
    let split_filename: Vec<&str> = filename.split('.').collect();
    let ext = split_filename[1].clone();
    let tmp_image = format!("{}/{}.tmp.{}", post_dir, new_filename, ext);
    let tmp_image_clone = tmp_image.clone();
    let cropped_image_path = format!("{}/{}.{}", post_dir, new_filename, &ext);
    let image_url = format!("{}/{}/{}.{}", token.0, token.1, new_filename, ext);
    
    // File::create is blocking operation, use threadpool
    let mut f = create_file(&tmp_image_clone).await;

    let mut done = false;
    // Field in turn is stream of *Bytes* object
    while let Some(chunk) = field.next().await {
        let data = chunk?;
        // filesystem operations are blocking, we have to use threadpool
        f = web::block(move || -> Result<std::fs::File, std::io::Error> {
            let mut g = f?; 
            g.write_all(&data)?;
            Ok(g)
        }).await?;
        done = true;
    }
    if done == false {
        return Err(Error::from("Could not save image.").into());
    }
    Ok((tmp_image, cropped_image_path, image_url))
}

async fn get_value(field: &mut Field) -> Result<Option<u32>, Error> {
    let mut value: Option<u32> = None;
    while let Some(chunk) = field.next().await {
        let data = chunk?;
        let v = String::from_utf8_lossy(&data).to_string();
        value = Some(v.parse()?);
    }
    Ok(value)
}


// NOTE: image wont upload from postman if you set Content-Type: multipart/form-data
// Postman->Body->binary
pub async fn upload_image(mut payload: Multipart, token: web::Path<(String, String)>) -> Result<HttpResponse, Error> {
    
    let mut image_data: Option<(String, String, String)> = None;
    let mut props: u32 = 0;
    let mut width: u32 = 0;
    let mut height: u32 = 0;
    let mut x: u32 = 0;
    let mut y: u32 = 0;

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        let name = match content_disposition.get_name() {
            Some(name) => name,
            None => {
                return Err(Error::from("Cannot get name").into());
            }
        };
        match name {
            "image" => {
                image_data = Some(save_image(&mut field, &token).await?);
                props += 1;
            },
            "width" => {
                match get_value(&mut field).await? {
                    Some(v) => {
                        width = v;
                        props += 1;
                    },
                    None => {}
                }
            },
            "height" => {
                match get_value(&mut field).await? {
                    Some(v) => {
                        height = v;
                        props += 1;
                    },
                    None => {}
                }
            },
            "x" => {
                match get_value(&mut field).await? {
                    Some(v) => {
                        x = v;
                        props += 1;
                    },
                    None => {}
                }
            },
            "y" => {
                match get_value(&mut field).await? {
                    Some(v) => {
                        y = v;
                        props += 1;
                    },
                    None => {}
                }
            },
            _ => {}
        }
    }
    
    let mut image_url: Option<String> = None;
    
    if props == 5 {
        if let Some(paths) = image_data {
            let mut img = image::open(&paths.0)?;
            let subimg = imageops::crop(&mut img, x, y, width, height);
            let d = subimg.to_image();
            d.save(&paths.1)?;
            image_url = Some(paths.2.clone());
        }
    }

    if image_url.is_none() {
        return Err(Error::from("Could not save image"));
    }
    Ok(HttpResponse::Ok().json(UploadResponse {
        image_url: image_url.unwrap()
    }))
}

// // NOTE: image wont upload from postman if you set Content-Type: multipart/form-data
// // Postman->Body->binary
// pub async fn test_image(mut payload: Multipart) -> Result<HttpResponse, Error> {
//     while let Ok(Some(mut field)) = payload.try_next().await {
//         let t = field.content_disposition();
//         let q = t.get_name().unwrap();
//         println!("Mime: {}", t);
//         println!("Name: {}", q);
        
//         if q != "image" {
            
//         }
//     }

//     Ok(HttpResponse::Ok().body("Ok."))
// }