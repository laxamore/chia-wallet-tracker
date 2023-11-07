use bech32::ToBase32;
use hex_literal::hex;
use chia_bls::{G1Element, DerivableKey};
use clvm_utils::tree_hash_atom;

use crate::curry_tree_hash::curry_tree_hash;

static DEFAULT_HIDDEN_PUZZLE_HASH: [u8; 32] = hex!("711d6c4e32c92e53179b199484cf8c897542bc57f2b22582799f9d657eec4699");
static STANDARD_PUZZLE_HASH: [u8; 32] = hex!("e9aaa49f45bad5c889b86ee3341550c155cfdd10c3a6757de618d20612fffd52");

fn master_pk_to_wallet_pk_unhardened_intermediate(master_pk: &G1Element) -> G1Element {
    let k = DerivableKey::derive_unhardened(master_pk, 12381);
    let k = DerivableKey::derive_unhardened(&k, 8444);
    DerivableKey::derive_unhardened(&k, 2)
}

pub fn generate_observe_wallet_address(master_pk: &G1Element, index: u32) -> String {
    let intermediate = master_pk_to_wallet_pk_unhardened_intermediate(master_pk);
    let key = DerivableKey::derive_unhardened(&intermediate, index);
    let synthetic_key = crate::derive_synthetic::DeriveSynthetic::derive_synthetic(&key, &DEFAULT_HIDDEN_PUZZLE_HASH);
    let pk_tree_hash = tree_hash_atom(&synthetic_key.to_bytes());
    let puzzle_hash = curry_tree_hash(STANDARD_PUZZLE_HASH, &[pk_tree_hash]);
    bech32::encode("xch", puzzle_hash.to_vec().to_base32(), bech32::Variant::Bech32m).unwrap()
}

pub fn generate_multiple_observe_wallet_addresses(master_pk: &G1Element, start_from_index: u32, count: &u32) -> Vec<String> {
    let mut addresses: Vec<String> = Vec::new();
    for i in start_from_index..(start_from_index + count) {
        addresses.push(generate_observe_wallet_address(master_pk, i));
    }
    addresses
}