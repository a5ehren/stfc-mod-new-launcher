//! Paths to be verified on a live Windows install (spec §9, plan Task 1).
//! PLACEHOLDER: pending the Windows research spike — do not ship Windows
//! multi-instance builds until these are verified.
pub const PLAYERPREFS_REG_KEY: &str = r"Software\UNVERIFIED\UNVERIFIED";
pub const SESSION_DIR_RELATIVE: &str = r"AppData\LocalLow\UNVERIFIED\UNVERIFIED";
