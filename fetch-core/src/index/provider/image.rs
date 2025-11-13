use std::{collections::HashSet, fs::Metadata, hash::{DefaultHasher, Hash, Hasher}, io::Cursor, sync::{Arc, LazyLock}};

use async_trait::async_trait;
use camino::Utf8Path;
use chrono::{DateTime, Utc};
use image::{DynamicImage, ImageFormat, ImageReader, RgbaImage, imageops::FilterType};
use log::debug;
use psd::{Psd, PsdLayer};
use serde_json::Map;
use tokio::{fs::{self, File}, io::AsyncReadExt, task};

use crate::{app_config::get_default_chunk_directory, index::{ChunkFile, ChunkType, embedding::siglip2::{Siglip2EmbeddedChunkFile, embed_chunk, embed_query}, provider::{ChunkQueryResult, ChunkingIndexProvider, IndexProviderError, IndexProviderErrorType}}, store::{ClearByFilter, Filter, FilterRelation, FilterValue, KeyedSequencedStore, QueryByFilter, QueryFull}};

pub struct ImageIndexProvider<S>
where
    S: KeyedSequencedStore<String, Siglip2EmbeddedChunkFile> +
        QueryFull<Siglip2EmbeddedChunkFile> +
        QueryByFilter<Siglip2EmbeddedChunkFile> +
        ClearByFilter<Siglip2EmbeddedChunkFile> +
        Send + Sync
{
    vector_store: Arc<S>,
}

impl<S> ImageIndexProvider<S>
where
    S: KeyedSequencedStore<String, Siglip2EmbeddedChunkFile> +
        QueryFull<Siglip2EmbeddedChunkFile> +
        QueryByFilter<Siglip2EmbeddedChunkFile> +
        ClearByFilter<Siglip2EmbeddedChunkFile> +
        Send + Sync
{
    pub fn using(vector_store: Arc<S>) -> Self {
        ImageIndexProvider { vector_store }
    }
}

