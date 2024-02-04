use crate::DownloaderError;
use crate::Message;
use bytes::{Buf, Bytes};
use flate2::read::GzDecoder;
use iced::widget::button;
use iced::widget::horizontal_space;
use iced::widget::text;
use iced::widget::Container;
use iced::widget::Text;
use iced::widget::{container, row, Row};
use iced::Element;
use iced::Length;
use octocrab::models::repos::Release;
use regex::Regex;
use sha2::{Digest, Sha512};
use tar::Archive;

#[derive(Debug, Clone)]
pub enum proton_status {
    Installed,
    Uninstalled,
    Downloaded,
    Installing,
    Downloading,
}
#[derive(Debug, Clone)]
pub struct Proton {
    status: proton_status,
    release: Release,
    tarball: Option<Bytes>,
    tarball_url: String,
    checksum_url: String,
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

impl Proton {
    pub fn new(release: Release, installed: bool) -> Result<Proton, DownloaderError> {
        println!("creating new proton");
        let (tar_url, check_url) = get_proton_urls(&release)?;
        Ok(Self {
            release,
            status: if installed {
                proton_status::Installed
            } else {
                proton_status::Uninstalled
            },
            tarball: None,
            tarball_url: tar_url,
            checksum_url: check_url,
        })
    }

    pub fn get_tarball_url(&self) -> String {
        self.tarball_url.clone()
    }

    pub fn get_checksum_url(&self) -> String {
        self.checksum_url.clone()
    }

    pub async fn install(&mut self) -> Result<(), DownloaderError> {
        self.download().await?;

        self.extract()?;

        Ok(())
    }

    fn extract(&mut self) -> Result<(), DownloaderError> {
        if let Some(tarball) = &self.tarball {
            let tar = GzDecoder::new(tarball.clone().reader());
            let mut archive = Archive::new(tar);
            archive
                .unpack(".")
                .or(Err(DownloaderError::FilesystemError))?;
        }
        Ok(())
    }

    async fn download(&mut self) -> Result<String, DownloaderError> {
        self.status = proton_status::Downloading;
        let (tarball_url, checksum_url) = self.get_proton_urls(&self.release)?;

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

        let checksum: Vec<&str> = checksum.split(' ').collect();
        println!("checksum: {}", checksum[0]);

        let mut hasher = Sha512::new();

        hasher.update(&content);

        let result = hasher.finalize();
        let mut caluclated_checksum = String::new();

        for byte in result {
            caluclated_checksum.push_str(format!("{:02x}", byte).as_str());
        }

        if caluclated_checksum.eq(&checksum[0]) {
            println!("Checksum matches!!!");
        }

        println!("calculated checksum: {}", caluclated_checksum);

        self.tarball = Some(content);
        Ok(self.release.tag_name.clone())
    }

    pub fn get_status(&self) -> &proton_status {
        &self.status
    }

    pub fn get_name(&self) -> String {
        self.release.tag_name.clone()
    }
}
