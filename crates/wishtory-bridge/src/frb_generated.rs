// Placeholder so the crate type-checks before the first code generation.
// `flutter_rust_bridge_codegen generate` overwrites this whole file.
#![allow(dead_code)]
use std::marker::PhantomData;

pub struct StreamSink<T>(PhantomData<T>);
impl<T> Clone for StreamSink<T> {
    fn clone(&self) -> Self {
        StreamSink(PhantomData)
    }
}
impl<T> StreamSink<T> {
    pub fn add(&self, _event: T) -> Result<(), ()> {
        Ok(())
    }
}
