use std::sync::Arc;

use holo_hash::EntryHash;
use holochain::{conductor::api::error::ConductorApiResult, sweettest::*};
use holochain_serialized_bytes::prelude::*;
use holochain_wasm_test_utils::TestWasm;

#[tokio::test(flavor = "multi_thread")]
async fn too_big_membrane_proof() {
    let mut conductor = SweetConductor::from_standard_config().await;

    let (dna, _) = SweetDnaFile::unique_from_test_wasms(vec![TestWasm::GenesisSelfCheckValid])
        .await
        .unwrap();

    let many_bytes: Vec<_> = std::iter::repeat(1).take(10_000).collect();
    let huge_proof = Some(Arc::new(SerializedBytes::from(UnsafeBytes::from(
        many_bytes,
    ))));

    let _app = conductor
        .setup_app(&"app", [(dna, huge_proof)])
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn too_big_entry() {
    let mut conductor = SweetConductor::from_standard_config().await;

    let (dna, _) = SweetDnaFile::unique_from_test_wasms(vec![TestWasm::SerRegression])
        .await
        .unwrap();

    let huge_string: String = std::iter::repeat('x').take(20_000_000).collect();
    let agents = SweetAgents::get(conductor.keystore(), 2).await;
    let apps = conductor
        .setup_app_for_agents(&"app", agents, [dna])
        .await
        .unwrap();
    let ((alice,), (_,)) = apps.into_tuples();

    let result: ConductorApiResult<EntryHash> = conductor
        .call_fallible(
            &alice.zome(TestWasm::SerRegression),
            "create_channel",
            huge_string,
        )
        .await;
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Attempted to create an Entry whose size exceeds the limit."));
}
