mod internal;
pub mod pkg;

pub use internal::naming::doc_id_from_title;
pub use internal::storage::{DocId, DocTitle, FileStorage, LocalFileStorage, StoredDoc};
