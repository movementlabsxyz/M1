use ripemd::Ripemd160;
use sha3::Digest;

// TODO: move to avalanche-types
/// Compute a cryptographically strong 160 bit hash of the input byte slice
pub fn compute_hash_160(buf: &[u8]) -> Vec<u8> {
    let hash = Ripemd160::digest(buf);

    let mut ripe = [0u8; 20];
    ripe.copy_from_slice(&hash);
    ripe.to_vec()
}
