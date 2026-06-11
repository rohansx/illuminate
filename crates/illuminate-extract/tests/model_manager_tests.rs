use std::fs;
use std::io::Write;

use illuminate_extract::model_manager::*;
use sha2::{Digest, Sha256};

#[test]
fn test_model_spec_creation() {
    let spec = gliner_large_v21_int8();
    assert_eq!(spec.name, "gliner_large-v2.1/onnx/model_int8.onnx");
    assert!(!spec.url.is_empty());
    assert!(!spec.sha256.is_empty());
    assert!(spec.size_bytes > 0);

    let spec2 = gliner_multitask_large();
    assert_eq!(
        spec2.name,
        "gliner-multitask-large-v0.5/onnx/model_int8.onnx"
    );

    let spec3 = minilm_l6_v2();
    assert_eq!(spec3.name, "minilm-l6-v2.onnx");
}

#[test]
fn test_cache_dir_creation() {
    let tmp = tempfile::tempdir().unwrap();
    let cache_dir = tmp.path().join("test_cache");

    let mgr = ModelManager::with_cache_dir(cache_dir.clone()).unwrap();
    assert!(cache_dir.exists());

    // model_path should be inside the cache dir
    let spec = gliner_large_v21_int8();
    let path = mgr.model_path(&spec);
    assert_eq!(
        path,
        cache_dir.join("gliner_large-v2.1/onnx/model_int8.onnx")
    );
}

#[test]
fn test_is_cached_returns_false_for_missing_model() {
    let tmp = tempfile::tempdir().unwrap();
    let mgr = ModelManager::with_cache_dir(tmp.path().to_path_buf()).unwrap();

    let spec = gliner_large_v21_int8();
    assert!(!mgr.is_cached(&spec));
}

#[test]
fn test_is_cached_returns_false_for_wrong_size() {
    let tmp = tempfile::tempdir().unwrap();
    let mgr = ModelManager::with_cache_dir(tmp.path().to_path_buf()).unwrap();

    let spec = ModelSpec {
        name: "tiny.onnx".into(),
        url: String::new(),
        sha256: String::new(),
        size_bytes: 1024,
    };

    // Write a file with the wrong size
    let path = mgr.model_path(&spec);
    fs::write(&path, b"hello").unwrap();
    assert!(!mgr.is_cached(&spec));
}

#[test]
fn test_is_cached_returns_true_for_correct_size() {
    let tmp = tempfile::tempdir().unwrap();
    let mgr = ModelManager::with_cache_dir(tmp.path().to_path_buf()).unwrap();

    let data = b"test model data";
    let spec = ModelSpec {
        name: "tiny.onnx".into(),
        url: String::new(),
        sha256: String::new(),
        size_bytes: data.len() as u64,
    };

    let path = mgr.model_path(&spec);
    fs::write(&path, data).unwrap();
    assert!(mgr.is_cached(&spec));
}

#[test]
fn test_verify_on_small_file() {
    let tmp = tempfile::tempdir().unwrap();
    let mgr = ModelManager::with_cache_dir(tmp.path().to_path_buf()).unwrap();

    let data = b"the quick brown fox jumps over the lazy dog";
    let hash = format!("{:x}", Sha256::digest(data));

    let spec = ModelSpec {
        name: "verify_test.bin".into(),
        url: String::new(),
        sha256: hash,
        size_bytes: data.len() as u64,
    };

    let path = mgr.model_path(&spec);
    let mut f = fs::File::create(&path).unwrap();
    f.write_all(data).unwrap();

    assert!(mgr.verify(&spec).unwrap());
}

#[test]
fn test_verify_returns_false_on_mismatch() {
    let tmp = tempfile::tempdir().unwrap();
    let mgr = ModelManager::with_cache_dir(tmp.path().to_path_buf()).unwrap();

    let spec = ModelSpec {
        name: "bad_hash.bin".into(),
        url: String::new(),
        sha256: "0000000000000000000000000000000000000000000000000000000000000000".into(),
        size_bytes: 5,
    };

    let path = mgr.model_path(&spec);
    fs::write(&path, b"hello").unwrap();

    assert!(!mgr.verify(&spec).unwrap());
}

/// Specs use nested names (`gliner_large-v2.1/onnx/model_int8.onnx`), so on a
/// fresh cache `download` must create the destination's parent directories
/// before writing — without this, first-run `illuminate models download`
/// fails with "No such file or directory". Exercises the REAL download path
/// against a localhost HTTP server (no mocks).
#[test]
fn test_download_creates_nested_parent_dirs() {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}/model.onnx", server.server_addr());
    let payload: &[u8] = b"onnx-bytes";
    let handle = std::thread::spawn(move || {
        if let Ok(req) = server.recv() {
            let _ = req.respond(tiny_http::Response::from_data(payload.to_vec()));
        }
    });

    let tmp = tempfile::tempdir().unwrap();
    let mgr = ModelManager::with_cache_dir(tmp.path().to_path_buf()).unwrap();
    let spec = ModelSpec {
        name: "nested/onnx/model_int8.onnx".into(),
        url,
        sha256: "skip".into(),
        size_bytes: payload.len() as u64,
    };

    let path = mgr
        .download(&spec)
        .expect("download must create parent dirs");
    assert!(path.ends_with("nested/onnx/model_int8.onnx"));
    assert_eq!(fs::read(&path).unwrap(), payload);
    handle.join().unwrap();
}

#[test]
fn test_verify_errors_on_missing_file() {
    let tmp = tempfile::tempdir().unwrap();
    let mgr = ModelManager::with_cache_dir(tmp.path().to_path_buf()).unwrap();

    let spec = ModelSpec {
        name: "nonexistent.bin".into(),
        url: String::new(),
        sha256: String::new(),
        size_bytes: 0,
    };

    assert!(mgr.verify(&spec).is_err());
}
