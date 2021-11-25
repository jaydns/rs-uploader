use chrono;
use clap::{load_yaml, App};
use s3::{bucket::Bucket, creds::Credentials, region::Region};
use std::{io::stdin, io::Read, process::exit};
use uuid::Uuid;

fn main() {
    // load image from stdin into `buffer`
    let mut buffer = Vec::new();
    stdin().lock().read_to_end(&mut buffer).ok();

    let uuid = Uuid::new_v4();
    let date = chrono::Local::now();

    let yaml = load_yaml!("cli.yaml");
    let matches = App::from(yaml).get_matches();

    let bucket_name = matches.value_of("bucket").unwrap();

    let region = Region::Custom {
        region: matches.value_of("region").unwrap().to_owned(),
        endpoint: matches.value_of("endpoint").unwrap().to_owned(),
    };

    let credentials = Credentials {
        access_key: Some(matches.value_of("access-key").unwrap().to_owned()),
        secret_key: Some(matches.value_of("secret-key").unwrap().to_owned()),
        security_token: None,
        session_token: None,
    };

    let bucket = Bucket::new(&bucket_name, region, credentials).unwrap();

    let ok = bucket
        .put_object_with_content_type(
            format!("{}{}.png", date.format("%Y/%m/"), uuid),
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
        matches.value_of("url").unwrap_or(""),
        date.format("/%Y/%m/"),
        uuid
    )
}
