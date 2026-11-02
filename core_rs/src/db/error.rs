//! Typed errors for the local database.
//!
//! Storage failures are one of two shapes: SQLite refused the statement, or
//! the file system refused the directory. Keeping them apart lets a caller
//! tell a corrupt database from a read-only home without parsing a string.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    /// SQLite rejected the statement, the connection, or the schema.
    #[error("database error: {0}")]
    Sql(#[from] rusqlite::Error),

    /// The data directory could not be created or read.
    #[error("file system error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;

    #[test]
    fn display_is_a_sentence_not_a_debug_dump() {
        let io = StorageError::Io(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "read-only file system",
        ));
        assert_eq!(io.to_string(), "file system error: read-only file system");
    }

    #[test]
    fn sql_failures_read_as_sentences() {
        let err = StorageError::Sql(rusqlite::Error::QueryReturnedNoRows);
        assert!(err.to_string().starts_with("database error:"));
        // The variant name is the Debug form; it must not leak into Display.
        assert!(!err.to_string().contains("QueryReturnedNoRows"));
    }

    #[test]
    fn errors_keep_their_source() {
        // `#[from]` also makes the wrapped error the source, so the original
        // cause survives the conversion.
        let io = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err = StorageError::from(io);
        assert!(err.to_string().starts_with("file system error:"));
        assert!(err.source().is_some());
    }

    #[test]
    fn a_caller_can_tell_the_two_kinds_apart() {
        // This is the point of the enum: kind matching instead of string
        // matching, so a locked database can be retried and a missing
        // directory cannot.
        let sql = StorageError::Sql(rusqlite::Error::QueryReturnedNoRows);
        let io = StorageError::Io(std::io::Error::other("boom"));
        assert!(matches!(sql, StorageError::Sql(_)));
        assert!(matches!(io, StorageError::Io(_)));
    }

    #[test]
    fn it_is_a_real_error() {
        fn assert_error<E: std::error::Error + Send + Sync + 'static>(_: &E) {}
        assert_error(&StorageError::Io(std::io::Error::other("x")));
    }
}
