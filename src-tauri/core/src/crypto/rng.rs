//! OS CSPRNG access.
//!
//! All randomness (keys, nonces, salts) comes from the operating system's
//! secure generator via `getrandom`. We deliberately avoid userspace PRNGs for
//! anything security-relevant.

use crate::error::{CoreError, Result};

/// Fill `buf` with cryptographically secure random bytes.
pub fn fill(buf: &mut [u8]) -> Result<()> {
    getrandom::getrandom(buf).map_err(|_| CoreError::Random)
}

/// Return `N` cryptographically secure random bytes.
pub fn random_array<const N: usize>() -> Result<[u8; N]> {
    let mut buf = [0u8; N];
    fill(&mut buf)?;
    Ok(buf)
}

/// Return a `Vec` of `n` cryptographically secure random bytes.
pub fn random_vec(n: usize) -> Result<Vec<u8>> {
    let mut buf = vec![0u8; n];
    fill(&mut buf)?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_distinct_values() {
        let a = random_array::<32>().unwrap();
        let b = random_array::<32>().unwrap();
        assert_ne!(a, b, "two random draws must differ");
        assert!(a.iter().any(|&x| x != 0));
    }
}
