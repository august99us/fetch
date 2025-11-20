use std::{fs::Metadata, sync::Arc};

use async_trait::async_trait;
use camino::Utf8Path;
use chrono::{DateTime, Utc};
use futures::TryFutureExt;
use image::{DynamicImage, ImageFormat, imageops::FilterType};
use log::{debug, info};
use pdfium_render::prelude::{PdfPage, PdfPageObjectsCommon};
use serde_json::Map;
use tokio::{fs::File, join, task};
use tokio_util::io::SyncIoBridge;

use crate::{environment::get_pdfium, index::{ChunkFile, ChunkType, embedding::{embeddinggemma::{self, EmbeddingGemmaEmbeddedChunkFile}, siglip2::{self, Siglip2EmbeddedChunkFile}}, provider::{ChunkQueryResult, ChunkingIndexProvider, IndexProviderError, IndexProviderErrorType, clear_chunkfiles, create_chunkfile_dir}}, store::{ClearByFilter, Filter, FilterRelation, FilterValue, KeyedSequencedData, KeyedSequencedStore, QueryByFilter, QueryFull}};

pub struct PdfIndexProvider<TS, IS>
where
    TS: KeyedSequencedStore<String, EmbeddingGemmaEmbeddedChunkFile> +
        QueryFull<EmbeddingGemmaEmbeddedChunkFile> +
        QueryByFilter<EmbeddingGemmaEmbeddedChunkFile> +
        ClearByFilter<EmbeddingGemmaEmbeddedChunkFile> +
        Send + Sync,
    IS: KeyedSequencedStore<String, Siglip2EmbeddedChunkFile> +
        QueryFull<Siglip2EmbeddedChunkFile> +
        QueryByFilter<Siglip2EmbeddedChunkFile> +
        ClearByFilter<Siglip2EmbeddedChunkFile> +
        Send + Sync
{
    text_store: Arc<TS>,
    image_store: Arc<IS>,
}

impl<TS, IS> PdfIndexProvider<TS, IS>
where
    TS: KeyedSequencedStore<String, EmbeddingGemmaEmbeddedChunkFile> +
        QueryFull<EmbeddingGemmaEmbeddedChunkFile> +
        QueryByFilter<EmbeddingGemmaEmbeddedChunkFile> +
        ClearByFilter<EmbeddingGemmaEmbeddedChunkFile> +
        Send + Sync,
    IS: KeyedSequencedStore<String, Siglip2EmbeddedChunkFile> +
        QueryFull<Siglip2EmbeddedChunkFile> +
        QueryByFilter<Siglip2EmbeddedChunkFile> +
        ClearByFilter<Siglip2EmbeddedChunkFile> +
        Send + Sync
{
    pub fn using(text_store: Arc<TS>, image_store: Arc<IS>) -> Self {
        PdfIndexProvider { text_store, image_store }
    }
}

