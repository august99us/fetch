use std::error::Error;
use embed_anything::embeddings::embed::Embedder;
use fetch::semantic_index::{lancedb_store::LanceDBStore, QuerySimilarFiles};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("hi");
    let embedder = Embedder::from_pretrained_hf("clip", "openai/clip-vit-base-patch32", None).unwrap();

    let lancedbstore = LanceDBStore::new("./data_dir", embedder, 512).await?;

    let results = lancedbstore.query_n("the thinker", 3).await?;

    println!("{:?}", results);

    Ok(())
}