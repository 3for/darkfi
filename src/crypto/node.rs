use bitvec::{order::Lsb0, view::AsBits};
use ff::PrimeField;
use group::Curve;
use lazy_static::lazy_static;
use std::io;

use super::merkle::Hashable;

pub const SAPLING_COMMITMENT_TREE_DEPTH: usize = 4;

/// Compute a parent node in the Sapling commitment tree given its two children.
pub fn merkle_hash(depth: usize, lhs: &[u8; 32], rhs: &[u8; 32]) -> bls12_381::Scalar {
    // This thing is nasty lol
    let lhs = {
        let mut tmp = [false; 256];
        for (a, b) in tmp.iter_mut().zip(lhs.as_bits::<Lsb0>()) {
            *a = *b;
        }
        tmp
    };

    let rhs = {
        let mut tmp = [false; 256];
        for (a, b) in tmp.iter_mut().zip(rhs.as_bits::<Lsb0>()) {
            *a = *b;
        }
        tmp
    };

    jubjub::ExtendedPoint::from(zcash_primitives::pedersen_hash::pedersen_hash(
        zcash_primitives::pedersen_hash::Personalization::MerkleTree(depth),
        lhs.iter()
            .copied()
            .take(bls12_381::Scalar::NUM_BITS as usize)
            .chain(
                rhs.iter()
                    .copied()
                    .take(bls12_381::Scalar::NUM_BITS as usize),
            ),
    ))
    .to_affine()
    .get_u()
}

pub fn hash_coin(coin: [u8; 32]) -> bls12_381::Scalar {
    let rhs = {
        let mut tmp = [false; 256];
        for (a, b) in tmp.iter_mut().zip(coin.as_bits::<Lsb0>()) {
            *a = *b;
        }
        tmp
    };

    jubjub::ExtendedPoint::from(zcash_primitives::pedersen_hash::pedersen_hash(
        zcash_primitives::pedersen_hash::Personalization::NoteCommitment,
        rhs.iter().copied(),
    ))
    .to_affine()
    .get_u()
}

/// A node within the Sapling commitment tree.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Node {
    pub repr: [u8; 32],
}

impl Node {
    pub fn new(repr: [u8; 32]) -> Self {
        Self { repr }
    }
}

impl Hashable for Node {
    fn read<R: io::Read>(mut reader: R) -> io::Result<Self> {
        let mut repr = [0u8; 32];
        reader.read_exact(&mut repr)?;
        Ok(Self::new(repr))
    }

    fn write<W: io::Write>(&self, mut writer: W) -> io::Result<()> {
        writer.write_all(self.repr.as_ref())
    }

    fn combine(depth: usize, lhs: &Self, rhs: &Self) -> Self {
        Self {
            repr: merkle_hash(depth, &lhs.repr, &rhs.repr).to_repr(),
        }
    }

    fn blank() -> Self {
        // The smallest u-coordinate that is not on the curve
        // is one.
        let uncommitted_note = bls12_381::Scalar::one();
        Self {
            repr: uncommitted_note.to_repr(),
        }
    }

    fn empty_root(depth: usize) -> Self {
        EMPTY_ROOTS[depth]
    }
}

impl From<Node> for bls12_381::Scalar {
    fn from(node: Node) -> Self {
        bls12_381::Scalar::from_repr(node.repr).expect("Tree nodes should be in the prime field")
    }
}

lazy_static! {
    static ref EMPTY_ROOTS: Vec<Node> = {
        let mut v = vec![Node::blank()];
        for d in 0..SAPLING_COMMITMENT_TREE_DEPTH {
            let next = Node::combine(d, &v[d], &v[d]);
            v.push(next);
        }
        v
    };
}