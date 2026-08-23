use std::ptr::NonNull;

use arrow::datatypes::SchemaRef;

use crate::{Result, Statement, ffi};

/// An owned native DuckDB data chunk fetched from a query result.
#[derive(Debug)]
pub struct NativeDataChunk {
    ptr: NonNull<ffi::_duckdb_data_chunk>,
}

impl NativeDataChunk {
    pub(crate) unsafe fn from_raw(ptr: ffi::duckdb_data_chunk) -> Self {
        Self {
            ptr: NonNull::new(ptr).expect("DuckDB returned a null data chunk"),
        }
    }

    /// Return the native DuckDB data chunk handle.
    ///
    /// The handle remains valid only while this owner is alive and must not be
    /// destroyed by the caller.
    #[inline]
    pub fn as_raw(&self) -> ffi::duckdb_data_chunk {
        self.ptr.as_ptr()
    }

    /// Return the number of rows in this chunk.
    #[inline]
    pub fn len(&self) -> usize {
        unsafe { ffi::duckdb_data_chunk_get_size(self.as_raw()) as usize }
    }

    /// Return whether this chunk contains no rows.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Drop for NativeDataChunk {
    fn drop(&mut self) {
        let mut ptr = self.as_raw();
        unsafe { ffi::duckdb_destroy_data_chunk(&mut ptr) };
    }
}

/// A lazy iterator over owned native DuckDB data chunks.
pub struct NativeDataChunkStream<'stmt, 'conn> {
    stmt: &'stmt mut Statement<'conn>,
    exhausted: bool,
}

impl<'stmt, 'conn> NativeDataChunkStream<'stmt, 'conn> {
    pub(crate) fn new(stmt: &'stmt mut Statement<'conn>) -> Self {
        Self { stmt, exhausted: false }
    }

    /// Return the Arrow-compatible schema reported by DuckDB after execution.
    ///
    /// Native chunks can contain DuckDB types, such as VARIANT, that do not
    /// have an Arrow representation. Those chunks remain iterable, while this
    /// method returns an error.
    pub fn get_schema(&self) -> Result<SchemaRef> {
        self.stmt.stmt.try_schema()
    }
}

impl Iterator for NativeDataChunkStream<'_, '_> {
    type Item = Result<NativeDataChunk>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.exhausted {
            return None;
        }
        match self.stmt.step_native_chunk() {
            Ok(Some(chunk)) => Some(Ok(chunk)),
            Ok(None) => {
                self.exhausted = true;
                None
            }
            Err(error) => {
                self.exhausted = true;
                Some(Err(error))
            }
        }
    }
}
