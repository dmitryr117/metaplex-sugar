use std::fs;

use async_trait::async_trait;
// use ini::ini;
use tokio::task::JoinHandle;

use crate::{
    common::*,
    config::*,
    upload::{
        assets::{AssetPair, DataType},
        uploader::{AssetInfo, ParallelUploader, Prepare},
    },
};

pub struct LocalMethod {
    pub directory: String,
    pub domain: String,
}

impl LocalMethod {
    pub async fn new(config_data: &ConfigData) -> Result<Self> {
        if let Some(config) = &config_data.local_config {
            let domain = if let Some(domain) = &config.domain {
                match url::Url::parse(&domain.to_string()) {
                    Ok(url) => url.to_string(),
                    Err(error) => {
                        return Err(anyhow!("Malformed domain URL ({})", error.to_string()))
                    }
                }
            } else {
                format!("http://localhost:8910")
            };
            Ok(Self {
                directory: config.directory.clone(),
                domain,
            })
        } else {
            Err(anyhow!("Missing LocalConfig value in config file."))
        }
    }

    async fn send(
        directory: String,
        domain: String,
        asset_info: AssetInfo,
    ) -> Result<(String, String)> {
        // Take care of any spaces in the directory path.
        let directory = directory.replace(' ', "_");
        let web_path = Path::new(&directory).join(&asset_info.name);
        let save_path = Path::new("/storage").join(&web_path);
        let save_path_str = save_path
            .to_str()
            .ok_or_else(|| anyhow!("Failed to convert localhost directory path to string."))?;
        let web_path_str = web_path
            .to_str()
            .ok_or_else(|| anyhow!("Failed to convert localhost directory path to string."))?;

        match asset_info.data_type {
            DataType::Image => {
                fs::copy(&asset_info.content, &save_path_str)?;
            }
            DataType::Metadata => {
                fs::write(&save_path_str, asset_info.content.into_bytes())?;
            }
            DataType::Animation => {
                fs::copy(&asset_info.content, &save_path_str)?;
            }
        };

        // save to localhost with a simple retry logic (mitigates dns lookup errors)

        let link = url::Url::parse(&domain)?.join(&web_path_str)?;

        Ok((asset_info.asset_id, link.to_string()))
    }
}

#[async_trait]
impl Prepare for LocalMethod {
    async fn prepare(
        &self,
        _sugar_config: &SugarConfig,
        _asset_pairs: &HashMap<isize, AssetPair>,
        _asset_indices: Vec<(DataType, &[isize])>,
    ) -> Result<()> {
        // nothing to do here
        Ok(())
    }
}

#[async_trait]
impl ParallelUploader for LocalMethod {
    fn upload_asset(&self, asset_info: AssetInfo) -> JoinHandle<Result<(String, String)>> {
        let directory = self.directory.clone();
        let domain = self.domain.clone();
        tokio::spawn(async move { LocalMethod::send(directory, domain, asset_info).await })
    }
}
