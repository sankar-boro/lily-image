use crate::unique::time_uuid;
use crate::error::Error;

use actix_web::{HttpResponse, web};
use actix_multipart::{Multipart, Field};
use futures::{StreamExt, TryStreamExt};
use std::{io::Write, path::Path, fs};
use serde::{Deserialize, Serialize};
use image::{self, imageops::{self, FilterType}, ImageEncoder};

static PATH: &str = "/home/sankar/Projects/lily-images";

#[derive(Serialize, Deserialize)]
pub struct UserRequest {
    user_id: String,
}

#[derive(Serialize, Deserialize)]
struct UploadResponse {
    image_name: String,
    ext: String,
    height_720: u32, 
    height_320: u32
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

fn create_dir(metadata: &MetaData) -> Result<String, Error> {
    let user_dir = format!("{}/{}", PATH, &metadata.user_id);
    let is_user_dir: bool = Path::new(&user_dir).is_dir();
    // let post_dir = format!("{}/{}", user_dir, &metadata.post_id);
    // let is_post_dir: bool = Path::new(&post_dir).is_dir();

    if !is_user_dir {
        std::fs::create_dir(&user_dir)?;
    }
    // if !is_post_dir {
    //     std::fs::create_dir(&post_dir)?;
    // }
    Ok(metadata.user_id.clone().to_owned())
}

#[derive(Debug)]
struct URLs{
    dim320: String,
    dim720: String,
    dim1024: String,
    tmp_s: String,
    image_name: String,
    ext: String,
    user_dir: String,
}

fn get_filename(field: &mut Field, user_dir: &str) -> Result<URLs, Error> {
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

    let dim320 = format!("{}_320.{}", &new_filename, ext);
    let dim720 = format!("{}_720.{}", &new_filename, ext);
    let dim1024 = format!("{}_1024.{}", &new_filename, ext);
    let tmp_s = format!("{}/{}/{}.tmp.{}", PATH, user_dir, &new_filename, ext);
    Ok(URLs {
        dim320,
        dim720,
        dim1024,
        tmp_s,
        image_name: new_filename,
        ext: ext.to_owned(),
        user_dir: user_dir.clone().to_owned()
    })
}

async fn save_image(field: &mut Field, metadata: &MetaData) -> Result<URLs, Error> {
    let user_dir = create_dir(&metadata)?;
    let filename = get_filename(field, &user_dir)?;
    
    // File::create is blocking operation, use threadpool
    let mut f = create_file(&filename.tmp_s).await;

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
    Ok(filename)
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


fn crop_image(paths: &URLs, metadata: &MetaData) -> Result<(u32, u32), Error> {
    let mut img = image::open(&paths.tmp_s)?;
    let subimg = imageops::crop(&mut img, metadata.x.clone(), metadata.y.clone(), metadata.width.clone(), metadata.height.clone());
    let d = subimg.to_image();
    let mut width_720 = metadata.width.clone();
    let mut height_720 = metadata.height.clone();
    let mut width_320 = metadata.width.clone();
    let mut height_320 = metadata.height.clone();
    if width_720 > 720 && height_720 > 576 {
        let crop_width_720 = (720*100)/width_720;
        width_720 = 720;
        height_720 = (height_720*crop_width_720)/100;


        let crop_width_320 = (320*100)/width_320;
        width_320 = 320;
        height_320 = (height_320*crop_width_320)/100;
    }
    let x = image::imageops::resize(&d, width_720, height_720, FilterType::Nearest);
    x.save(format!("{}/{}/{}", PATH, &paths.user_dir, &paths.dim720))?;

    let y = image::imageops::resize(&d, width_320, height_320, FilterType::Nearest);
    y.save(format!("{}/{}/{}", PATH, &paths.user_dir, &paths.dim320))?;

    fs::remove_file(&paths.tmp_s)?;

    Ok((height_720, height_320))
}

// NOTE: image wont upload from postman if you set Content-Type: multipart/form-data
// Postman->Body->binary
pub async fn upload_image(mut payload: Multipart) -> Result<HttpResponse, Error> {

    let mut image_data: Option<URLs> = None;
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
    
    let mut image_dim: (u32, u32) = (0, 0);
    if let Some(paths) = &image_data {
        if let Some(metadata) = metadata {
            image_dim = crop_image(&paths, &metadata)?;
        }
    }

    if image_data.is_none() {
        return Err(Error::from("Could not save image"));
    }

    let image_data = image_data.unwrap();

    Ok(HttpResponse::Ok().json(UploadResponse {
        image_name: image_data.image_name.clone(),
        ext: image_data.ext.clone(),
        height_720: image_dim.0,
        height_320: image_dim.1,
    }))
}
