use clvmr::sha2::{Digest, Sha256};
use clvm_utils::tree_hash_atom;

fn tree_hash_pair(first: [u8; 32], rest: [u8; 32]) -> [u8; 32] {
    let mut sha256 = Sha256::new();
    sha256.update([2]);
    sha256.update(first);
    sha256.update(rest);
    sha256.finalize().into()
}

pub fn curry_tree_hash(program_hash: [u8; 32], arg_hashes: &[[u8; 32]]) -> [u8; 32] {
    let nil = tree_hash_atom(&[]);
    let op_q = tree_hash_atom(&[1]);
    let op_a = tree_hash_atom(&[2]);
    let op_c = tree_hash_atom(&[4]);

    let quoted_program = tree_hash_pair(op_q, program_hash);
    let mut quoted_args = tree_hash_atom(&[1]);

    for &arg_hash in arg_hashes.iter().rev() {
        let quoted_arg = tree_hash_pair(op_q, arg_hash);
        let terminated_args = tree_hash_pair(quoted_args, nil);
        let terminated_args = tree_hash_pair(quoted_arg, terminated_args);
        quoted_args = tree_hash_pair(op_c, terminated_args);
    }

    let terminated_args = tree_hash_pair(quoted_args, nil);
    let program_and_args = tree_hash_pair(quoted_program, terminated_args);
    tree_hash_pair(op_a, program_and_args)
}