use std::io::prelude::*;
use std::fs::File;
use futures::{channel::mpsc::{channel, Receiver}, SinkExt, StreamExt};
use std::path::Path;
use std::sync::Arc;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

fn async_watcher() -> notify::Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
    let (mut tx, rx) = channel(1);

    let watcher = RecommendedWatcher::new(move |res| {
        futures::executor::block_on(async {
            tx.send(res).await.unwrap();
        })
    }, Config::default())?;

    Ok((watcher, rx))
}

async fn async_watch<P: AsRef<Path>>(path: P, post_url: &str) -> notify::Result<()> {
    let client = Arc::new(reqwest::Client::new());
    let (mut watcher, mut rx) = async_watcher()?;

    watcher.watch(path.as_ref(), RecursiveMode::NonRecursive)?;

    while let Some(res) = rx.next().await {
        match res {
            Ok(event) => {
                match event {
                    Event {
                        paths,
                        ..
                    } => {
                        for path in paths {
                            if path.is_file() && path.extension().unwrap() == "png" {
                                println!("file created: {:?}", path);
                                std::thread::sleep(std::time::Duration::from_secs(1));

                                let mut file = File::open(&path)?;
                                let mut buf = Vec::new();
                                file.read_to_end(&mut buf)?;

                                let form = reqwest::multipart::Form::new()
                                    .part("file", reqwest::multipart::Part::bytes(buf).file_name("image.png").mime_str("image/png").unwrap());

                                let res = client.post(post_url)
                                    .multipart(form)
                                    .timeout(std::time::Duration::from_secs(2))
                                    .send();

                                let res = res.await;

                                match res {
                                    Ok(res) => {
                                        println!("status: {}", res.status());
                                        println!("text: {}", res.text().await.unwrap());
                                    }
                                    Err(e) => println!("error: {:?}", e),
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Enter dir: ");
    let mut dir = String::new();
    std::io::stdin().read_line(&mut dir).unwrap();
    if !Path::new(&dir.trim()).is_dir() {
        println!("{} is not a directory", dir.trim());
        return Ok(());
    }

    println!("Enter room id: ");
    let mut room_id = String::new();
    std::io::stdin().read_line(&mut room_id).unwrap();

    let post_url = format!("https://share.neos.love/v1/upload/{}", room_id.trim());
    println!("post url: {}", post_url);

    futures::executor::block_on(async {
        if let Err(e) = async_watch(&dir.trim(), &post_url).await {
            println!("error: {:?}", e)
        }
    });

    Ok(())
}