#[async_trait]
impl<TS, IS> ChunkingIndexProvider for PdfIndexProvider<TS, IS>
where
    TS: KeyedSequencedStore<String, EmbeddingGemmaEmbeddedChunkFile> +
        QueryFull<EmbeddingGemmaEmbeddedChunkFile> +
        QueryByFilter<EmbeddingGemmaEmbeddedChunkFile> +
        ClearByFilter<EmbeddingGemmaEmbeddedChunkFile> +
        Send + Sync,
    IS: KeyedSequencedStore<String, Siglip2EmbeddedChunkFile> +
        QueryFull<Siglip2EmbeddedChunkFile> +
        QueryByFilter<Siglip2EmbeddedChunkFile> +
        ClearByFilter<Siglip2EmbeddedChunkFile> +
        Send + Sync
{
    fn provides_indexing_for_extension(&self, ext: &str) -> bool {
        ext.eq("pdf")
    }

    async fn index(&self, path: &Utf8Path, opt_modified: Option<DateTime<Utc>>) -> Result<(), IndexProviderError> {
        debug!("PDF Index Provider: Indexing file at path: {}", path);
        let file = File::open(path).await
            .map_err(|e| IndexProviderError {
                provider_name: PROVIDER_NAME.to_string(),
                r#type: IndexProviderErrorType::IO {
                    path: path.to_string(),
                    source: e.into(),
                },
            })?;
        let metadata = file.metadata().await
            .map_err(|e| IndexProviderError {
                provider_name: PROVIDER_NAME.to_string(),
                r#type: IndexProviderErrorType::IO {
                    path: path.to_string(),
                    source: e.into(),
                },
            })?;

        // If the store has indexed chunks for this file, then check the stored original_file_modified_date to
        // make sure it comes before the current file's modified date. If so, then make sure to clear the previously
        // stored chunks from the store before proceeding.
        let discover_filter = &[Filter {
            attribute: ChunkFile::ORIGINAL_FILE_ATTR,
            filter: FilterValue::String(path.as_str()),
            relation: FilterRelation::Eq,
        }];
        let discovered_chunks: (Option<ChunkFile>, Option<ChunkFile>) = futures::try_join!(
            self.text_store.query_filter_n(discover_filter, 1, 0)
                .map_ok(|vec| vec.into_iter().map(|ec| ec.chunkfile).next()),
            self.image_store.query_filter_n(discover_filter, 1, 0)
                .map_ok(|vec| vec.into_iter().map(|ec| ec.chunkfile).next()),
        ).map_err(|e| IndexProviderError {
            provider_name: PROVIDER_NAME.to_string(),
            r#type: IndexProviderErrorType::Store {
                operation: "query filter",
                source: e.into(),
            }
        })?;

        if let Some(discovered_chunk) = discovered_chunks.0.or(discovered_chunks.1) {
            let last_modified: DateTime<Utc> = opt_modified.unwrap_or(DateTime::from(metadata.modified()
                .expect("File modified datetime not available on this platform")));
            let stored_modified = discovered_chunk.original_file_modified_date;
            if last_modified.timestamp_millis() <= stored_modified.timestamp_millis() {
                info!("Attempted indexing on file: {} but the stored modified_date ({}) was equal to or later than the \
                    file's modified_date ({}). Ignoring.", path, stored_modified, last_modified);
                return Ok(());
            }

            self.clear(path, Some(last_modified)).await?;
        }

        // generate folder to store file chunks
        let chunk_out_dir = create_chunkfile_dir(path).await
            .map_err(|e| IndexProviderError {
                provider_name: PROVIDER_NAME.to_string(),
                r#type: IndexProviderErrorType::IO {
                    path: path.to_string(),
                    source: e.into(),
                }
            })?;

        debug!("PDF Index Provider: Chunking file at path: {} to out_dir: {}", path, chunk_out_dir);
        let chunkfiles = chunk_pdf(path, file, metadata, &chunk_out_dir).await
            .map_err(|e| IndexProviderError {
                provider_name: PROVIDER_NAME.to_owned(),
                r#type: IndexProviderErrorType::Chunking {
                    path: path.to_string(),
                    source: e,
                }
            })?;

        debug!("PDF Index Provider: Embedding chunks at dir: {}", chunk_out_dir);
        let mut embedded_text_chunkfiles = vec![];
        let mut embedded_image_chunkfiles = vec![];
        for chunkfile in chunkfiles {
            debug!("Pdf Index Provider: Embedding chunk with id: {}", chunkfile.get_key());
            match chunkfile.chunk_type {
                ChunkType::Text => {
                    embedded_text_chunkfiles.push(embeddinggemma::embed_chunk(chunkfile).await
                        .map_err(|e| IndexProviderError {
                            provider_name: PROVIDER_NAME.to_string(),
                            r#type: IndexProviderErrorType::Embedding { source: e },
                        })?);
                },
                ChunkType::Image => {
                    embedded_image_chunkfiles.push(siglip2::embed_chunk(chunkfile).await
                        .map_err(|e| IndexProviderError {
                            provider_name: PROVIDER_NAME.to_string(),
                            r#type: IndexProviderErrorType::Embedding { source: e },
                        })?);
                }
                _ => unreachable!("PDF chunker should only produce text and image chunks"),
            }
        }

        debug!("PDF Index Provider: Storing chunks and embeddings for path: {}", path);
        futures::try_join!(
            self.text_store.put(embedded_text_chunkfiles),
            self.image_store.put(embedded_image_chunkfiles),
        ).map_err(|e| IndexProviderError {
            provider_name: PROVIDER_NAME.to_string(),
            r#type: IndexProviderErrorType::Store {
                operation: "put",
                source: e.into(),
            }
        })?;

        Ok(())
    }

    async fn clear(&self, path: &Utf8Path, opt_modified: Option<DateTime<Utc>>) -> Result<(), IndexProviderError> {
        debug!("PDF Index Provider: Clearing index of path: {}", path);

        // TODO: This chunkfile clearing does not care about opt_modified.
        //       Maybe make this better in the future?
        clear_chunkfiles(path).await.map_err(|e| IndexProviderError {
            provider_name: PROVIDER_NAME.to_string(),
            r#type: IndexProviderErrorType::IO { path: path.to_string(), source: e.into() }
        })?;

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
        // TODO: Consider better error handling than concurrent try_join!
        // try_join! does not expose both errors, only one, and will cancel the other
        // future if one fails (although the clear operation might already be in progress)
        futures::try_join!(
            self.text_store.clear_filter(&filters),
            self.image_store.clear_filter(&filters)
        ).map_err(|e| IndexProviderError {
            provider_name: PROVIDER_NAME.to_string(),
            r#type: IndexProviderErrorType::Store {
                operation: "clear filter",
                source: e.into(),
            }
        })?;

        Ok(())
    }

    async fn query_n(&self, str: &str, num_results: u32, offset: u32) -> Result<Vec<ChunkQueryResult>, IndexProviderError> {
        debug!("PDF Index Provider: Querying index of with params: {}, \
            num_results: {}, offset: {}", str, num_results, offset);
        debug!("PDF Index Provider: Embedding query");

        let text_chunk_future = async move {
            let text_vec = embeddinggemma::embed_query(str).await.map_err(|e| IndexProviderError {
                provider_name: PROVIDER_NAME.to_string(),
                r#type: IndexProviderErrorType::Embedding { source: e },
            })?;

            self.text_store.query_full_n(
                Some(text_vec),
                None, // Some(str), // temporarily disabled for tuning
                &[],
                num_results,
                offset
            ).await.map_err(|e| IndexProviderError {
                provider_name: PROVIDER_NAME.to_string(),
                r#type: IndexProviderErrorType::Store {
                    operation: "query full",
                    source: e,
                }
            })
        };
        let image_chunk_future = async move {
            let image_vec = siglip2::embed_query(str).await.map_err(|e| IndexProviderError {
                provider_name: PROVIDER_NAME.to_string(),
                r#type: IndexProviderErrorType::Embedding { source: e },
            })?;

            self.image_store.query_full_n(
                Some(image_vec),
                None, // Some(str), // temporarily disabled for tuning
                &[],
                num_results,
                offset
            ).await.map_err(|e| IndexProviderError {
                provider_name: PROVIDER_NAME.to_string(),
                r#type: IndexProviderErrorType::Store {
                    operation: "query full",
                    source: e,
                }
            })
        };

        let (text_result, image_result) = join!(
            text_chunk_future,
            image_chunk_future
        );
        let text_chunks = text_result?;
        let image_chunks = image_result?;

        let chunks = text_chunks.into_iter()
            .map(|c| (c.score, c.result.chunkfile))
            .chain(image_chunks.into_iter().map(|c| (c.score, c.result.chunkfile)))
            .collect::<Vec<(f32, _)>>();


        let mut results = vec![];
        for (score, chunkfile) in chunks {
            if score >= MIN_SCORE {
                // normalize to 0-100
                let norm_score = ((score - MIN_SCORE) / (EXPECTED_MAX_SCORE - MIN_SCORE)) * 100.0;
                debug!("PDF Index Provider: Normalized result score: orig: {}, chunkfile: {}, orig_score: {}, \
                    norm_score: {}", chunkfile.original_file, chunkfile.chunkfile, score, norm_score);
                results.push(ChunkQueryResult::new(chunkfile, norm_score));
            } else {
                debug!("PDF Index Provider: Result score is under minimum threshold: orig: {}, chunkfile: {}, \
                    orig_score: {}", chunkfile.original_file, chunkfile.chunkfile, score)
            }
        }
        Ok(results)
    }
}

