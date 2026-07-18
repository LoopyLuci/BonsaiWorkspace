//! bmn-encoder: concrete hardware/software video encoder wrappers, an
//! adaptive bitrate ladder generator, and a round-robin encoder pool,
//! built on top of the shared BMN media types in `bmn-common`.

pub mod hardware;
pub mod ladder;
pub mod pool;
pub mod software;

pub use hardware::HardwareEncoder;
pub use ladder::{AdaptiveBitrateladder, BitrateProfile};
pub use pool::EncoderPool;
pub use software::SoftwareEncoder;

/// Which concrete encoder backend to drive. Reuses `bmn-common`'s
/// [`EncoderType`](bmn_common::encoder::EncoderType) rather than duplicating
/// an equivalent enum.
pub use bmn_common::encoder::EncoderType as EncoderBackend;
