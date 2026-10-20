use std::time::{Duration, Instant};

use crate::error::AppError;

use super::token_usage::transport;

pub fn reload_via_cli() -> Result<(), AppError> {
    let deadline = Instant::now() + Duration::from_secs(20);
    transport::connect(deadline)?.reload(deadline)
}

#[cfg(test)]
mod tests;
