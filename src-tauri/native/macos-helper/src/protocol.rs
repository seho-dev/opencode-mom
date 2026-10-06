use std::time::Duration;

pub const HELLO_MAGIC: &[u8; 8] = b"OMLID001";
pub const HELLO_LEN: usize = 40;
pub const REQUEST_LEN: usize = 24;
pub const ACK_LEN: usize = 16;
pub const MAX_TTL: Duration = Duration::from_secs(10);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Busy(u64),
    Idle,
    Stop,
}

pub fn hello(token: &[u8; 32]) -> [u8; HELLO_LEN] {
    let mut bytes = [0; HELLO_LEN];
    bytes[..8].copy_from_slice(HELLO_MAGIC);
    bytes[8..].copy_from_slice(token);
    bytes
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn authenticate(bytes: &[u8], token: &[u8; 32]) -> bool {
    bytes.len() == HELLO_LEN
        && bytes[..8] == HELLO_MAGIC[..]
        && bytes[8..]
            .iter()
            .zip(token)
            .fold(0u8, |difference, (a, b)| difference | (a ^ b))
            == 0
}

pub fn request(seq: u64, operation: Operation) -> [u8; REQUEST_LEN] {
    let mut bytes = [0; REQUEST_LEN];
    bytes[0] = match operation {
        Operation::Busy(ttl) => {
            bytes[16..].copy_from_slice(&ttl.to_le_bytes());
            1
        }
        Operation::Idle => 2,
        Operation::Stop => 3,
    };
    bytes[8..16].copy_from_slice(&seq.to_le_bytes());
    bytes
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn parse_request(bytes: &[u8], expected_seq: u64) -> Result<Operation, &'static str> {
    if bytes.len() != REQUEST_LEN || bytes[1..8] != [0; 7] {
        return Err("Invalid frame");
    }
    let seq = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
    let ttl = u64::from_le_bytes(bytes[16..24].try_into().unwrap());
    if seq == 0 || seq != expected_seq || seq == u64::MAX {
        return Err("Invalid sequence");
    }
    match (bytes[0], ttl) {
        (1, 1..=10_000) if ttl <= MAX_TTL.as_millis() as u64 => Ok(Operation::Busy(ttl)),
        (2, 0) => Ok(Operation::Idle),
        (3, 0) => Ok(Operation::Stop),
        _ => Err("Invalid operation or lease"),
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn ack(seq: u64, flag: Option<bool>, success: bool) -> [u8; ACK_LEN] {
    let mut bytes = [0; ACK_LEN];
    bytes[0] = u8::from(!success);
    bytes[1] = flag.map_or(2, u8::from);
    bytes[8..].copy_from_slice(&seq.to_le_bytes());
    bytes
}

pub fn valid_ack(bytes: &[u8], seq: u64) -> bool {
    bytes.len() == ACK_LEN
        && bytes[0] <= 1
        && bytes[1] <= 2
        && bytes[2..8] == [0; 6]
        && u64::from_le_bytes(bytes[8..16].try_into().unwrap()) == seq
}

pub fn parse_ack(bytes: &[u8], seq: u64) -> Result<bool, &'static str> {
    if !valid_ack(bytes, seq) {
        return Err("Invalid helper acknowledgment");
    }
    if bytes[0] != 0 || bytes[1] > 1 {
        return Err("The helper could not verify the power policy");
    }
    Ok(bytes[1] == 1)
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn decode_token(hex: &str) -> Result<[u8; 32], &'static str> {
    if hex.len() != 64 || !hex.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("Invalid token");
    }
    let mut token = [0; 32];
    for (index, byte) in token.iter_mut().enumerate() {
        *byte =
            u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).map_err(|_| "Invalid token")?;
    }
    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_are_bounded_authenticated_and_sequenced() {
        let token = [0xab; 32];
        assert!(authenticate(&hello(&token), &token));
        assert!(!authenticate(&hello(&[0xac; 32]), &token));
        assert!(!authenticate(&[0; HELLO_LEN + 1], &token));
        for operation in [Operation::Busy(10_000), Operation::Idle, Operation::Stop] {
            let frame = request(1, operation);
            assert_eq!(parse_request(&frame, 1), Ok(operation));
            assert!(parse_request(&frame, 2).is_err());
            assert!(parse_request(&frame[..23], 1).is_err());
        }
        for ttl in [0, 10_001, u64::MAX] {
            assert!(parse_request(&request(1, Operation::Busy(ttl)), 1).is_err());
        }
        assert!(parse_request(&request(0, Operation::Idle), 0).is_err());
        assert!(parse_request(&request(u64::MAX, Operation::Idle), u64::MAX).is_err());
        assert!(parse_request(&[0; REQUEST_LEN + 1], 1).is_err());
        assert_eq!(parse_ack(&ack(5, Some(true), true), 5), Ok(true));
        assert!(parse_ack(&ack(5, Some(true), false), 5).is_err());
        assert!(parse_ack(&ack(5, Some(true), true), 6).is_err());
        assert!(!valid_ack(&[0; ACK_LEN + 1], 1));
        assert!(decode_token("../bad").is_err());
        assert_eq!(decode_token(&"ab".repeat(32)), Ok(token));
    }
}
