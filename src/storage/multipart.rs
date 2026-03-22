use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipartUpload {
    pub upload_id: String,
    pub bucket: String,
    pub key: String,
    pub created_at: DateTime<Utc>,
    pub owner: String,
    pub content_type: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadPart {
    pub part_number: u32,
    pub object_hash: String,
    pub size: u64,
    pub etag: String,
}

pub struct MultipartUploadManager {
    uploads: HashMap<String, MultipartUpload>,
    parts: HashMap<String, HashMap<u32, UploadPart>>,
}

impl MultipartUploadManager {
    pub fn new() -> Self {
        Self {
            uploads: HashMap::new(),
            parts: HashMap::new(),
        }
    }

    pub fn create_upload(
        &mut self,
        bucket: String,
        key: String,
        owner: String,
    ) -> Result<MultipartUpload> {
        let upload_id = uuid::Uuid::new_v4().to_string();
        let upload = MultipartUpload {
            upload_id: upload_id.clone(),
            bucket,
            key,
            created_at: Utc::now(),
            owner,
            content_type: None,
            metadata: None,
        };

        self.uploads.insert(upload_id.clone(), upload);
        self.parts.insert(upload_id.clone(), HashMap::new());

        Ok(self.uploads.get(&upload_id).unwrap().clone())
    }

    pub fn get_upload(&self, upload_id: &str) -> Option<&MultipartUpload> {
        self.uploads.get(upload_id)
    }

    pub fn add_part(&mut self, upload_id: &str, part: UploadPart) -> Result<()> {
        if !self.uploads.contains_key(upload_id) {
            return Err(crate::error::Error::UploadNotFound(upload_id.to_string()));
        }

        self.parts
            .get_mut(upload_id)
            .unwrap()
            .insert(part.part_number, part);

        Ok(())
    }

    pub fn get_parts(&self, upload_id: &str) -> Result<Vec<UploadPart>> {
        let parts = self
            .parts
            .get(upload_id)
            .ok_or_else(|| crate::error::Error::UploadNotFound(upload_id.to_string()))?;

        let mut sorted_parts: Vec<_> = parts.values().cloned().collect();
        sorted_parts.sort_by_key(|p| p.part_number);

        Ok(sorted_parts)
    }

    pub fn complete_upload(&mut self, upload_id: &str) -> Result<Vec<UploadPart>> {
        let parts = self.get_parts(upload_id)?;

        // Verify parts are sequential
        for (i, part) in parts.iter().enumerate() {
            if part.part_number != (i + 1) as u32 {
                return Err(crate::error::Error::InvalidPartOrder);
            }
        }

        self.uploads.remove(upload_id);
        self.parts.remove(upload_id);

        Ok(parts)
    }

    pub fn abort_upload(&mut self, upload_id: &str) -> Result<()> {
        self.uploads
            .remove(upload_id)
            .ok_or_else(|| crate::error::Error::UploadNotFound(upload_id.to_string()))?;
        self.parts.remove(upload_id);
        Ok(())
    }

    pub fn list_uploads(&self, bucket: &str) -> Vec<MultipartUpload> {
        self.uploads
            .values()
            .filter(|u| u.bucket == bucket)
            .cloned()
            .collect()
    }
}
