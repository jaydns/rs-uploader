use std::{io::stdin, io::Read, process::exit};

use load_dotenv::load_dotenv;
use nanoid::nanoid;
use s3::{bucket::Bucket, creds::Credentials, region::Region};

load_dotenv!();

fn main() {
    // load image from stdin into `buffer`
    let mut buffer = Vec::new();
    stdin().lock().read_to_end(&mut buffer).ok();

    let nanoid = nanoid!();
    let date = chrono::Local::now();

    let bucket_name = env!("S3_BUCKET_NAME");

    let region = Region::Custom {
        region: env!("S3_REGION").to_owned(),
        endpoint: env!("S3_ENDPOINT").to_owned(),
    };

    let credentials = Credentials {
        access_key: Some(env!("S3_ACCESS_KEY").to_owned()),
        secret_key: Some(env!("S3_SECRET_KEY").to_owned()),
        security_token: None,
        session_token: None,
    };

    let bucket = Bucket::new(bucket_name, region, credentials).unwrap();

    let ok = bucket
        .put_object_with_content_type(
            format!("{}{}.png", date.format("%Y/%m/"), nanoid),
            &buffer,
            "image/png",
        )
        .is_ok();

    if !ok {
        eprintln!("Error uploading image");
        exit(1);
    }

    println!(
        "{{\"imageUrl\": \"{}{}{}.png\"}}",
        env!("S3_URL"),
        date.format("/%Y/%m/"),
        nanoid
    );
}
