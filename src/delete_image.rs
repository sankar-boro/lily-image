use crate::unique::time_uuid;
use crate::error::Error;

use actix_web::{HttpResponse, web};
use actix_multipart::{Multipart, Field};
use futures::{StreamExt, TryStreamExt};
use std::{io::Write, path::Path};
use serde::{Deserialize, Serialize};
use image::{self, imageops};

pub async fn delete_image(mut payload: Multipart) -> Result<HttpResponse, Error> {
    

}
