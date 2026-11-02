//! Typed errors for the audio engine.
//!
//! The engine used to report every failure as an `anyhow::Error`, which
//! meant a caller could only print it. These variants keep the failure kind
//! so the difference between "mpv is not there" and "one command was
//! rejected" survives to the caller.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AudioError {
    /// mpv handed back a null handle, so the library is unusable.
    #[error("failed to create an mpv handle")]
    HandleCreate,

    /// The handle existed but refused to initialise, with mpv's own code.
    #[error("mpv failed to initialise, code {0}")]
    InitFailed(i32),

    /// mpv rejected a command. The command is trimmed because it always
    /// carries a trailing newline.
    #[error("mpv rejected the command `{command}`, code {code}")]
    CommandRejected { command: String, code: i32 },

    /// The command string held a NUL byte and could not be built.
    #[error("invalid command string: {0}")]
    InvalidCommand(#[from] std::ffi::NulError),
}

impl AudioError {
    /// The failure mpv reports when a command is refused.
    pub fn command_rejected(command: &str, code: i32) -> Self {
        Self::CommandRejected {
            command: command.trim().to_string(),
            code,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_is_a_sentence_not_a_debug_dump() {
        assert_eq!(
            AudioError::HandleCreate.to_string(),
            "failed to create an mpv handle"
        );
        assert_eq!(
            AudioError::InitFailed(-1).to_string(),
            "mpv failed to initialise, code -1"
        );
    }

    #[test]
    fn a_rejected_command_names_itself() {
        let err = AudioError::command_rejected("seek 10 absolute\n", -11);
        assert_eq!(
            err.to_string(),
            "mpv rejected the command `seek 10 absolute`, code -11"
        );
        // The trailing newline must not leak into the message.
        assert!(!err.to_string().contains('\n'));
    }

    #[test]
    fn a_nul_byte_reads_through() {
        let nul = std::ffi::CString::new("bad\0command").unwrap_err();
        let err = AudioError::from(nul);
        assert!(err.to_string().starts_with("invalid command string:"));
    }

    #[test]
    fn it_is_a_real_error() {
        fn assert_error<E: std::error::Error + Send + Sync + 'static>(_: &E) {}
        assert_error(&AudioError::HandleCreate);
    }
}
