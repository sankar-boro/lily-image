use crate::unique::time_uuid;
use crate::error::Error;

use actix_web::{HttpResponse, web};
use actix_multipart::{Multipart, Field};
use futures::{StreamExt, TryStreamExt};
use std::{io::Write, path::Path, fs};
use serde::{Deserialize, Serialize};
use image::{self, imageops::{self, FilterType}};

static PATH: &str = "/home/sankar/Projects/lily-images/";

#[derive(Serialize, Deserialize)]
pub struct UserRequest {
    user_id: String,
}

#[derive(Serialize, Deserialize)]
struct UploadResponse {
    image_url: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct MetaData {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    user_id: String,
    post_id: String
}

async fn create_file(p: &String) -> Result<std::fs::File, std::io::Error> {
    std::fs::File::create(p)
}

fn create_dir(metadata: &MetaData) -> Result<(String, String), Error> {
    let user_dir = format!("{}{}", PATH, &metadata.user_id);
    let is_user_dir: bool = Path::new(&user_dir).is_dir();
    let post_dir = format!("{}/{}", user_dir, &metadata.post_id);
    let is_post_dir: bool = Path::new(&post_dir).is_dir();

    if !is_user_dir {
        std::fs::create_dir(&user_dir)?;
    }
    if !is_post_dir {
        std::fs::create_dir(&post_dir)?;
    }
    Ok((user_dir, post_dir))
}

async fn save_image(field: &mut Field, metadata: &MetaData) -> Result<(String,String,String), Error> {
    let (user_dir, post_dir) = create_dir(&metadata)?;

    let content_type = field.content_disposition();
    let meta_filename = content_type.get_filename();
    let meta_filename = match meta_filename {
        Some(r) => r,
        None => {
            return Err(Error::from("could not get metadata filename.").into());
        }
    };
    let split_filename: Vec<&str> = meta_filename.split('.').collect();
    if split_filename.len() != 2 {
        return Err(Error::from("error in splitting filename.").into());
    }

    let new_filename = time_uuid().to_string();
    let ext = split_filename[1].clone();

    let img_tmp_path = format!("{}/{}.tmp.{}", post_dir, new_filename, ext);
    // let tmp_image_clone = tmp_image.clone();
    let crp_img_path = format!("{}/{}.{}", post_dir, new_filename, &ext);
    let image_url = format!("{}/{}/{}.{}", &metadata.user_id, &metadata.post_id, new_filename, ext);
    
    // File::create is blocking operation, use threadpool
    let mut f = create_file(&img_tmp_path).await;

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
    Ok((img_tmp_path, crp_img_path, image_url))
}

async fn get_value(field: &mut Field) -> Result<Option<String>, Error> {
    let mut value: Option<String> = None;
    while let Some(chunk) = field.next().await {
        let data = chunk?;
        let v = String::from_utf8_lossy(&data).to_string();
        value = Some(v);
    }
    Ok(value)
}


fn crop_image(paths: &(String, String, String), metadata: &MetaData) -> Result<Option<String>, Error> {
    let mut image_url: Option<String> = None;
    let mut img = image::open(&paths.0)?;
    let subimg = imageops::crop(&mut img, metadata.x.clone(), metadata.y.clone(), metadata.width.clone(), metadata.height.clone());
    let d = subimg.to_image();
    let x = image::imageops::resize(&d, metadata.width.clone()/100*50, metadata.height.clone()/100*50, FilterType::Nearest);
    x.save(&paths.1)?;
    image_url = Some(paths.2.clone());
    fs::remove_file(paths.0.clone());
    Ok(image_url)
}

// NOTE: image wont upload from postman if you set Content-Type: multipart/form-data
// Postman->Body->binary
pub async fn upload_image(mut payload: Multipart) -> Result<HttpResponse, Error> {

    let mut image_data: Option<(String, String, String)> = None;
    let mut metadata: Option<MetaData> = None;

    while let Ok(Some(mut field)) = payload.try_next().await {
        let content_disposition = field.content_disposition();
        let name = match content_disposition.get_name() {
            Some(name) => name,
            None => {
                return Err(Error::from("Cannot get name").into());
            }
        };
        match name {
            "metadata" => {
                let x = get_value(&mut field).await?;
                if let Some(x) = x {
                    metadata = Some(serde_json::from_str(&x)?);
                } 
            },
            "image" => {
                if let Some(metadata) = &metadata {
                    image_data = Some(save_image(&mut field, metadata).await?);
                }
            },
            _ => {}
        }
    }
    
    let mut image_url: Option<String> = None;
    if let Some(paths) = image_data {
        if let Some(metadata) = metadata {
            image_url = crop_image(&paths, &metadata)?;
        }
    }

    if image_url.is_none() {
        return Err(Error::from("Could not save image"));
    }
    Ok(HttpResponse::Ok().json(UploadResponse {
        image_url: image_url.unwrap()
    }))
}
