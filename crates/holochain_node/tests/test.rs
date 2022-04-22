use holochain_node as h;

struct MyDna;

impl h::DnaT for MyDna {
    fn validate(&self, _op: h::Op, _context: &h::ValidationContext) -> bool {
        // In this DNA, we only publish content that is less than 20 bytes long
        // action.content.len() < 20
        true
    }

    fn fingerprint(&self) -> h::DnaFingerprint {
        vec![].into()
    }
}

#[tokio::test]
async fn scenario1() {
    let node = h::Node::new();
    let keystore = holochain_keystore::test_keystore::spawn_test_keystore()
        .await
        .unwrap();
    let alice_key = keystore.new_sign_keypair_random().await.unwrap();
    let bobbo_key = keystore.new_sign_keypair_random().await.unwrap();

    let alice_cell = node.add_cell(MyDna, alice_key);
    let bobbo_cell = node.add_cell(MyDna, bobbo_key);
}
