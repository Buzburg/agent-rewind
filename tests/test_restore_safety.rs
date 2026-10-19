use agent_rewind::checkpoint::{Checkpoint, BLOBS_DIR};
use agent_rewind::{Engine, RestoreEngine};
use std::fs;
use tempfile::{tempdir, TempDir};

fn fixture() -> (TempDir, Engine, Checkpoint) {
    let directory = tempdir().unwrap();
    let root = directory.path().join("workspace");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("first.txt"), "original first").unwrap();
    fs::create_dir(root.join("nested")).unwrap();
    fs::write(root.join("nested/last.txt"), "original last").unwrap();
    let engine = Engine::new(root);
    let snapshot = engine.create_checkpoint("original", None, None).unwrap();
    fs::write(engine.root.join("first.txt"), "changed first").unwrap();
    fs::write(engine.root.join("nested/last.txt"), "changed last").unwrap();
    (directory, engine, snapshot)
}

fn unchanged(engine: &Engine) {
    assert_eq!(
        fs::read_to_string(engine.root.join("first.txt")).unwrap(),
        "changed first"
    );
}

#[cfg(unix)]
#[test]
fn reject_symlink_leaf_before_any_write() {
    let (directory, engine, snapshot) = fixture();
    let outside = directory.path().join("outside.txt");
    fs::write(&outside, "outside").unwrap();
    let target = engine.root.join("nested/last.txt");
    fs::remove_file(&target).unwrap();
    std::os::unix::fs::symlink(&outside, target).unwrap();
    assert!(RestoreEngine::new(&engine)
        .restore_checkpoint(&snapshot)
        .is_err());
    unchanged(&engine);
    assert_eq!(fs::read_to_string(outside).unwrap(), "outside");
}

#[cfg(unix)]
#[test]
fn reject_symlink_ancestor_before_any_write() {
    let (directory, engine, snapshot) = fixture();
    let outside = directory.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("last.txt"), "outside").unwrap();
    fs::remove_dir_all(engine.root.join("nested")).unwrap();
    std::os::unix::fs::symlink(&outside, engine.root.join("nested")).unwrap();
    assert!(RestoreEngine::new(&engine)
        .restore_checkpoint(&snapshot)
        .is_err());
    unchanged(&engine);
    assert_eq!(
        fs::read_to_string(outside.join("last.txt")).unwrap(),
        "outside"
    );
}

#[test]
fn hard_link_restore_does_not_modify_outside_inode() {
    let (directory, engine, snapshot) = fixture();
    let outside = directory.path().join("outside.txt");
    fs::write(&outside, "outside").unwrap();
    let target = engine.root.join("nested/last.txt");
    fs::remove_file(&target).unwrap();
    fs::hard_link(&outside, &target).unwrap();
    RestoreEngine::new(&engine)
        .restore_checkpoint(&snapshot)
        .unwrap();
    assert_eq!(fs::read_to_string(outside).unwrap(), "outside");
    assert_eq!(fs::read_to_string(target).unwrap(), "original last");
}

#[test]
fn missing_and_corrupt_blobs_fail_before_deleting_or_writing() {
    for corrupt in [false, true] {
        let (_directory, engine, snapshot) = fixture();
        let blob = engine
            .root
            .join(BLOBS_DIR)
            .join(&snapshot.files["nested/last.txt"].hash);
        if corrupt {
            fs::write(blob, "corrupt").unwrap();
        } else {
            fs::remove_file(blob).unwrap();
        }
        let extra = engine.root.join("added.txt");
        fs::write(&extra, "preserve until snapshot validated").unwrap();
        assert!(RestoreEngine::new(&engine)
            .restore_checkpoint(&snapshot)
            .is_err());
        unchanged(&engine);
        assert!(extra.is_file());
    }
}

#[test]
fn traversal_and_storage_paths_fail_before_any_write() {
    let (_directory, engine, snapshot) = fixture();
    for path in [
        "../outside.txt",
        "/outside.txt",
        "C:/outside.txt",
        "nested\\outside",
        ".rewind/manifest.json",
        ".git/config",
        ".GIT/config",
        ".git./config",
        "NUL.txt",
        "a/../outside",
    ] {
        let mut malformed = snapshot.clone();
        let mut record = malformed.files["first.txt"].clone();
        record.path = path.to_string();
        malformed.files.insert(path.to_string(), record);
        assert!(
            RestoreEngine::new(&engine)
                .restore_checkpoint(&malformed)
                .is_err(),
            "{path}"
        );
        unchanged(&engine);
    }
}

#[test]
fn bad_hash_cannot_read_outside_blob_store() {
    let (_directory, engine, mut snapshot) = fixture();
    snapshot.files.get_mut("first.txt").unwrap().hash = "../../outside.txt".into();
    assert!(RestoreEngine::new(&engine)
        .restore_checkpoint(&snapshot)
        .is_err());
    unchanged(&engine);
}

#[test]
fn reject_other_workspace_and_unbound_legacy_snapshot() {
    let (directory, engine, snapshot) = fixture();
    let other = directory.path().join("other");
    fs::create_dir(&other).unwrap();
    assert!(RestoreEngine::new(&Engine::new(&other))
        .restore_checkpoint(&snapshot)
        .is_err());
    assert_eq!(fs::read_dir(other).unwrap().count(), 0);
    let mut legacy = serde_json::to_value(&snapshot).unwrap();
    legacy.as_object_mut().unwrap().remove("workspace_root");
    let legacy: Checkpoint = serde_json::from_value(legacy).unwrap();
    assert!(RestoreEngine::new(&engine)
        .restore_checkpoint(&legacy)
        .is_err());
    unchanged(&engine);
}

#[cfg(unix)]
#[test]
fn cleanup_does_not_traverse_untracked_directory_links() {
    let (directory, engine, snapshot) = fixture();
    let outside = directory.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::create_dir(outside.join("empty")).unwrap();
    std::os::unix::fs::symlink(&outside, engine.root.join("linked-folder")).unwrap();
    RestoreEngine::new(&engine)
        .restore_checkpoint(&snapshot)
        .unwrap();
    assert!(outside.join("empty").is_dir());
}

#[test]
fn conflicting_manifest_paths_fail_before_any_write() {
    let (_directory, engine, mut snapshot) = fixture();
    let mut child = snapshot.files["first.txt"].clone();
    child.path = "first.txt/child".into();
    snapshot.files.insert(child.path.clone(), child);
    assert!(RestoreEngine::new(&engine)
        .restore_checkpoint(&snapshot)
        .is_err());
    unchanged(&engine);
}
