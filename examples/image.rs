// use crate::unique::time_uuid;
// use crate::error::Error;

use actix_web::{HttpResponse, web};
use actix_multipart::{Multipart, Field};
use futures::{StreamExt, TryStreamExt};
use std::{io::Write, path::Path};
use serde::{Deserialize, Serialize};
use image::{self, imageops::{self, FilterType}};

static PATH: &str = "/home/sankar/Pictures/sample.jpg";

fn main() -> std::io::Result<()> {
    let new_image = image::open(PATH).unwrap(); 
    Ok(())
}