// private constants and functions

const PROVIDER_NAME: &str = "PdfIndexProvider";

// These constants define chunking behavior
// EmbeddingGemma can do up to 2048 tokens context length, so this could be tuned up.
// The tokenizing in this chunker is not as robust. I am just splitting by whitespace. For example,
// I do not tokenize punctuation separately, I do not separate special characters, I will not
// slice up words/with/slashes/and/hyphens, etc, so I expect the actual token count will be somewhat
// higher when inputted into EmbeddingGemma
const TEXT_CHUNK_CHANNEL: &str = "text";
const TEXT_CHUNK_MAX_TOKENS: u32 = 1000;
// Length/width of the longest side in the chunked image
const IMAGE_CHUNK_CHANNEL: &str = "image";
const IMAGE_CHUNK_MAX_SIDE: u32 = 512;

// These constants must be tuned to the hybrid query results of lance FTS and siglip2 vector cosine similarity reranking
// TODO: tune
const EXPECTED_MAX_SCORE: f32 = 1.0;
const MIN_SCORE: f32 = 0.1;

async fn chunk_pdf(path: &Utf8Path, file: File, metadata: Metadata, out_dir: &Utf8Path)
    -> Result<Vec<ChunkFile>, anyhow::Error>
{
    let file = SyncIoBridge::new(file);
    let file_creation: DateTime<Utc> = DateTime::from(metadata.created()
        .expect("File creation datetime not available on this platform"));
    let file_modified: DateTime<Utc> = DateTime::from(metadata.modified()
        .expect("File modified datetime not available on this platform"));
    let file_length = metadata.len();

    let path = path.to_owned();
    let out_dir = out_dir.to_owned();
    let chunk_files = task::spawn_blocking(move || {
        let pdfium = get_pdfium();
        let document = pdfium.load_pdf_from_reader(file, None)?;
        let pages = document.pages();

        let mut chunks = vec![];
        for (page_index, page) in pages.iter().enumerate() {
            chunks.extend(create_text_chunks(
                &page,
                page_index,
                &path,
                file_creation,
                file_modified,
                file_length,
                &out_dir
            )?);
            chunks.extend(create_image_chunks(
                &page,
                page_index,
                &path,
                file_creation,
                file_modified,
                file_length,
                &out_dir
            )?);
        }

        Ok::<Vec<ChunkFile>, anyhow::Error>(chunks)
    }).await??; // this is Result<Result<vec, closure_error>, tokio::task_error>

    Ok(chunk_files)
}