#[async_trait]
impl<S> ChunkingIndexProvider for ImageIndexProvider<S>
where
    S: KeyedSequencedStore<String, Siglip2EmbeddedChunkFile> +
        QueryFull<Siglip2EmbeddedChunkFile> +
        QueryByFilter<Siglip2EmbeddedChunkFile> +
        ClearByFilter<Siglip2EmbeddedChunkFile> +
        Send + Sync
{
    fn provides_indexing_for_extension(&self, ext: &str) -> bool {
        EXTENSIONS.contains(ext)
    }

    async fn index(&self, path: &Utf8Path, opt_modified: Option<DateTime<Utc>>) -> Result<(), IndexProviderError> {
        debug!("Image Index Provider: Indexing file at path: {}", path);
        let mut file = File::open(path).await
            .map_err(|e| IndexProviderError {
                provider_name: PROVIDER_NAME.to_string(),
                r#type: IndexProviderErrorType::IO {
                    path: path.to_string(),
                    source: e.into(),
                }
            })?;
        let metadata = file.metadata().await
            .map_err(|e| IndexProviderError {
                provider_name: PROVIDER_NAME.to_string(),
                r#type: IndexProviderErrorType::IO {
                    path: path.to_string(),
                    source: e.into(),
                }
            })?;

        // If the store has indexed chunks for this file, then check the stored original_file_modified_date to
        // make sure it comes before the current file's modified date. If so, then make sure to clear the previously
        // stored chunks from the store before proceeding.
        let prev_indexed = self.vector_store.query_filter_n(
            &[Filter {
                attribute: ChunkFile::ORIGINAL_FILE_ATTR,
                filter: FilterValue::String(path.as_str()),
                relation: FilterRelation::Eq,
            }],
            1,
            0,
        ).await.map_err(|e| IndexProviderError {
            provider_name: PROVIDER_NAME.to_string(),
            r#type: IndexProviderErrorType::Store {
                operation: "query_filter_n",
                source: e.into()
            }
        })?;
        if !prev_indexed.is_empty() {
            let last_modified: DateTime<Utc> = opt_modified.unwrap_or(DateTime::from(metadata.modified()
                .expect("Modified date not available on platform")));
            let stored_modified = prev_indexed.first().unwrap().chunkfile.original_file_modified_date;
            if last_modified.timestamp_millis() <= stored_modified.timestamp_millis() {
                return Err(IndexProviderError {
                    provider_name: PROVIDER_NAME.to_string(),
                    r#type: IndexProviderErrorType::Sequencing {
                        provided_datetime: last_modified,
                        stored_datetime: stored_modified,
                    },
                });
            }

            self.clear(path, Some(last_modified)).await?;
        }

        // generate folder to store file chunks
        let chunk_data_dir = get_default_chunk_directory();
        let mut hasher = DefaultHasher::new();
        path.as_str().hash(&mut hasher);
        let filename_hash = hasher.finish().to_string();
        let chunk_out_dir = chunk_data_dir.join(filename_hash);
        fs::create_dir_all(&chunk_out_dir).await.map_err(|e| IndexProviderError {
            provider_name: PROVIDER_NAME.to_string(),
            r#type: IndexProviderErrorType::IO {
                path: path.to_string(),
                source: e.into(),
            }
        })?;

        debug!("Image Index Provider: Chunking file at path: {} to out_dir: {}", path, chunk_out_dir);
        let chunkfiles = if path.extension() == Some("psd") {
            chunk_psd(path, &mut file, &metadata, &chunk_out_dir).await?
        } else {
            chunk_image(path, &mut file, &metadata, &chunk_out_dir).await?
        };

        debug!("Image Index Provider: Embedding chunks at dir: {}", chunk_out_dir);
        let mut embedded_chunkfiles = vec![];
        for chunkfile in chunkfiles {
            embedded_chunkfiles.push(embed_chunk(chunkfile).await.map_err(|e| IndexProviderError {
                provider_name: PROVIDER_NAME.to_string(),
                r#type: IndexProviderErrorType::Embedding { source: e },
            })?);
        }

        debug!("Image Index Provider: Storing chunks and embeddings for path: {}", path);
        self.vector_store.put(embedded_chunkfiles).await.map_err(|e| IndexProviderError {
            provider_name: PROVIDER_NAME.to_string(),
            r#type: IndexProviderErrorType::Store {
                operation: "put",
                source: e.into(),
            }
        })
    }

    async fn clear(&self, path: &Utf8Path, opt_modified: Option<DateTime<Utc>>) -> Result<(), IndexProviderError> {
        debug!("Image Index Provider: Clearing index of path: {}", path);
        // TODO: Where is deleting the chunkfile itself?

        let mut filters = vec![Filter {
            attribute: ChunkFile::ORIGINAL_FILE_ATTR,
            filter: FilterValue::String(path.as_str()),
            relation: FilterRelation::Eq,
        }];
        if let Some(modified_dt) = &opt_modified {
            filters.push(Filter {
                attribute: ChunkFile::FILE_MODIFIED_DATE_ATTR,
                filter: FilterValue::DateTime(modified_dt),
                relation: FilterRelation::Eq,
            });
        }

        self.vector_store.clear_filter(&filters).await.map_err(|e| IndexProviderError {
            provider_name: PROVIDER_NAME.to_string(),
            r#type: IndexProviderErrorType::Store {
                operation: "clear by filter",
                source: e.into(),
            }
        })
    }

    async fn query_n(&self, str: &str, num_results: u32, offset: u32) -> Result<Vec<ChunkQueryResult>, IndexProviderError> {
        debug!("Image Index Provider: Querying index of with params: {}, \
            num_results: {}, offset: {}", str, num_results, offset);
        debug!("Image Index Provider: Embedding query");
        let vec = embed_query(str).await.map_err(|e| IndexProviderError {
            provider_name: PROVIDER_NAME.to_string(),
            r#type: IndexProviderErrorType::Embedding { source: e },
        })?;

        let chunks = self.vector_store.query_full_n(
            vec,
            Some(str),
            &[],
            num_results,
            offset
        ).await.map_err(|e| IndexProviderError {
            provider_name: PROVIDER_NAME.to_string(),
            r#type: IndexProviderErrorType::Store {
                operation: "query full",
                source: e,
            }
        })?;

        let mut results = vec![];
        for chunk in chunks {
            if chunk.score < MIN_SCORE {
                // end results list at min_score
                break;
            }

            // normalize to 0-100
            let norm_score = ((chunk.score - MIN_SCORE) / (EXPECTED_MAX_SCORE - MIN_SCORE)) * 100.0;
            debug!("Image Index Provider: Normalized result score: {}, orig_score: {}, \
                norm_score: {}", chunk.result.chunkfile.chunkfile, chunk.score, norm_score);
            results.push(ChunkQueryResult::new(chunk.result.chunkfile, norm_score));
        }
        Ok(results)
    }
}

