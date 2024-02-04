use crate::Proton;
use bytes::{Buf, BufMut, BytesMut};
use flate2::bufread::GzDecoder;
use iced::subscription;
use sha2::{Digest, Sha512};
use tar::Archive;

pub fn install(release: &Proton) -> iced::Subscription<(String, Progress)> {
    let id = release.get_name();
    subscription::unfold(
        id,
        State::Ready(release.get_checksum_url(), release.get_tarball_url()),
        move |state| start_installation(id, state),
    )
}

async fn start_installation(id: String, state: State) -> ((String, Progress), State) {
    match state {
        State::Ready(checksum_url, tarball_url) => {
            let response = reqwest::get(&checksum_url).await;

            match response {
                Ok(response) => {
                    let content = response.text().await;
                    if let Ok(checksum) = content {
                        let checksum: Vec<&str> = checksum.split(' ').collect();

                        let checksum = checksum[0];
                        return (
                            (id, Progress::Started),
                            State::TarballDownloadStarting {
                                tarball_url,
                                checksum: checksum.to_string(),
                            },
                        );
                    } else {
                        ((id, Progress::Errored), State::Finished)
                    }
                }
                Err(_) => ((id, Progress::Errored), State::Finished),
            }
        }
        State::TarballDownloadStarting {
            tarball_url,
            checksum,
        } => {
            let response = reqwest::get(&tarball_url).await;

            match response {
                Ok(response) => {
                    if let Some(total) = response.content_length() {
                        (
                            (id, Progress::Started),
                            State::TarballDownloading {
                                response,
                                total,
                                downloaded: 0,
                                checksum,
                                tarball: BytesMut::new(),
                            },
                        )
                    } else {
                        ((id, Progress::Errored), State::Finished)
                    }
                }
                Err(_) => ((id, Progress::Errored), State::Finished),
            }
        }
        State::TarballDownloading {
            mut response,
            total,
            downloaded,
            checksum,
            mut tarball,
        } => match response.chunk().await {
            Ok(Some(chunk)) => {
                tarball.put(chunk);
                let percentage = (tarball.len() as f32 / total as f32) * 100.0;

                (
                    (id, Progress::Advanced(percentage)),
                    State::TarballDownloading {
                        response,
                        tarball,
                        total,
                        downloaded,
                        checksum,
                    },
                )
            }
            Ok(None) => (
                (id, Progress::CheckIntegrity),
                State::CheckIntegrity { checksum, tarball },
            ),
            Err(_) => ((id, Progress::Errored), State::Finished),
        },
        State::CheckIntegrity { checksum, tarball } => {
            let mut hasher = Sha512::new();

            hasher.update(&tarball);

            let result = hasher.finalize();
            let mut caluclated_checksum = String::new();

            for byte in result {
                caluclated_checksum.push_str(format!("{:02x}", byte).as_str());
            }

            if caluclated_checksum.eq(&checksum) {
                ((id, Progress::Installing), State::Install { tarball })
            } else {
                ((id, Progress::Errored), State::Finished)
            }
        }
        State::Install { tarball } => {
            let tar = GzDecoder::new(tarball.clone().reader());
            let mut archive = Archive::new(tar);
            match archive.unpack(".") {
                Ok(_) => ((id, Progress::Finished), State::Finished),
                Err(_) => ((id, Progress::Errored), State::Finished),
            }
        }
        State::Finished => todo!(),
    }
}

pub enum Progress {
    Started,
    Advanced(f32),
    CheckIntegrity,
    Installing,
    Finished,
    Errored,
}

pub enum State {
    Ready(String, String),
    TarballDownloadStarting {
        tarball_url: String,
        checksum: String,
    },
    TarballDownloading {
        response: reqwest::Response,
        tarball: BytesMut,
        total: u64,
        downloaded: u64,
        checksum: String,
    },
    CheckIntegrity {
        checksum: String,
        tarball: BytesMut,
    },
    Install {
        tarball: BytesMut,
    },
    Finished,
}
