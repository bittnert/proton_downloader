pub mod install;
pub mod installer;
pub mod proton;
use flate2::read::GzDecoder;
use iced::executor;
use iced::futures::StreamExt;
use iced::widget::{
    button, column, container, horizontal_space, row, text, vertical_space, Button, Column, Row,
    Text,
};
use iced::{Application, Command, Element, Length, Settings, Theme};
use octocrab::models::repos::Release;
use octocrab::{checks, Octocrab};
use proton::Proton;
use regex::Regex;
use sha2::{Digest, Sha512};
use std::collections::HashMap;
use std::fs::File;
use std::io::{copy, BufReader};
use std::str::FromStr;
use tar::Archive;
use tempfile::Builder;
use tokio::io;
use tokio_stream::wrappers::ReadDirStream;

struct Downloader {
    content: Vec<Release>,
    status: String,
    proton_list: HashMap<String, Proton>,
}

fn main() -> iced::Result {
    //println!("Hello, world!");
    Downloader::run(Settings {
        ..Settings::default()
    })
}

#[derive(Debug, Clone)]
pub enum Message {
    Refresh,
    ReleasesLoaded(Result<Vec<Release>, DownloaderError>),
    FilesystemLoaded(Result<Vec<String>, DownloaderError>),
    Install(String),
    Installed(Result<(), DownloaderError>),
}

impl Application for Downloader {
    type Message = Message;
    type Executor = executor::Default;
    type Theme = Theme;
    type Flags = ();

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        (
            Self {
                content: Vec::new(),
                status: String::from_str("Loading available releases").unwrap(),
                proton_list: HashMap::new(),
            },
            // Command::none(),
            Command::perform(get_releases(), Message::ReleasesLoaded),
        )
    }

    fn title(&self) -> String {
        String::from("Proton Downloader")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ReleasesLoaded(Ok(content)) => {
                self.content = content;
                self.status =
                    String::from_str("Checking installed packages in file system").unwrap();
                Command::perform(get_installed_wrapper(), Message::FilesystemLoaded)
                //let _installed = get_installed().unwrap();
            }
            Message::ReleasesLoaded(Err(_e)) => {
                self.status = String::from_str("Failed to load data").unwrap();
                Command::none()
            }
            Message::FilesystemLoaded(Ok(content)) => {
                self.status = String::from_str("Done").unwrap();
                for item in &self.content {
                    if let Ok(proton) = Proton::new(item.clone(), content.contains(&item.tag_name))
                    {
                        self.proton_list.insert(item.tag_name.clone(), proton);
                    }
                }
                Command::none()
            }
            Message::FilesystemLoaded(Err(_e)) => {
                self.status = String::from_str("Failed to check files in local folder").unwrap();
                Command::none()
            }
            Message::Refresh => {
                self.proton_list.clear();
                Command::none()
            }
            Message::Install(name) => {
                /*
                println!("Installing {}", name.tag_name);
                //println!("{:?}", name.assets);
                if let Some(url) = name.tarball_url {
                    println!("{:?}", url);
                    println!("{:?}", url.host());
                }*/
                //Command::none()
                Command::perform(item.install(), Message::Installed)
                //Command::perform(download_proton(name), Message::Installed)
            }
            Message::Installed(success) => {
                println!("Installed");
                match success {
                    Ok(_) => println!("Success"),
                    Err(e) => println!("error {:?}", e),
                }
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let controls: Row<'_, Message> = row![horizontal_space(Length::Fill), button("refresh")];

        let content = self.get_list();

        let status = text(self.status.clone());

        //let proton = proton_widget(String::from_str("Test name").unwrap(), false);

        container(column![
            //proton,
            controls,
            content,
            vertical_space(Length::Fill),
            status
        ])
        .into()
    }
}

impl Downloader {
    fn get_list(&self) -> Element<'_, Message> {
        let mut retval: Vec<Element<'_, Message>> = Vec::new();
        /*
                        for item in self.content.clone() {
                    retval.push(
                        container(row![
                            text(item.tag_name.clone()),
                            horizontal_space(Length::Fill),
                            button("install").on_press(Message::Install(item))
                        ])
                        .padding(1)
                        .into(),
                    );
                }
        */
        for item in self.proton_list {
            retval.push(
                container(row![
                    text(item.get_name()),
                    horizontal_space(Length::Fill),
                    button("Install").on_press(Message::Install(&item))
                ])
                .padding(1)
                .into(),
            );
        }
        container(Column::with_children(retval)).into()
    }
}