// private functions and variables

const PROVIDER_NAME: &str = "ImageIndexProvider";

static EXTENSIONS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut set = HashSet::new();
    // file types supported by image crate
    set.insert("avif");
    set.insert("bmp");
    set.insert("dds");
    set.insert("ff");
    set.insert("hdr");
    set.insert("ico");
    set.insert("jpg");
    set.insert("jpeg");
    set.insert("exr");
    set.insert("png");
    set.insert("pnm");
    set.insert("qoi");
    set.insert("tga");
    set.insert("tif");
    set.insert("tiff");
    set.insert("webp");
    // psd files, using psd crate
    #[cfg(feature = "psd")]
    {
        set.insert("psd");
    }
    // Add more extensions and their corresponding preview calculation functions here
    set
});

const CHUNK_MAX_SIDE: u32 = 512;
const IMAGE_CHUNK_EXTENSION: &str = "webp";
const IMAGE_CHUNK_CHANNEL: &str = "base";
const IMAGE_CHUNK_SEQUENCE_ID: f32 = 0.0;
const IMAGE_CHUNK_LENGTH: f32 = 1.0;

// These constants must be tuned to the hybrid query results of lance FTS and siglip2 vector cosine similarity reranking
// TODO: tune
const EXPECTED_MAX_SCORE: f32 = 1.0;
const MIN_SCORE: f32 = 0.015;

async fn chunk_image(path: &Utf8Path, file: &mut File, metadata: &Metadata, out_dir: &Utf8Path)
    -> Result<Vec<ChunkFile>, IndexProviderError>
{
    let file_creation: DateTime<Utc> = DateTime::from(metadata.created()
        .expect("Created date not available on platform"));
    let file_modification: DateTime<Utc> = DateTime::from(metadata.modified()
        .expect("Modified date not available on platform"));
    let file_length = metadata.len();
    let mut file_bytes: Vec<u8> = Vec::with_capacity(file_length as usize);
    file.read_to_end(&mut file_bytes).await.map_err(|e| IndexProviderError {
        provider_name: PROVIDER_NAME.to_string(),
        r#type: IndexProviderErrorType::IO {
            path: path.to_string(),
            source: e.into(),
        }
    })?;

    let path_clone = path.to_owned();
    let out_dir_clone = out_dir.to_owned();
    let chunk_files = task::spawn_blocking(move || {
        let image = ImageReader::new(Cursor::new(file_bytes))
            .with_guessed_format()?
            .decode()?;

        // TODO: chunk large images into multiple chunks? with separate focus window to total window?
        // or really long aspect ratios?

        let image = image.resize(
            CHUNK_MAX_SIDE,
            CHUNK_MAX_SIDE,
            FilterType::Triangle,
        );

        let chunk_filename = format!("{}-{}.{}", IMAGE_CHUNK_CHANNEL, IMAGE_CHUNK_SEQUENCE_ID,
            IMAGE_CHUNK_EXTENSION);
        let chunkfile_path = out_dir_clone.join(chunk_filename);
        image.save_with_format(&chunkfile_path, ImageFormat::WebP)?;
        
        Ok::<Vec<ChunkFile>, anyhow::Error>(vec![ChunkFile {
            original_file: path_clone,
            chunk_channel: IMAGE_CHUNK_CHANNEL.to_owned(),
            chunk_sequence_id: IMAGE_CHUNK_SEQUENCE_ID,
            chunkfile: chunkfile_path,
            chunk_type: ChunkType::Image,
            chunk_length: IMAGE_CHUNK_LENGTH,
            original_file_creation_date: file_creation,
            original_file_modified_date: file_modification,
            original_file_size: file_length,
            original_file_tags: Map::new(),
        }])
    }).await // this is Result<Result<vec, closure_error>, tokio::task_error>
    .map_err(|e| IndexProviderError {
        provider_name: PROVIDER_NAME.to_string(),
        r#type: IndexProviderErrorType::Unknown {
            msg: "Failed to join image chunking task",
            source: e.into(),
        },
    })?
    .map_err(|e| IndexProviderError {
        provider_name: PROVIDER_NAME.to_string(),
        r#type: IndexProviderErrorType::Chunking {
            path: path.to_string(),
            source: e,
        },
    })?;

    Ok(chunk_files)
}

