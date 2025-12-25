use crate::types::CompressionType;
use oxide_core::Result;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Create an archive from paths.
pub fn create_archive<W: Write>(
    writer: W,
    paths: &[PathBuf],
    base_dir: &Path,
    compression: CompressionType,
) -> Result<()> {
    match compression {
        CompressionType::Zstd => {
            let mut encoder = zstd::stream::write::Encoder::new(writer, 3)
                .map_err(|e| oxide_core::Error::Internal(format!("Zstd init failed: {}", e)))?;
            {
                let mut builder = tar::Builder::new(&mut encoder);
                for p in paths {
                    let abs_path = if p.is_absolute() {
                        p.clone()
                    } else {
                        base_dir.join(p)
                    };
                    if abs_path.exists() {
                        // Compute relative path for the archive name
                        // If p is absolute, we might want to strip prefix?
                        // If p is relative, use it as is.
                        // The original logic used `p` (the requested path) as the name in archive.
                        let name = if p.is_absolute() {
                            p.strip_prefix(base_dir).unwrap_or(p)
                        } else {
                            p.as_path()
                        };

                        if abs_path.is_dir() {
                            builder.append_dir_all(name, &abs_path).map_err(|e| {
                                oxide_core::Error::Internal(format!("Failed to pack dir: {}", e))
                            })?;
                        } else {
                            builder
                                .append_path_with_name(&abs_path, name)
                                .map_err(|e| {
                                    oxide_core::Error::Internal(format!(
                                        "Failed to pack file: {}",
                                        e
                                    ))
                                })?;
                        }
                    }
                }
                builder.finish().map_err(|e| {
                    oxide_core::Error::Internal(format!("Failed to finish tar: {}", e))
                })?;
            }
            encoder
                .finish()
                .map_err(|e| oxide_core::Error::Internal(format!("Zstd finish failed: {}", e)))?;
        }
        CompressionType::None => {
            let mut builder = tar::Builder::new(writer);
            for p in paths {
                let abs_path = if p.is_absolute() {
                    p.clone()
                } else {
                    base_dir.join(p)
                };
                if abs_path.exists() {
                    let name = if p.is_absolute() {
                        p.strip_prefix(base_dir).unwrap_or(p)
                    } else {
                        p.as_path()
                    };

                    if abs_path.is_dir() {
                        builder.append_dir_all(name, &abs_path).map_err(|e| {
                            oxide_core::Error::Internal(format!("Failed to pack dir: {}", e))
                        })?;
                    } else {
                        builder
                            .append_path_with_name(&abs_path, name)
                            .map_err(|e| {
                                oxide_core::Error::Internal(format!("Failed to pack file: {}", e))
                            })?;
                    }
                }
            }
            builder
                .finish()
                .map_err(|e| oxide_core::Error::Internal(format!("Failed to finish tar: {}", e)))?;
        }
        _ => {
            return Err(oxide_core::Error::Internal(
                "Unsupported compression for archiving".into(),
            ));
        }
    }
    Ok(())
}

/// Extract an archive to a destination.
pub fn extract_archive<R: Read>(
    reader: R,
    dest: &Path,
    compression: CompressionType,
) -> Result<()> {
    match compression {
        CompressionType::Zstd => {
            let decoder = zstd::stream::read::Decoder::new(reader).map_err(|e| {
                oxide_core::Error::Internal(format!("Failed to create decoder: {}", e))
            })?;
            let mut archive = tar::Archive::new(decoder);
            archive.unpack(dest).map_err(|e| {
                oxide_core::Error::Internal(format!("Failed to unpack archive: {}", e))
            })?;
        }
        CompressionType::None => {
            let mut archive = tar::Archive::new(reader);
            archive.unpack(dest).map_err(|e| {
                oxide_core::Error::Internal(format!("Failed to unpack archive: {}", e))
            })?;
        }
        _ => {
            return Err(oxide_core::Error::Internal(
                "Unsupported compression for extraction".into(),
            ));
        }
    }
    Ok(())
}
