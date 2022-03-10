use clap::Parser;
use clipboard::{ClipboardContext, ClipboardProvider};
use load_dotenv::load_dotenv;
use nanoid::nanoid;
use notify::{RecursiveMode, Watcher};
use notify_rust::Notification;
use s3::{bucket::Bucket, creds::Credentials, region::Region};
use std::{
    fs::{self, File},
    io::{self, Read},
    sync, thread,
    time::Duration,
};
use tray_item::TrayItem;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    watch: Option<String>,

    #[clap(short, long)]
    delete_after_upload: bool,
}

load_dotenv!();

fn upload_image(buffer: &[u8], file_ext: &str) -> Result<String, String> {
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
        format!("{}{}.{}", date.format("%Y/%m/"), nanoid, file_ext),
        buffer,
        &mime_guess::from_ext(file_ext).first().unwrap().to_string(),
    );

    if res.is_err() {
        let error = res.err().unwrap();
        eprintln!("{:?}", error);
        return Err(error.to_string());
    }

    return Ok(format!(
        "{}{}{}.{}",
        env!("S3_URL"),
        date.format("/%Y/%m/"),
        nanoid,
        file_ext
    ));
}

fn main() {
    let args = Args::parse();

    if args.watch.is_none() {
        let mut buffer = Vec::new();
        io::stdin().read_to_end(&mut buffer).unwrap();
        let url = upload_image(&buffer, "png").unwrap();
        println!("{{\"imageUrl\": \"{}\"}}", url);
        return;
    }

    let mut tray = TrayItem::new("rs-uploader", "").unwrap();

    let mut clipboard_ctx: ClipboardContext = ClipboardProvider::new().unwrap();

    let (tx, rx) = sync::mpsc::channel();

    let mut watcher = notify::watcher(tx, Duration::from_millis(500)).unwrap();

    watcher
        .watch(args.watch.unwrap(), RecursiveMode::NonRecursive)
        .unwrap();

    thread::spawn(move || loop {
        match rx.recv() {
            Ok(event) => {
                if let notify::DebouncedEvent::Write(path) = event {
                    let mut buffer = Vec::new();

                    {
                        let mut file = File::open(path.clone()).unwrap();
                        file.read_to_end(&mut buffer).unwrap();
                    }

                    let url =
                        upload_image(&buffer, path.clone().extension().unwrap().to_str().unwrap())
                            .unwrap();

                    clipboard_ctx.set_contents(url.to_string()).unwrap();

                    Notification::new()
                        .summary("rs-uploader")
                        .body(format!("Uploaded, copied to clipboard: {}", &url).as_str())
                        .show()
                        .unwrap();

                    if args.delete_after_upload {
                        fs::remove_file(path.clone()).unwrap();
                    }
                }
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    });

    let inner = tray.inner_mut();

    inner.add_quit_item("Quit");
    inner.display();
}
