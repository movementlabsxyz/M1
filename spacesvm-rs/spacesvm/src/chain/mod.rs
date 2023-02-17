pub mod crypto;
pub mod storage;
pub mod tx;
pub mod vm;

#[cfg(test)]
mod tests {
    use avalanche_types::{hash, ids, key};

    #[test]
    fn signature_recovers() {
        let secret_key = key::secp256k1::private_key::Key::generate().unwrap();
        let public_key = secret_key.to_public_key();

        let hash = hash::keccak256("yolo message".as_bytes());
        let sig = secret_key.sign_digest(&hash.as_bytes()).unwrap();
        let sender =
            key::secp256k1::public_key::Key::from_signature(hash.as_bytes(), &sig.to_bytes())
                .unwrap();
        assert_eq!(public_key.to_string(), sender.to_string());

        println!("id: {:?}", ids::Id::from_slice("baz".as_bytes()));
    }
}
