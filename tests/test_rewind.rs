use agent_rewind::diff::ChangeType;
use agent_rewind::{DiffEngine, Engine, RestoreEngine};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_checkpoint_and_restore() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let engine = Engine::new(root);

    // 1. Create initial files
    fs::write(root.join("hello.txt"), "Hello, world!").unwrap();
    fs::write(root.join("config.json"), "{\"version\": 1}").unwrap();

    // 2. Capture initial checkpoint
    let cp1 = engine.create_checkpoint("initial state", None, None).unwrap();
    assert_eq!(cp1.files.len(), 2);
    assert!(cp1.files.contains_key("hello.txt"));
    assert!(cp1.files.contains_key("config.json"));

    // 3. Simulate destructive agent edits
    fs::write(root.join("hello.txt"), "Overwritten by agent hallucination").unwrap();
    fs::remove_file(root.join("config.json")).unwrap();
    fs::write(root.join("unwanted_file.js"), "console.log('garbage')").unwrap();

    // 4. Compute diff against cp1
    let diff_engine = DiffEngine::new(&engine);
    let deltas = diff_engine.compute_working_tree_diff(&cp1).unwrap();
    assert_eq!(deltas.len(), 3);

    let hello_delta = deltas.iter().find(|d| d.path == "hello.txt").unwrap();
    assert_eq!(hello_delta.change_type, ChangeType::Modified);

    let config_delta = deltas.iter().find(|d| d.path == "config.json").unwrap();
    assert_eq!(config_delta.change_type, ChangeType::Deleted);

    let unwanted_delta = deltas.iter().find(|d| d.path == "unwanted_file.js").unwrap();
    assert_eq!(unwanted_delta.change_type, ChangeType::Added);

    // 5. Restore checkpoint
    let restore_engine = RestoreEngine::new(&engine);
    restore_engine.restore_checkpoint(&cp1).unwrap();

    // 6. Verify restored state
    assert_eq!(fs::read_to_string(root.join("hello.txt")).unwrap(), "Hello, world!");
    assert_eq!(fs::read_to_string(root.join("config.json")).unwrap(), "{\"version\": 1}");
    assert!(!root.join("unwanted_file.js").exists());

    // 7. Verify diff is now clean
    let clean_deltas = diff_engine.compute_working_tree_diff(&cp1).unwrap();
    assert!(clean_deltas.is_empty());
}

#[test]
fn test_multiple_checkpoints_history() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let engine = Engine::new(root);

    fs::write(root.join("v1.txt"), "v1").unwrap();
    let cp1 = engine.create_checkpoint("cp 1", None, None).unwrap();

    fs::write(root.join("v2.txt"), "v2").unwrap();
    let cp2 = engine.create_checkpoint("cp 2", None, None).unwrap();

    let list = engine.list_checkpoints().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, cp1.id);
    assert_eq!(list[1].id, cp2.id);

    // Fetch by index "1"
    let fetched = engine.get_checkpoint("1").unwrap().unwrap();
    assert_eq!(fetched.id, cp1.id);
}