async fn chunk_psd(path: &Utf8Path, file: &mut File, metadata: &Metadata, out_dir: &Utf8Path)
    -> Result<Vec<ChunkFile>, IndexProviderError>
{
    let file_creation: DateTime<Utc> = DateTime::from(metadata.created()
        .expect("Created date not available on platform"));
    let file_modification: DateTime<Utc> = DateTime::from(metadata.modified()
        .expect("Modified date not available on platform"));
    let file_length = metadata.len();
    let mut file_bytes: Vec<u8> = Vec::with_capacity(file_length as usize);
    file.read_to_end(&mut file_bytes).await.map_err(|e| IndexProviderError {
        provider_name: PROVIDER_NAME.to_string(),
        r#type: IndexProviderErrorType::IO {
            path: path.to_string(),
            source: e.into(),
        }
    })?;

    let path_clone = path.to_owned();
    let out_dir_clone = out_dir.to_owned();
    let chunk_files = task::spawn_blocking(move || {
        let psd = Psd::from_bytes(&file_bytes)?;

        let width = psd.width();
        let height = psd.height();
        // return true for all layers for now, include all layers in the flattened image
        let filter = &|(_i, _l): (usize, &PsdLayer)| true;
        let flattened_bytes = psd.flatten_layers_rgba(filter)?;

        let image = DynamicImage::from(RgbaImage::from_raw(width, height, flattened_bytes).unwrap());

        let image = image.resize(
            CHUNK_MAX_SIDE,
            CHUNK_MAX_SIDE,
            FilterType::Triangle,
        );

        let chunk_filename = format!("{}-{}.{}", IMAGE_CHUNK_CHANNEL, IMAGE_CHUNK_SEQUENCE_ID,
            IMAGE_CHUNK_EXTENSION);
        let chunkfile_path = out_dir_clone.join(chunk_filename);
        image.save_with_format(&chunkfile_path, ImageFormat::WebP)?;
        
        Ok::<Vec<ChunkFile>, anyhow::Error>(vec![ChunkFile {
            original_file: path_clone,
            chunk_channel: IMAGE_CHUNK_CHANNEL.to_owned(),
            chunk_sequence_id: IMAGE_CHUNK_SEQUENCE_ID,
            chunkfile: chunkfile_path,
            chunk_type: ChunkType::Image,
            chunk_length: IMAGE_CHUNK_LENGTH,
            original_file_creation_date: file_creation,
            original_file_modified_date: file_modification,
            original_file_size: file_length,
            original_file_tags: Map::new(),
        }])
    }).await // this is Result<Result<vec, closure_error>, tokio::task_error>
    .map_err(|e| IndexProviderError {
        provider_name: PROVIDER_NAME.to_string(),
        r#type: IndexProviderErrorType::Unknown {
            msg: "Failed to join image chunking task",
            source: e.into(),
        },
    })?
    .map_err(|e| IndexProviderError {
        provider_name: PROVIDER_NAME.to_string(),
        r#type: IndexProviderErrorType::Chunking {
            path: path.to_string(),
            source: e,
        },
    })?;

    Ok(chunk_files)
}