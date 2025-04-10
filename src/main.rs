use std::error::Error;
use embed_anything::embeddings::embed::Embedder;
use fetch::vector_store::{lancedb_store::LanceDBStore, QueryVectorKeys};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("hi");
    let embedder = Embedder::from_pretrained_hf("clip", "openai/clip-vit-base-patch32", None).unwrap();

    let lancedbstore = LanceDBStore::new("./data_dir", 512).await?;

    let vector_query = embedder.embed(&["the thinker".to_string()], None).await?.get(0).unwrap().to_dense()?;

    let results = lancedbstore.query_n_keys(vector_query, 3).await?;

    println!("{:?}", results);

    Ok(())
}