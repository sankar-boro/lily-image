use crate::unique::time_uuid;
use crate::error::Error;
use crate::{PATH, TRASH};

use actix_web::{HttpResponse, web};
use actix_multipart::{Multipart, Field};
use futures::{StreamExt, TryStreamExt};
use std::{io::Write, path::Path, fs};
use serde::{Deserialize, Serialize};
use image::{self, imageops::{self, FilterType}};

#[derive(Serialize, Deserialize)]
pub struct UserRequest {
    user_id: String,
}

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)]
struct UploadResponse {
    imgName: String,
    imgExt: String,
    imgMd: Option<u32>, 
    imgSm: Option<u32>,
    imgLg: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone)]
#[allow(non_snake_case)]
struct RequestMetadata {
    xAxis: u32,
    yAxis: u32,
    imgWidth: u32,
    imgHeight: u32,
    userId: String
}

#[derive(Serialize, Deserialize, Clone)]
#[allow(non_snake_case)]
struct RequestMetadataUpdate {
    xAxis: u32,
    yAxis: u32,
    imgWidth: u32,
    imgHeight: u32,
    userId: String,
    imgExt: String,
    imgName: String,
}

impl RequestMetadataUpdate {
    fn move_trash(&self) -> Result<(), std::io::Error> {
        let from_sm = format!("{}/{}/{}_320.{}", PATH, &self.userId, &self.imgName, &self.imgExt);
        let from_md = format!("{}/{}/{}_720.{}", PATH, &self.userId, &self.imgName, &self.imgExt);
        let to_sm = format!("{}/{}_320.{}", TRASH, &self.imgName, &self.imgExt);
        let to_md = format!("{}/{}_720.{}", TRASH, &self.imgName, &self.imgExt);
        fs::rename(from_sm, to_sm)?;
        fs::rename(from_md, to_md)?;
        Ok(())
    }
}

