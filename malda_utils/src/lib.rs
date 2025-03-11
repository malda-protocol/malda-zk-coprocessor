//! Guest utilities for Risc Zero zkVM methods
//!
//! This crate provides common utilities, types, and functions used by guest methods
//! running inside the Risc Zero zkVM. It includes modules for constants, custom types,
//! cryptographic operations, and validation functions.

/// Commonly used constants
pub mod constants;

/// Custom types and data structures
pub mod types;

/// Validation and verification utilities
pub mod validators;

/// Cryptographic operations and primitives
pub mod cryptography;

/// L1 validation
pub mod validators_ethereum_light_client;
