use crate::DownloaderError;
use bytes::{BufMut, BytesMut};
use futures_core::Stream;
use futures_util::StreamExt;
use reqwest::Response;
use sha2::{Digest, Sha512};

struct Installer {
    name: String,
    tarball_url: String,
    checksum_url: String,
    tarball: BytesMut,
    checksum: String,
    total_size: u64,
    downloaded: u64,
}

impl Installer {
    pub fn new(name: String, tarball_url: String, checksum_url: String) -> Installer {
        Self {
            name,
            tarball_url,
            checksum_url,
            tarball: BytesMut::new(),
            checksum: String::new(),
            total_size: 0,
            downloaded: 0,
        }
    }

    pub async fn start_download(&self) -> Result<&String, DownloaderError> {
        /*The checksum file is small enough and can directly be downloaded without reporting a progress */
        let response = reqwest::get(&self.checksum_url)
            .await
            .or(Err(DownloaderError::DownloadError))?;

        let content = response
            .text()
            .await
            .or(Err(DownloaderError::DownloadError))?;

        let checksum: Vec<&str> = content.split(' ').collect();
        self.checksum = String::from(checksum[0]);
        println!("checksum: {}", checksum[0]);

        let response = reqwest::get(&self.tarball_url)
            .await
            .or(Err(DownloaderError::DownloadError))?;

        self.total_size = response
            .content_length()
            .ok_or(DownloaderError::DownloadError)?;

        let tarball_stream = response.bytes_stream();

        while let Some(item) = tarball_stream.next().await {
            let chunk = item.or(Err(DownloaderError::DownloadError))?;
            self.tarball.put(chunk);
            self.downloaded += chunk.len() as u64;
        }

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
        Ok(&self.name)
    }

    pub async fn continue_download(&self) -> Result<(String, u32), DownloaderError> {
        Err(DownloaderError::DownloadError)
    }

    fn check_downloaded_file(&self) -> bool {
        true
    }
}