fn create_text_chunks(
    page: &PdfPage,
    page_index: usize,
    path: &Utf8Path,
    file_creation: DateTime<Utc>,
    file_modified: DateTime<Utc>,
    file_length: u64,
    out_dir: &Utf8Path
) -> Result<Vec<ChunkFile>, anyhow::Error> {
    let text = page.text()?.all();

    // Separate page text into chunks if necessary (larger than max tokens)
    let chunks = chunk_text(&text);
    let num_chunks_in_page = chunks.len();

    // Assuming each page is "1.0" chunk length
    let chunk_length = 1.0 / num_chunks_in_page as f32;
    let mut text_chunks = vec![];
    for (i, chunk) in chunks.into_iter().enumerate() {
        // The chunk sequence is the page index plus a fractional part marking the start of
        // the chunk within the page
        // For example, the middle third of page 5 would be considered sequence id 5.3333.
        // The chunk length would be 1.0/(3 chunks in the page) = 0.3333, so the chunk would
        // represent the range 5.3333-5.6666.
        let chunk_sequence = page_index as f32 + (i as f32 / num_chunks_in_page as f32);
        let chunkfile = out_dir.join(format!("{}-{}.txt", TEXT_CHUNK_CHANNEL, chunk_sequence));

        // Write out the text chunk
        let chunk_owned = chunk.to_owned();
        std::fs::write(&chunkfile, &chunk_owned)?;

        // Add the full text blob to the metadata in the chunkfile struct, so it can be
        // searched with FTS
        let mut tags_map = Map::new();
        tags_map.insert("full_text".to_string(), chunk_owned.into());

        text_chunks.push(ChunkFile {
            original_file: path.to_owned(),
            chunk_channel: TEXT_CHUNK_CHANNEL.to_owned(),
            chunk_sequence_id: chunk_sequence,
            chunkfile,
            chunk_type: ChunkType::Text,
            chunk_length,
            original_file_creation_date: file_creation,
            original_file_modified_date: file_modified,
            original_file_size: file_length,
            original_file_tags: tags_map,
        });
    }

    Ok(text_chunks)
}

