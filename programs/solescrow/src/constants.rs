

/// Minimum time buffer for end dates (1 hour in seconds)
pub const MIN_END_TIME_BUFFER: i64 = 3600;

/// Seeds for PDA derivation
pub mod seeds {
    /// Asymmetric escrow PDA seed
    pub const ASYM_ESCROW: &[u8] = b"asym_escrow";
    
    /// Program config PDA seed
    pub const PROGRAM_CONFIG: &[u8] = b"program_config";
    
    /// Escrow vault PDA seed
    pub const ESCROW_VAULT: &[u8] = b"escrow_vault";
}