use std::{collections::HashSet, fs::Metadata, hash::{DefaultHasher, Hash, Hasher}, io::Cursor, sync::LazyLock};

use async_trait::async_trait;
use camino::Utf8Path;
use chrono::{DateTime, Utc};
use image::{ImageFormat, ImageReader, imageops::FilterType};
use log::{debug, info};
use serde_json::Map;
use tokio::{fs::{self, File}, io::AsyncReadExt, task};

use crate::{app_config::get_default_chunk_directory, index::{ChunkFile, ChunkQueryResult, ChunkType, ChunkingIndexProvider, ORIGINAL_FILE_ATTR, embedding::siglip2_image_embedder::{Siglip2EmbeddedChunkFile, embed_chunk, embed_query}}, store::{ClearByFilter, Filter, FilterRelation, FilterValue, KeyedSequencedStore, QueryByFilter, QueryFull}};
pub struct BasicImageIndexProvider<S>
where
    S: KeyedSequencedStore<String, Siglip2EmbeddedChunkFile> +
        QueryFull<Siglip2EmbeddedChunkFile> +
        QueryByFilter<Siglip2EmbeddedChunkFile> +
        ClearByFilter<Siglip2EmbeddedChunkFile> +
        Send + Sync
{
    vector_store: S,
}

impl<S> BasicImageIndexProvider<S>
where
    S: KeyedSequencedStore<String, Siglip2EmbeddedChunkFile> +
        QueryFull<Siglip2EmbeddedChunkFile> +
        QueryByFilter<Siglip2EmbeddedChunkFile> +
        ClearByFilter<Siglip2EmbeddedChunkFile> +
        Send + Sync
{
    pub fn using(vector_store: S) -> Self {
        BasicImageIndexProvider { vector_store }
    }
}

#[async_trait]
impl<S> ChunkingIndexProvider for BasicImageIndexProvider<S>
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

    async fn index(&self, path: &Utf8Path) -> Result<(), anyhow::Error> {
        debug!("Basic Image Index Provider: Indexing file at path: {}", path);
        let mut file = File::open(path).await?;
        let metadata = file.metadata().await?;

        // If the store has indexed chunks for this file, then check the stored original_file_modified_date to
        // make sure it comes before the current file's modified date. If so, then make sure to clear the previously
        // stored chunks from the store before proceeding.
        let prev_indexed = self.vector_store.query_filter_n(
            vec![Filter {
                attribute: ORIGINAL_FILE_ATTR,
                filter: FilterValue::String(path.as_str()),
                relation: FilterRelation::Eq,
            }],
            1,
            0,
        ).await?;
        if prev_indexed.len() >= 1 {
            let last_modified: DateTime<Utc> = DateTime::from(metadata.modified()?);
            let stored_modified = prev_indexed.first().unwrap().chunkfile.original_file_modified_date;
            if last_modified <= stored_modified {
                info!("Attempted indexing on file: {} but the stored modified_date ({}) was equal to or later than the \
                    file's modified_date ({}). Ignoring.", path.to_string(), stored_modified, last_modified);
                return Ok(());
            }

            self.clear(path).await?;
        }

        // generate folder to store file chunks
        let chunk_data_dir = get_default_chunk_directory();
        let mut hasher = DefaultHasher::new();
        path.as_str().hash(&mut hasher);
        let filename_hash = hasher.finish().to_string();
        let chunk_out_dir = chunk_data_dir.join(filename_hash);
        fs::create_dir_all(&chunk_out_dir).await?;

        debug!("Basic Image Index Provider: Chunking file at path: {} to out_dir: {}", path, chunk_out_dir);
        let chunkfiles = chunk_image(path, &mut file, &metadata, &chunk_out_dir).await?;

        debug!("Basic Image Index Provider: Embedding chunks at dir: {}", chunk_out_dir);
        let mut embedded_chunkfiles = vec![];
        for chunkfile in chunkfiles {
            embedded_chunkfiles.push(embed_chunk(chunkfile).await?);
        }

        debug!("Basic Image Index Provider: Storing chunks and embeddings for path: {}", path);
        self.vector_store.put(embedded_chunkfiles).await.map_err(|e| e.into())
    }

    async fn clear(&self, path: &Utf8Path) -> Result<(), anyhow::Error> {
        debug!("Basic Image Index Provider: Clearing index of path: {}", path);
        // TODO: Where is deleting the chunkfile itself?
        self.vector_store.clear_filter(vec![Filter {
            attribute: ORIGINAL_FILE_ATTR,
            filter: FilterValue::String(path.as_str()),
            relation: FilterRelation::Eq,
        }]).await.map_err(|e| e.into())
    }

    async fn query_n(&self, str: &str, num_results: u32, offset: u32) -> Result<Vec<ChunkQueryResult>, anyhow::Error> {
        debug!("Basic Image Index Provider: Querying index of with params: {}, \
            num_results: {}, offset: {}", str, num_results, offset);
        debug!("Embedding query");
        let vec = embed_query(str).await?;

        let chunks = self.vector_store.query_full_n(vec, Some(str), vec![], num_results, offset).await?;

        let mut results = vec![];
        for chunk in chunks {
            if chunk.score < MIN_SCORE {
                // end results list at min_score
                break;
            }

            // normalize to 0-100
            let norm_score = ((chunk.score - MIN_SCORE) / (EXPECTED_MAX_SCORE - MIN_SCORE)) * 100.0;
            debug!("Basic Image Index Provider: Normalized result score: {}, orig_score: {}, \
                norm_score: {}", chunk.result.chunkfile.chunkfile, chunk.score, norm_score);
            results.push(ChunkQueryResult::new(chunk.result.chunkfile, norm_score));
        }
        Ok(results)
    }
}

// private functions and variables

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
        // TODO
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
const MIN_SCORE: f32 = 0.0;


async fn chunk_image(path: &Utf8Path, file: &mut File, metadata: &Metadata, out_dir: &Utf8Path)
    -> Result<Vec<ChunkFile>, anyhow::Error>
{
    let file_creation: DateTime<Utc> = DateTime::from(metadata.created()?);
    let file_modification: DateTime<Utc> = DateTime::from(metadata.modified()?);
    let file_length = metadata.len();
    let mut file_bytes: Vec<u8> = Vec::with_capacity(file_length as usize);
    file.read_to_end(&mut file_bytes).await?;

    let path = path.to_owned();
    let out_dir = out_dir.to_owned();
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
        let chunkfile_path = out_dir.join(chunk_filename);
        image.save_with_format(&chunkfile_path, ImageFormat::WebP)?;
        
        Ok::<Vec<ChunkFile>, anyhow::Error>(vec![ChunkFile {
            original_file: path,
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
    }).await??; // this is Result<Result<vec, closure_error>, tokio::task_error>

    Ok(chunk_files)
}