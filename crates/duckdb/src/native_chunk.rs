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
    stmt: Option<&'stmt mut Statement<'conn>>,
}

impl<'stmt, 'conn> NativeDataChunkStream<'stmt, 'conn> {
    pub(crate) fn new(stmt: &'stmt mut Statement<'conn>) -> Self {
        Self { stmt: Some(stmt) }
    }

    /// Return the schema reported by DuckDB after execution.
    pub fn get_schema(&self) -> SchemaRef {
        self.stmt
            .as_ref()
            .expect("native chunk iterator always holds a statement")
            .stmt
            .schema()
    }
}

impl Iterator for NativeDataChunkStream<'_, '_> {
    type Item = Result<NativeDataChunk>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.stmt.as_deref()?.step_native_chunk() {
            Ok(Some(chunk)) => Some(Ok(chunk)),
            Ok(None) => {
                self.stmt = None;
                None
            }
            Err(error) => {
                self.stmt = None;
                Some(Err(error))
            }
        }
    }
}
