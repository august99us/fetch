use std::{error::Error, path::Path};
use embed_anything::embeddings::embed::Embedder;
use fetch::{previewable::PossiblyPreviewable, index::{lancedb_store::LanceDBStore, IndexPreview, QuerySimilarFiles}};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("hi");
    let file = Path::new("test.jpg");
    let preview = file.preview()?.unwrap();

    let embedder = Embedder::from_pretrained_hf("clip", "openai/clip-vit-base-patch32", None).unwrap();

    let mut lancedbstore = LanceDBStore::new("./data_dir", embedder, 512).await?;

    let delete_result = lancedbstore.clear().await;
    println!("the result of the clear operation {:?}", delete_result);

    lancedbstore.index(preview).await?;

    let results = lancedbstore.query("dog").await?;

    println!("{:?}", results);

    Ok(())
}