impl RequestMetadata {
    fn create_dir(&self) -> Result<(), Error> {
        let user_dir = format!("{}/{}", PATH, &self.userId);
        let is_user_dir: bool = Path::new(&user_dir).is_dir();
        if !is_user_dir {
            std::fs::create_dir(&user_dir)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
#[allow(non_snake_case)]
struct ImageProps {
    imgName: String,
    imgExt: String,
    tmpPath: String,
}

fn create_url(field: &mut Field, user_id: &str) -> Result<ImageProps, Error> {
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
    let img_name = time_uuid().to_string();
    let img_ext = split_filename[1].to_owned();
    let tmp_path = format!("{}/{}/{}.tmp.{}", PATH, user_id, &img_name, &img_ext);

    match img_ext.as_str() {
        "jpg" | "jpeg" | "png" => {},
        _ => {
            return Err(Error::from("INVALID_EXT")); 
        }
    }

    Ok(ImageProps {
        imgName: img_name,
        imgExt: img_ext,
        tmpPath: tmp_path
    })
}

async fn parse_image(field: &mut Field, path: &str) -> Result<(), Error> {
    let mut f = std::fs::File::create(&path);

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
    Ok(())
}

async fn parse_metadata(field: &mut Field) -> Result<Option<RequestMetadata>, Error> {
    let mut value: Option<RequestMetadata> = None;
    while let Some(chunk) = field.next().await {
        let data = chunk?;
        let v = serde_json::from_str(&String::from_utf8_lossy(&data).to_string())?;
        value = Some(v);
    }
    Ok(value)
}

async fn parse_metadata_update(field: &mut Field) -> Result<Option<RequestMetadataUpdate>, Error> {
    let mut value: Option<RequestMetadataUpdate> = None;
    while let Some(chunk) = field.next().await {
        let data = chunk?;
        let v = serde_json::from_str(&String::from_utf8_lossy(&data).to_string())?;
        value = Some(v);
    }
    Ok(value)
}

fn crop_image(img_props: &ImageProps, x: u32, y: u32, w: u32, h: u32, user_id: &str) -> Result<(u32, u32), Error> {
    let mut img = image::open(&img_props.tmpPath)?;
    let subimg = imageops::crop(&mut img, x, y, w, h);
    let d = subimg.to_image();
    let mut width_720 = w;
    let mut height_720 = h;
    let mut width_320 = w;
    let mut height_320 = h;
    if width_720 > 720 && height_720 > 576 {
        let crop_width_720 = (720*100)/width_720;
        width_720 = 720;
        height_720 = (height_720*crop_width_720)/100;


        let crop_width_320 = (320*100)/width_320;
        width_320 = 320;
        height_320 = (height_320*crop_width_320)/100;
    }
    let x = image::imageops::resize(&d, width_720, height_720, FilterType::Nearest);
    x.save(format!("{}/{}/{}_720.{}", PATH, user_id, &img_props.imgName, &img_props.imgExt))?;

    let y = image::imageops::resize(&d, width_320, height_320, FilterType::Nearest);
    y.save(format!("{}/{}/{}_320.{}", PATH, user_id, &img_props.imgName, &img_props.imgExt))?;

    fs::remove_file(&img_props.tmpPath)?;

    Ok((height_720, height_320))
}

pub async fn upload_image(mut payload: Multipart) -> Result<HttpResponse, Error> {

    let mut image_data: Option<ImageProps> = None;
    let mut metadata: Option<RequestMetadata> = None;

    if let Some(mut field) = payload.try_next().await? {
        metadata = parse_metadata(&mut field).await?;
    }

    if let Some(mut field) = payload.try_next().await? {
        if let Some(me) = &metadata {
            me.create_dir()?;
            image_data = Some(create_url(&mut field, &me.userId)?);
            if let Some(url) = &image_data {
                let tmp_path = format!("{}/{}/{}.tmp.{}", PATH,&me.userId, &url.imgName, &url.imgExt);
                parse_image(&mut field, &tmp_path).await?
            }
        }
    }
    
    let mut image_dim: (u32, u32) = (0, 0);
    if let Some(paths) = &image_data {
        if let Some(me) = metadata {
            image_dim = crop_image(&paths, me.xAxis, me.yAxis, me.imgWidth, me.imgHeight, &me.userId)?;
        }
    }

    if image_data.is_none() {
        return Err(Error::from("Could not save image"));
    }

    let image_data = image_data.unwrap();

    Ok(HttpResponse::Ok().json(UploadResponse {
        imgName: image_data.imgName.clone(),
        imgExt: image_data.imgExt.clone(),
        imgMd: Some(image_dim.0),
        imgSm: Some(image_dim.1),
        imgLg: None,
    }))
}


pub async fn update_image(mut payload: Multipart) -> Result<HttpResponse, Error> {

    let mut image_data: Option<ImageProps> = None;
    let mut metadata: Option<RequestMetadataUpdate> = None;

    if let Some(mut field) = payload.try_next().await? {
        metadata = parse_metadata_update(&mut field).await?;
    }

    if let Some(mut field) = payload.try_next().await? {
        if let Some(me) = &metadata {
            image_data = Some(create_url(&mut field, &me.userId)?);
            if let Some(url) = &image_data {
                let tmp_path = format!("{}/{}/{}.tmp.{}", PATH,&me.userId, &url.imgName, &url.imgExt);
                parse_image(&mut field, &tmp_path).await?
            }
        }
    }
    
    let mut image_dim: (u32, u32) = (0, 0);
    if let Some(paths) = &image_data {
        if let Some(me) = &metadata {
            image_dim = crop_image(&paths, me.xAxis, me.yAxis, me.imgWidth, me.imgHeight, &me.userId)?;
        }
    }

    if image_data.is_none() {
        return Err(Error::from("Could not save image"));
    }

    let image_data = image_data.unwrap();

    if let Some(md) = &metadata {
        md.move_trash()?;
    }

    Ok(HttpResponse::Ok().json(UploadResponse {
        imgName: image_data.imgName.clone(),
        imgExt: image_data.imgExt.clone(),
        imgMd: Some(image_dim.0),
        imgSm: Some(image_dim.1),
        imgLg: None,
    }))
}