fn get_proton_urls(release: &Release) -> Result<(String, String), DownloaderError> {
    let mut checksum_url: String = String::new();
    let mut tarball_url: String = String::new();
    let mut checksum_found: bool = false;
    let mut tarball_found: bool = false;
    let checksum_re = Regex::new(r"\.sha512sum$").unwrap();
    let tarball_re = Regex::new(r"\.tar\.gz$").unwrap();

    for item in &release.assets {
        let browser_url = &item.browser_download_url;

        let url: String = match browser_url.host_str() {
            Some(host) => String::from(host),
            None => String::new(),
        };

        let url = format!("http://{}{}", url, browser_url.path());

        if checksum_re.is_match(browser_url.path()) {
            checksum_url = url;
            checksum_found = true;
        } else if tarball_re.is_match(browser_url.path()) {
            tarball_url = url;
            tarball_found = true;
        }
    }

    if tarball_found && checksum_found {
        Ok((tarball_url, checksum_url))
    } else {
        Err(DownloaderError::DownloadError)
    }
}

async fn download_proton(release: Release) -> Result<(String), DownloaderError> {
    let tmp_dir = Builder::new()
        .prefix("buffer")
        .tempdir()
        .or(Err(DownloaderError::DownloadError))?;

    let (tarball_url, checksum_url) = get_proton_urls(&release)?;

    println!("tarball url: {}", tarball_url);
    println!("checksum url: {}", checksum_url);

    let response = reqwest::get(&checksum_url)
        .await
        .or(Err(DownloaderError::DownloadError))?;

    let content = response
        .text()
        .await
        .or(Err(DownloaderError::DownloadError))?;

    let checksum = content;

    let response = reqwest::get(&tarball_url)
        .await
        .or(Err(DownloaderError::DownloadError))?;

    let content = response
        .bytes()
        .await
        .or(Err(DownloaderError::DownloadError))?;

    let tarball = content;

    let checksum: Vec<&str> = checksum.split(' ').collect();
    println!("checksum: {}", checksum[0]);

    let mut hasher = Sha512::new();

    hasher.update(tarball);

    let result = hasher.finalize();
    let mut caluclated_checksum = String::new();

    for byte in result {
        caluclated_checksum.push_str(format!("{:02x}", byte).as_str());
    }

    if caluclated_checksum.eq(&checksum[0]) {
        println!("Checksum matches!!!");
    }

    println!("calculated checksum: {}", caluclated_checksum);

    Ok(release.tag_name.clone())
}

async fn get_installed_wrapper() -> Result<Vec<String>, DownloaderError> {
    match get_installed().await {
        Ok(val) => Ok(val),
        Err(e) => Err(DownloaderError::FilesystemError),
    }
}

async fn get_installed() -> Result<Vec<String>, io::Error> {
    let mut home_path = match home::home_dir() {
        Some(path) => path,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Did not find home folder",
            ))
        }
    };

    if home_path.is_dir() {
        home_path.push(".steam/steam/compatibilitytools.d");
        let mut retval: Vec<String> = Vec::new();
        /*
                for entry in
                    tokio_stream::wrappers::ReadDirStream::new(tokio::fs::read_dir(home_path).await?)
                {
        */
        let mut stream =
            tokio_stream::wrappers::ReadDirStream::new(tokio::fs::read_dir(home_path).await?);
        while let Some(entry) = stream.next().await {
            let entry = entry?.path();
            if entry.is_dir() {
                if let Some(name) = entry.file_name() {
                    if let Some(name_str) = name.to_str() {
                        match String::from_str(name_str) {
                            Ok(m) => {
                                println!("found folder {}", m);
                                retval.push(m);
                            }
                            Err(_e) => {
                                return Err(io::Error::new(
                                    io::ErrorKind::Other,
                                    "Failed to convert to string",
                                ))
                            }
                        }
                    }
                }
            }
        }
        Ok(retval)
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "Did not find home folder",
        ))
    }
}

async fn download_release(name: String) {
    let octocrab = match Octocrab::builder().build() {
        Ok(obj) => obj,
        Err(_e) => return,
    };

    /*octocrab
    .repos("GloriousEggroll", "proton-ge-custom")
    .releases()
    .get_asset(asset_id);*/
}

async fn get_releases() -> Result<Vec<Release>, DownloaderError> {
    let octocrab = match Octocrab::builder().build() {
        Ok(obj) => obj,
        Err(_e) => return Err(DownloaderError::NetworkError),
    };

    let releases = match octocrab
        .repos("GloriousEggroll", "proton-ge-custom")
        .releases()
        .list()
        .per_page(1)
        .page(1 as u32)
        .send()
        .await
    {
        Ok(release) => release,
        Err(_e) => return Err(DownloaderError::NetworkError),
    };

    let mut retval: Vec<Release> = Vec::with_capacity(releases.items.len());

    for item in releases.items {
        retval.push(item);
    }

    Ok(retval)
}

#[derive(Debug, Clone)]
enum DownloaderError {
    NetworkError,
    FilesystemError,
    DownloadError,
}
