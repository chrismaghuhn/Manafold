use crate::hmac_counter::next_raw_u64;
use crate::types::{RandomStreamCursorV1, RandomStreamKeyV1, RandomValidationError, RootSeed256};

pub fn uniform_below_u64(
    root: &RootSeed256,
    key: &RandomStreamKeyV1,
    cursor: &RandomStreamCursorV1,
    n: u64,
) -> Result<(u64, u64, RandomStreamCursorV1), RandomValidationError> {
    if n == 0 {
        return Err(RandomValidationError::InvalidRandomBound);
    }
    if n == 1 {
        return Ok((0, 0, *cursor));
    }
    let threshold = ((1u128 << 64) % (n as u128)) as u64;
    let mut current = *cursor;
    let mut consumed = 0u64;
    loop {
        let (word, next) = next_raw_u64(root, key, &current)?;
        consumed += 1;
        if word >= threshold {
            return Ok((word % n, consumed, next));
        }
        current = next;
    }
}

pub fn uniform_range_u64(
    root: &RootSeed256,
    key: &RandomStreamKeyV1,
    cursor: &RandomStreamCursorV1,
    lower: u64,
    upper: u64,
) -> Result<(u64, u64, RandomStreamCursorV1), RandomValidationError> {
    if lower >= upper {
        return Err(RandomValidationError::InvalidRandomBound);
    }
    let width = upper - lower;
    let (value, consumed, next) = uniform_below_u64(root, key, cursor, width)?;
    Ok((lower + value, consumed, next))
}

pub fn shuffle<T: Clone>(
    values: &mut [T],
    root: &RootSeed256,
    key: &RandomStreamKeyV1,
    cursor: &RandomStreamCursorV1,
) -> Result<(u64, RandomStreamCursorV1), RandomValidationError> {
    let len = values.len();
    if len <= 1 {
        return Ok((0, *cursor));
    }
    let mut current = *cursor;
    let mut total_consumed = 0u64;
    let mut i = len - 1;
    while i >= 1 {
        let (j, consumed, next) = uniform_below_u64(root, key, &current, i as u64 + 1)?;
        total_consumed += consumed;
        current = next;
        values.swap(i, j as usize);
        i -= 1;
    }
    Ok((total_consumed, current))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RandomStreamKindV1;

    const ALL_ZERO_SEED: &str = "0000000000000000000000000000000000000000000000000000000000000000";

    fn global_key() -> RandomStreamKeyV1 {
        RandomStreamKeyV1::global(RandomStreamKindV1::SyntheticM1)
    }

    #[test]
    fn bound_zero_errors() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let key = global_key();
        let cursor = RandomStreamCursorV1::default();
        assert_eq!(
            uniform_below_u64(&seed, &key, &cursor, 0),
            Err(RandomValidationError::InvalidRandomBound)
        );
    }

    #[test]
    fn bound_one_returns_zero_no_draws() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let key = global_key();
        let cursor = RandomStreamCursorV1::default();
        let (value, consumed, next) = uniform_below_u64(&seed, &key, &cursor, 1).unwrap();
        assert_eq!(value, 0);
        assert_eq!(consumed, 0);
        assert_eq!(next, cursor);
    }

    #[test]
    fn bound_ten_normative_kat() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let key = global_key();
        let cursor = RandomStreamCursorV1::default();
        let (value, consumed, _) = uniform_below_u64(&seed, &key, &cursor, 10).unwrap();
        assert_eq!(value, 7);
        assert_eq!(consumed, 1);
    }

    #[test]
    fn forced_rejection_stub() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let key = global_key();
        let cursor = RandomStreamCursorV1::default();
        let (value, consumed, _) =
            uniform_below_u64_stub(&seed, &key, &cursor, 10, &[0, 6]).unwrap();
        assert_eq!(value, 6);
        assert_eq!(consumed, 2);
    }

    fn uniform_below_u64_stub(
        _root: &RootSeed256,
        _key: &RandomStreamKeyV1,
        cursor: &RandomStreamCursorV1,
        n: u64,
        stub_words: &[u64],
    ) -> Result<(u64, u64, RandomStreamCursorV1), RandomValidationError> {
        if n == 0 {
            return Err(RandomValidationError::InvalidRandomBound);
        }
        if n == 1 {
            return Ok((0, 0, *cursor));
        }
        let threshold = ((1u128 << 64) % (n as u128)) as u64;
        let mut consumed = 0u64;
        for &word in stub_words {
            consumed += 1;
            if word >= threshold {
                return Ok((word % n, consumed, *cursor));
            }
        }
        Err(RandomValidationError::StreamExhausted)
    }

    #[test]
    fn bound_seven_distinguishes_2e64_from_2e128() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let key = global_key();
        let cursor = RandomStreamCursorV1::default();
        let (value, consumed, _) = uniform_below_u64(&seed, &key, &cursor, 7).unwrap();
        assert!(value < 7);
        assert!(consumed >= 1);
    }

    #[test]
    fn checked_range_validates_before_draw() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let key = global_key();
        let cursor = RandomStreamCursorV1::default();
        assert_eq!(
            uniform_range_u64(&seed, &key, &cursor, 5, 5),
            Err(RandomValidationError::InvalidRandomBound)
        );
        assert_eq!(
            uniform_range_u64(&seed, &key, &cursor, 10, 5),
            Err(RandomValidationError::InvalidRandomBound)
        );
    }

    #[test]
    fn shuffle_normative_kat() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let key = global_key();
        let cursor = RandomStreamCursorV1::default();
        let mut values = vec![0, 1, 2, 3, 4];
        let (consumed, _) = shuffle(&mut values, &seed, &key, &cursor).unwrap();
        assert_eq!(values, vec![1, 3, 4, 0, 2]);
        assert_eq!(consumed, 4);
    }

    #[test]
    fn shuffle_length_zero_one_no_draws() {
        let seed = RootSeed256::from_lower_hex(ALL_ZERO_SEED).unwrap();
        let key = global_key();
        let cursor = RandomStreamCursorV1::default();
        let mut empty: Vec<i32> = vec![];
        let (consumed, _) = shuffle(&mut empty, &seed, &key, &cursor).unwrap();
        assert_eq!(consumed, 0);
        let mut one = vec![42];
        let (consumed, _) = shuffle(&mut one, &seed, &key, &cursor).unwrap();
        assert_eq!(consumed, 0);
        assert_eq!(one, vec![42]);
    }
}
