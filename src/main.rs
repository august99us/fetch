use std::path::Path;
use fetch::{embeddable::Embeddable, previewable::PossiblyPreviewable, storage::{lancedb_store::LanceDBStore, IndexPreview, QuerySimilarFiles}};

#[tokio::main]
async fn main() -> Result<(), String> {
    println!("hi");
    let file = Path::new("test.jpg");
    let preview = file.preview()?.unwrap();
    let embedding = preview.calculate_embedding();
    println!("{:?}", embedding);

    let lancedbstore = LanceDBStore::new("./data_dir").await.map_err(|e| e.to_string())?;

    lancedbstore.clear();

    lancedbstore.index(preview);

    let results = lancedbstore.query("dog").await?;

    println!("{:?}", results);

    Ok(())
}