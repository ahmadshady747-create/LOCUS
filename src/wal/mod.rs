//! WAL Subsystem — High-Throughput Binary WAL, CRC32-C Framed Log Engine & Crash Recovery.

#![forbid(unsafe_code)]

pub mod compactor;
pub mod log;
pub mod recovery;

pub use compactor::{
    CheckpointId, CheckpointIssuer, CheckpointSnapshot, CompactorConfig, CompactorError,
    CompactorPolicy, CompactorStats, WalLogCompactor,
};
pub use log::{
    crc32c, BinaryFrameCodec, BinaryWalWriter, FsyncPolicy, TxId, WalConfig, WalError, WalFrame,
    WalFrameHeader, WalFrameType, WalVersion,
};
pub use recovery::{
    RecoveredTransaction, RecoveryMode, RecoveryReceipt, ScanResult, ScannedTx,
    TransactionRecoveryStatus, WalCheckpointPayload, WalCrashRecoveryManager, WalLogScanner,
    WalMutationPayload, WalReplayEngine,
};