fn chunk_text(text: &str) -> Vec<&str> {
    // roughly, by whitespace
    let tokens = text.split_whitespace().collect::<Vec<&str>>();
    let divisor = (tokens.len() as u32 / TEXT_CHUNK_MAX_TOKENS) + 1;
    let token_target = (tokens.len() as f32 / divisor as f32).ceil() as u32;
    partition_by_whitespaces(text, token_target)
}

fn partition_by_whitespaces(text: &str, whitespace_count: u32) -> Vec<&str> {
    let mut partitions = Vec::new();
    let mut start = 0;
    let mut ws_seen = 0;
    
    for (idx, ch) in text.char_indices() {
        if ch.is_whitespace() {
            ws_seen += 1;
            
            if ws_seen == whitespace_count {
                // Partition from start up to and including this whitespace
                let end = idx + ch.len_utf8();
                partitions.push(&text[start..end]);
                start = end;
                ws_seen = 0;
            }
        }
    }
    
    // Don't forget the last partition if there's remaining text
    if start < text.len() {
        partitions.push(&text[start..]);
    }
    
    partitions
}

fn create_image_chunks(
    page: &PdfPage,
    page_index: usize,
    path: &Utf8Path,
    file_creation: DateTime<Utc>,
    file_modified: DateTime<Utc>,
    file_length: u64,
    out_dir: &Utf8Path
) -> Result<Vec<ChunkFile>, anyhow::Error> {
    let images = extract_images_from_page(page)?;
    let images_len = images.len();

    let chunk_len = 1.0 / images_len as f32;
    let mut image_chunks = vec![];
    for (index, image) in images.into_iter().enumerate() {
        let image = image.resize(
            IMAGE_CHUNK_MAX_SIDE,
            IMAGE_CHUNK_MAX_SIDE,
            FilterType::Triangle,
        );

        let chunk_sequence = page_index as f32 + (index as f32 / images_len as f32);
        let chunk_filename = format!("{}-{}.webp", IMAGE_CHUNK_CHANNEL, chunk_sequence);
        let chunkfile = out_dir.join(chunk_filename);
        image.save_with_format(&chunkfile, ImageFormat::WebP)?;
        
        image_chunks.push(ChunkFile {
            original_file: path.to_owned(),
            chunk_channel: IMAGE_CHUNK_CHANNEL.to_owned(),
            chunk_sequence_id: chunk_sequence,
            chunkfile,
            chunk_type: ChunkType::Image,
            chunk_length: chunk_len,
            original_file_creation_date: file_creation,
            original_file_modified_date: file_modified,
            original_file_size: file_length,
            original_file_tags: Map::new(),
        });
    }

    Ok(image_chunks)
}

fn extract_images_from_page(
    page: &PdfPage,
) -> Result<Vec<DynamicImage>, anyhow::Error> {
    let mut images = vec![];

    // Iterate through all objects on the page
    for object in page.objects().iter() {
        // Check if object is an image
        if let Some(image_object) = object.as_image_object() {
            // Potentially at some point it would be possible to determine exactly where the image
            // is positioned on the page, and base the sequence id of the image on that. This is
            // worth some thought.
            images.push(image_object.get_raw_image()?);
        }
    }

    Ok(images)
}