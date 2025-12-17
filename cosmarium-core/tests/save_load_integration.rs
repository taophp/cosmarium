use cosmarium_core::document::{DocumentFormat, DocumentManager};
use cosmarium_core::events::EventBus;
use cosmarium_core::project::Project;
use std::sync::Arc;
use tokio::sync::RwLock;

// This integration test verifies that creating a project, adding a document
// with content, saving the document to a file under the project's content
// directory, saving the project metadata, then re-loading the project will
// preserve the document ID in the project's metadata and the file on disk.

#[tokio::test]
async fn save_and_load_project_with_document() {
    // Create a temporary directory
    let mut tmp = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    tmp.push(format!("cosmarium_integration_{}", nanos));
    let project_path = tmp.join("proj_test");
    tokio::fs::create_dir_all(&project_path).await.unwrap();

    // Create project
    let mut project = Project::new("Integration Test", &project_path, "novel").unwrap();

    // Initialize document manager and create a document
    let event_bus = Arc::new(RwLock::new(EventBus::new()));
    let mut dm = DocumentManager::new();
    dm.initialize(event_bus.clone()).await.unwrap();

    let content = "# Hello\n\nThis is an integration test.";
    let doc_id = dm
        .create_document("Test Doc", content, DocumentFormat::Markdown)
        .await
        .unwrap();

    // Set file path under project's content directory
    let content_dir = project_path.join("content");
    tokio::fs::create_dir_all(&content_dir).await.unwrap();
    let file_name = format!("doc_{}.md", doc_id);
    let file_path = content_dir.join(&file_name);

    if let Some(doc) = dm.get_document_mut(doc_id) {
        doc.set_file_path(&file_path);
    }

    // Save the document
    dm.save_document(doc_id).await.unwrap();

    // Add document to project and save project
    project.add_document(doc_id);
    project.save().await.unwrap();

    // Assert file exists
    assert!(file_path.exists(), "Document file should exist on disk");

    // Load project from disk
    let loaded = Project::load(&project_path).await.unwrap();

    // Assert the document id is recorded in project metadata
    assert!(
        loaded.documents().contains(&doc_id),
        "Loaded project should reference the document id"
    );

    // Read file contents and compare
    let read_content = tokio::fs::read_to_string(&file_path).await.unwrap();
    assert!(read_content.contains("This is an integration test."));
}
