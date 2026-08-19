use crate::types::{RandomStreamCursorV1, RandomStreamKeyV1, RootSeed256, RandomValidationError};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

const STREAM_DOMAIN: &[u8] = b"mtgml.rng.stream-key.v1";
const RAW_DOMAIN: &[u8] = b"mtgml.rng.raw-block.v1";

pub fn derive_stream_key(root: &RootSeed256, key: &RandomStreamKeyV1) -> [u8; 32] {
    let canonical = key.to_canonical_bytes();
    let mut data = Vec::with_capacity(STREAM_DOMAIN.len() + 1 + 4 + canonical.len());
    data.extend_from_slice(STREAM_DOMAIN);
    data.push(0x00);
    data.extend_from_slice(&(canonical.len() as u32).to_be_bytes());
    data.extend_from_slice(&canonical);
    let mut mac =
        HmacSha256::new_from_slice(root.as_bytes()).expect("HMAC accepts any key length");
    mac.update(&data);
    let result = mac.finalize().into_bytes();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

pub fn raw_block(k_stream: &[u8; 32], block_index: u64) -> [u8; 32] {
    let mut data = Vec::with_capacity(RAW_DOMAIN.len() + 1 + 8);
    data.extend_from_slice(RAW_DOMAIN);
    data.push(0x00);
    data.extend_from_slice(&block_index.to_be_bytes());
    let mut mac =
        HmacSha256::new_from_slice(k_stream).expect("HMAC accepts any key length");
    mac.update(&data);
    let result = mac.finalize().into_bytes();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

pub fn raw_u64_at(block: &[u8; 32], lane: usize) -> u64 {
    debug_assert!(lane < 4);
    let offset = lane * 8;
    u64::from_be_bytes([
        block[offset],
        block[offset + 1],
        block[offset + 2],
        block[offset + 3],
        block[offset + 4],
        block[offset + 5],
        block[offset + 6],
        block[offset + 7],
    ])
}

pub fn next_raw_u64(
    root: &RootSeed256,
    key: &RandomStreamKeyV1,
    cursor: &RandomStreamCursorV1,
) -> Result<(u64, RandomStreamCursorV1), RandomValidationError> {
    let i = cursor.next_raw_u64;
    if i == u64::MAX {
        return Err(RandomValidationError::StreamExhausted);
    }
    let k_stream = derive_stream_key(root, key);
    let block_index = i / 4;
    let lane = (i % 4) as usize;
    let block = raw_block(&k_stream, block_index);
    let value = raw_u64_at(&block, lane);
    Ok((value, RandomStreamCursorV1 { next_raw_u64: i + 1 }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{RandomStreamKindV1, RandomStreamScopeV1};

    const ALL_ZERO_SEED: &str =
        "0000000000000000000000000000000000000000000000000000000000000000";

    fn global_key() -> RandomStreamKeyV1 {
        RandomStreamKeyV1 {
            kind: RandomStreamKindV1::SyntheticM1,
            scope: RandomStreamScopeV1::Global,
        }
    }

    #[test]
    fn hmac_sha256_empty_key_empty_data() {
        let mut mac =
            HmacSha256::new_from_slice(b"").expect("HMAC accepts any key length");
        mac.update(b"");
        let result = mac.finalize().into_bytes();
        assert_eq!(
            crate::types::encode_lower_hex(&result),
            "b613679a0814d9ec772f95d778c35fc5ff1697c493715653c6c712144292c5ad"
        );
    }

    #[test]
    fn stream_derivation_zero_seed_global() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let key = global_key();
        let k_stream = derive_stream_key(&seed, &key);
        assert_eq!(
            crate::types::encode_lower_hex(&k_stream),
            "73635feaa9e90effe337e2cc9e1d801f63c9ede8d51b21a1120e624da2d648f9"
        );
    }

    #[test]
    fn raw_block_0_kat() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let key = global_key();
        let k_stream = derive_stream_key(&seed, &key);
        let block = raw_block(&k_stream, 0);
        assert_eq!(
            crate::types::encode_lower_hex(&block),
            "6818e6bd053d9b770e26253e8d724b0403c524aeb6b3cff52508069342e336e4"
        );
    }

    #[test]
    fn raw_block_1_kat() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let key = global_key();
        let k_stream = derive_stream_key(&seed, &key);
        let block = raw_block(&k_stream, 1);
        assert_eq!(
            crate::types::encode_lower_hex(&block),
            "ac6a5d827f0dcbbf060d1adce197e55569da50c9030d2a2b2a7f637923566d45"
        );
    }

    #[test]
    fn raw_words_0_to_7_kat() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let key = global_key();
        let k_stream = derive_stream_key(&seed, &key);
        let block0 = raw_block(&k_stream, 0);
        let block1 = raw_block(&k_stream, 1);
        assert_eq!(raw_u64_at(&block0, 0), 0x6818e6bd053d9b77);
        assert_eq!(raw_u64_at(&block0, 1), 0x0e26253e8d724b04);
        assert_eq!(raw_u64_at(&block0, 2), 0x03c524aeb6b3cff5);
        assert_eq!(raw_u64_at(&block0, 3), 0x2508069342e336e4);
        assert_eq!(raw_u64_at(&block1, 0), 0xac6a5d827f0dcbbf);
        assert_eq!(raw_u64_at(&block1, 1), 0x060d1adce197e555);
        assert_eq!(raw_u64_at(&block1, 2), 0x69da50c9030d2a2b);
        assert_eq!(raw_u64_at(&block1, 3), 0x2a7f637923566d45);
    }

    #[test]
    fn cursor_boundary_kat() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let key = global_key();
        let cursor = RandomStreamCursorV1 {
            next_raw_u64: u64::MAX - 1,
        };
        let (value, new_cursor) = next_raw_u64(&seed, &key, &cursor).unwrap();
        assert_eq!(value, 0x021a6c120112e7b3);
        assert_eq!(new_cursor.next_raw_u64, u64::MAX);
    }

    #[test]
    fn exhausted_cursor_errors() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let key = global_key();
        let cursor = RandomStreamCursorV1 {
            next_raw_u64: u64::MAX,
        };
        assert_eq!(
            next_raw_u64(&seed, &key, &cursor),
            Err(RandomValidationError::StreamExhausted)
        );
    }

    #[test]
    fn stream_isolation() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let key1 = global_key();
        let _key2 = RandomStreamKeyV1 {
            kind: RandomStreamKindV1::SyntheticM1,
            scope: RandomStreamScopeV1::Player(mtgml_model::PlayerId(1)),
        };
        let cursor1 = RandomStreamCursorV1::default();
        let (_, new_cursor1) = next_raw_u64(&seed, &key1, &cursor1).unwrap();
        assert_eq!(new_cursor1.next_raw_u64, 1);
        let cursor2 = RandomStreamCursorV1::default();
        assert_eq!(cursor2.next_raw_u64, 0);
    }

    #[test]
    fn lane3_advances_to_next_block_lane0() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let key = global_key();
        let cursor = RandomStreamCursorV1 { next_raw_u64: 3 };
        let (value, new_cursor) = next_raw_u64(&seed, &key, &cursor).unwrap();
        assert_eq!(value, 0x2508069342e336e4);
        assert_eq!(new_cursor.next_raw_u64, 4);
        let cursor4 = new_cursor;
        let (value4, _) = next_raw_u64(&seed, &key, &cursor4).unwrap();
        assert_eq!(value4, 0xac6a5d827f0dcbbf);
    }
}
