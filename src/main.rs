extern crate clipboard;
extern crate notify;

use std::{fs::File, io::stdin, io::Read, sync::mpsc::channel, time::Duration};

use clap::Parser;
use clipboard::{ClipboardContext, ClipboardProvider};
use load_dotenv::load_dotenv;
use nanoid::nanoid;
use notify::{watcher, RecursiveMode, Watcher};
use notify_rust::Notification;
use s3::{bucket::Bucket, creds::Credentials, region::Region};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    watch: Option<String>,
}

load_dotenv!();

fn upload_image(buffer: &[u8]) -> Result<String, String> {
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

    let res = bucket.put_object_with_content_type(
        format!("{}{}.png", date.format("%Y/%m/"), nanoid),
        buffer,
        "image/png",
    );

    if res.is_err() {
        let error = res.err().unwrap();
        eprintln!("{:?}", error);
        return Err(error.to_string());
    }

    return Ok(format!(
        "{}{}{}.png",
        env!("S3_URL"),
        date.format("/%Y/%m/"),
        nanoid
    ));
}

fn main() {
    let args = Args::parse();

    if args.watch.is_none() {
        let mut buffer = Vec::new();
        stdin().read_to_end(&mut buffer).unwrap();
        let url = upload_image(&buffer).unwrap();
        println!("{{\"imageUrl\": \"{}\"}}", url);
        return;
    }

    let mut clipboard_ctx: ClipboardContext = ClipboardProvider::new().unwrap();

    let (tx, rx) = channel();

    let mut watcher = watcher(tx, Duration::from_millis(500)).unwrap();

    watcher
        .watch(args.watch.unwrap(), RecursiveMode::NonRecursive)
        .unwrap();

    loop {
        match rx.recv() {
            Ok(event) => {
                if let notify::DebouncedEvent::Create(path) = event {
                    let mut buffer = Vec::new();

                    {
                        let mut file = File::open(path).unwrap();
                        file.read_to_end(&mut buffer).unwrap();
                    }

                    let url = upload_image(&buffer).unwrap();

                    clipboard_ctx.set_contents(url.to_string()).unwrap();

                    Notification::new()
                        .summary("rs-uploader")
                        .body(format!("Uploaded, copied to clipboard: {}", &url).as_str())
                        .show()
                        .unwrap();
                }
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }
}
