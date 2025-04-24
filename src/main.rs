use std::error::Error;
use candle_core::{DType, Device, Tensor, D};
use candle_nn::VarBuilder;
use candle_transformers::models::clip::{ClipConfig, ClipModel};
use tokenizers::Tokenizer;
use fetch::vector_store::{lancedb_store::LanceDBStore, QueryVectorKeys};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let device = Device::new_cuda(0)?;
    let lancedbstore = LanceDBStore::new("./data_dir", 512).await.unwrap();

    let (model, tokenizer) = get_model_and_tokenizer(&device).unwrap();

    let encoding = tokenizer.encode("the thinker".to_string(), true).unwrap();
    let tokens = Tensor::new(encoding.get_ids().to_vec(), &device)?.unsqueeze(0)?;

    let embedding = model.get_text_features(&tokens)?;
    let embedding = div_l2_norm(&embedding).unwrap();
    let vector_query = embedding.to_vec2::<f32>()?.swap_remove(0);

    let results = lancedbstore.query_n_keys(vector_query, 3).await.unwrap();

    println!("{:?}", results);

    Ok(())
}

fn get_model_and_tokenizer(device: &Device) -> Result<(ClipModel, Tokenizer), Box<dyn Error>> {
    let api = hf_hub::api::sync::Api::new()?;
    let api = api.repo(hf_hub::Repo::new(
        "openai/clip-vit-base-patch32".to_string(),
        hf_hub::RepoType::Model,
    ));
    let model_file = api.get("pytorch_model.bin")?;
    let tokenizer_file = api.get("tokenizer.json")?;

    let vb = VarBuilder::from_pth(model_file, DType::F32, device)?;

    let config = ClipConfig::vit_base_patch32();
    let model = ClipModel::new(vb, &config)?;
    let tokenizer = Tokenizer::from_file(tokenizer_file).unwrap();
    Ok((model, tokenizer))
}

pub fn div_l2_norm(v: &Tensor) -> Result<Tensor, Box<dyn Error>> {
    let l2_norm = v.sqr()?.sum_keepdim(D::Minus1)?.sqrt()?;
    Ok(v.broadcast_div(&l2_norm).map_err(|e| Box::new(e))?)
}