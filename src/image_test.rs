use crate::error::Error;

use std::fs::File;
use std::{future::Future, borrow::BorrowMut};
use std::task::Poll;
use futures::stream::Next;
use actix_web::{HttpResponse, web};
use actix_multipart::{Multipart, Field, MultipartError};
use futures::{StreamExt, TryStreamExt, FutureExt};
use std::{io::Write, path::Path};
use serde::{Deserialize, Serialize};
use image::{self, imageops::{self, FilterType}};

static PATH: &str = "/home/sankar/Pictures/sample.jpg";

struct MultipartInner<'a> {
    inner: &'a mut Multipart
}

impl<'a> Future for MultipartInner<'a> {
    type Output = Result<Option<Field>, MultipartError>;
    
    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        self.inner.try_next().poll_unpin(cx)
    }
}

// // NOTE: image wont upload from postman if you set Content-Type: multipart/form-data
// // Postman->Body->binary
pub async fn test_image(mut payload: Multipart) -> Result<HttpResponse, Error> {
    // let mut image_data: Option<(String, String, String)> = None;
    // let mut metadata: Option<MetaData> = None;

    let mut mi = MultipartInner { inner: &mut payload };

    while let Some(field) = mi.borrow_mut().await? {
        let content_disposition = field.content_disposition();
        let name = match content_disposition.get_name() {
            Some(name) => name,
            None => {
                return Err(Error::from("Cannot get name").into());
            }
        };
        match name {
            "metadata" => {
                // let x = get_value(&mut field).await?;
                // if let Some(x) = x {
                //     metadata = Some(serde_json::from_str(&x)?);
                // } 
            },
            "image" => {
                let content_type = field.content_disposition();
                let get_filename = content_type.get_filename();
                let get_name = content_type.get_name();
                println!("get_filename: {:?}, get_name: {:?}", get_filename, get_name);
            },
            _ => {}
        }
    }

    Ok(HttpResponse::Ok().body("Ok."))
}