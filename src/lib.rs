use std::ops::{Deref, DerefMut};

use magnus::rb_sys::FromRawValue;

struct RubyAllocator {}

unsafe impl allocator_api2::alloc::Allocator for RubyAllocator {
    fn allocate(
        &self,
        layout: std::alloc::Layout,
    ) -> Result<std::ptr::NonNull<[u8]>, allocator_api2::alloc::AllocError> {
        let ptr = unsafe {
            rb_sys::ruby_xmalloc(
                layout
                    .size()
                    .try_into()
                    .map_err(|_| allocator_api2::alloc::AllocError)?,
            )
        };
        Ok(std::ptr::NonNull::slice_from_raw_parts(
            unsafe { std::ptr::NonNull::new_unchecked(ptr as *mut u8) },
            layout.size(),
        ))
    }

    unsafe fn deallocate(&self, ptr: std::ptr::NonNull<u8>, _: std::alloc::Layout) {
        rb_sys::ruby_xfree(ptr.as_ptr() as *mut std::ffi::c_void);
    }
}

pub struct String {
    buf: allocator_api2::vec::Vec<u8, RubyAllocator>,
}

impl String {
    pub fn new() -> Self {
        Self {
            buf: allocator_api2::vec::Vec::new_in(RubyAllocator {}),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buf: allocator_api2::vec::Vec::with_capacity_in(capacity, RubyAllocator {}),
        }
    }

    pub fn from_str(s: &str) -> Self {
        let mut result = Self {
            buf: allocator_api2::vec::Vec::with_capacity_in(s.len(), RubyAllocator {}),
        };


        result.push_str(s);

        result
    }

    pub fn push(&mut self, c: char) {
        match c.len_utf8() {
            1 => self.buf.push(c as u8),
            _ => self
                .buf
                .extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes()),
        }
    }

    pub fn push_str(&mut self, s: &str) {
        self.buf.extend_from_slice(s.as_bytes());
    }

    pub fn clear(&mut self) {
        self.buf.clear();
    }

    pub fn insert(&mut self, idx: usize, c: char) {
        unsafe {
            self.insert_bytes(idx, c.encode_utf8(&mut [0; 4]).as_bytes());
        }
    }

    pub fn insert_str(&mut self, idx: usize, s: &str) {
        unsafe { self.insert_bytes(idx, s.as_bytes()) };
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.buf) }
    }

    pub fn as_mut_str(&mut self) -> &mut str {
        unsafe { std::str::from_utf8_unchecked_mut(&mut self.buf) }
    }

    pub fn remove(&mut self, idx: usize) -> char {
        let ch = match self[idx..].chars().next() {
            Some(ch) => ch,
            None => panic!("cannot remove a char from the end of a string"),
        };

        let next = idx + ch.len_utf8();
        let len = self.len();
        unsafe {
            std::ptr::copy(
                self.buf.as_ptr().add(next),
                self.buf.as_mut_ptr().add(idx),
                len - next,
            );
            self.buf.set_len(len - (next - idx));
        }
        ch
    }

    pub fn pop(&mut self) -> Option<char> {
        let ch = self.chars().rev().next()?;
        let newlen = self.len() - ch.len_utf8();
        unsafe {
            self.buf.set_len(newlen);
        }
        Some(ch)
    }

    pub fn into_rstring(self) -> magnus::RString {
        let raw_value = unsafe {
            rb_sys::rb_utf8_str_new(self.buf.as_ptr() as *const i8, self.buf.len().try_into().unwrap())
        };

        std::mem::forget(self);

        magnus::RString::from_value(unsafe { magnus::Value::from_raw(raw_value) }).unwrap()
    }

    unsafe fn insert_bytes(&mut self, idx: usize, bytes: &[u8]) {
        let len = self.len();
        let amt = bytes.len();
        self.buf.reserve(amt);

        std::ptr::copy(
            self.buf.as_ptr().add(idx),
            self.buf.as_mut_ptr().add(idx + amt),
            len - idx,
        );
        std::ptr::copy(bytes.as_ptr(), self.buf.as_mut_ptr().add(idx), amt);
        self.buf.set_len(len + amt);
    }
}

impl Deref for String {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl DerefMut for String {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_str()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_empty() {
        let s = super::String::new();
        assert_eq!(s.len(), 0);
        assert_eq!(s.as_str(), "");
    }
}
