use std::{error::Error, sync::OnceLock};

use candle_core::{DType, Device, Tensor, D};
use candle_nn::VarBuilder;
use candle_transformers::models::clip::{ClipConfig, ClipModel};
use tokenizers::Tokenizer;

use crate::previewable::{PreviewType, PreviewedFile};

/// Adds the embeddable trait, signifying that a struct or object has data that it can use to
/// create an embedding.
/// 
/// Using the Embedder trait here ties the API of fetch to the API of embed_anything. This is something
/// I am willing to commit to because the functions that embed_anything seems to intend to provide match
/// closely the functionalities that I am looking to satisfy with such a trait, were I to build one myself.
/// This was my previous intention with fetch-translation, however having found embed_anything I believe
/// this is no longer necessary.
/// 
/// Interestingly, the authors of embed_anything originally set out to do what looks like the same goal
/// as what I am trying to achieve with fetch, with their Starlight Search project.
/// https://starlight-search.com/blog/2024/12/15/embed-anything/
/// It sounds like their strategy was to locally embed the entire document, and therefore they ran into issues
/// with both large documents and locally embedding things. Solution for large documents was to stream the
/// document instead of loading the entire thing into memory, and for local embeddings they built embed_
/// anything. My strategy differs slightly in that I only intend to embed limited sized previews of files,
/// but I also don't yet have a solution for something like a pdf file (which is both on the larger side,
/// and also contains multiple modalities within the same file).
pub trait Embeddable {
    /// Calculates the embedding for the presented data in the objects using the Embedder passed in the
    /// arguments. Embedder model should support both image and text embeddings.
    async fn calculate_embedding(&self) -> Result<Vec<f32>, Box<dyn Error>>;
}

#[derive(thiserror::Error, Debug)]
pub enum EmbeddingError {
    #[error("Error")]
    Unknown { msg: &'static str, #[source] source: Box<dyn Error> },
}

impl Embeddable for PreviewedFile<'_> {
    async fn calculate_embedding(&self) -> Result<Vec<f32>, Box<dyn Error>> {
        match self.r#type {
            PreviewType::Image => {
                // TODO: make this implementation more mature, both using a better model and better code,
                // with error handling, etc.
                let (model, _tokenizer) = get_model_and_tokenizer()?;
                let image = load_image(&self.preview_path, 224)?; // MUST be 224 or the tensor math for the model doesn't work out i think?
                let mut images = vec![];
                images.push(image);
                let images = Tensor::stack(&images, 0)?;

                let embedding = model.get_image_features(&images)?;
                let embedding = div_l2_norm(&embedding)?;
                let vector = embedding.to_vec2::<f32>()?.swap_remove(0);
                Ok(vector)
            },
            _ => todo!(),
        }
    }
}

impl Embeddable for &str {
    async fn calculate_embedding(&self) -> Result<Vec<f32>, Box<dyn Error>> {
        let (model, tokenizer) = get_model_and_tokenizer()?;

        let encoding = tokenizer.encode(*self, true).map_err(|e| EmbeddingError::Unknown { msg: "sth", source: e })?;
        let tokens = Tensor::new(encoding.get_ids().to_vec(), device())?.unsqueeze(0)?;

        let embedding = model.get_text_features(&tokens)?;
        let embedding = div_l2_norm(&embedding)?;
        let vector = embedding.to_vec2::<f32>()?.swap_remove(0);
        Ok(vector)
    }
}

// TODO Modularize model code and refactor into a separate package (fetch-translation?)
fn get_model_and_tokenizer() -> Result<(ClipModel, Tokenizer), Box<dyn Error>> {
    let device = device();

    let api = hf_hub::api::sync::Api::new()?;
    let api = api.repo(hf_hub::Repo::new(
        "openai/clip-vit-base-patch32".to_string(),
        hf_hub::RepoType::Model,
    ));
    let model_file = api.get("pytorch_model.bin")?;
    let tokenizer_file = api.get("tokenizer.json")?;

    let vb = VarBuilder::from_pth(model_file, DType::F32, &device)?;

    let config = ClipConfig::vit_base_patch32();
    let model = ClipModel::new(vb, &config)?;
    let tokenizer = Tokenizer::from_file(tokenizer_file).map_err(|e| EmbeddingError::Unknown { msg: "sth", source: e })?;
    Ok((model, tokenizer))
}

fn load_image<T: AsRef<std::path::Path>>(path: T, image_size: usize) -> Result<Tensor, Box<dyn Error>> {
    let device = device();

    let img = image::ImageReader::open(path)?.decode()?;
    let (height, width) = (image_size, image_size);
    let img = img.resize_to_fill(
        width as u32,
        height as u32,
        image::imageops::FilterType::Triangle,
    );
    let img = img.to_rgb8();
    let img = img.into_raw();
    let img = Tensor::from_vec(img, (height, width, 3), device)?
        .permute((2, 0, 1))?
        .to_dtype(DType::F32)?
        .affine(2. / 255., -1.)?;
    Ok(img)
}

pub fn div_l2_norm(v: &Tensor) -> Result<Tensor, Box<dyn Error>> {
    let l2_norm = v.sqr()?.sum_keepdim(D::Minus1)?.sqrt()?;
    Ok(v.broadcast_div(&l2_norm).map_err(|e| Box::new(e))?)
}

fn device() -> &'static Device {
    static DEVICE: OnceLock<Device> = OnceLock::new();
    &DEVICE.get_or_init(|| Device::cuda_if_available(0).unwrap())
}