use std::{ffi::c_void, ptr::NonNull};

unsafe extern "C" {
    fn openbox_key_new() -> *mut c_void;
    fn openbox_key_push(analyzer: *mut c_void, samples: *const f32, length: usize) -> bool;
    fn openbox_key_finish(analyzer: *mut c_void) -> i32;
    fn openbox_key_delete(analyzer: *mut c_void);
}

pub(super) struct Key(NonNull<c_void>);

impl Key {
    pub fn new() -> Result<Self, String> {
        // The pointer has one owner and stays on the analysis thread until Drop.
        NonNull::new(unsafe { openbox_key_new() })
            .map(Self)
            .ok_or("Cannot initialize libkeyfinder".into())
    }

    pub fn push(&mut self, samples: &[f32]) -> Result<(), String> {
        // The bridge borrows this slice synchronously and never retains its address.
        if unsafe { openbox_key_push(self.0.as_ptr(), samples.as_ptr(), samples.len()) } {
            Ok(())
        } else {
            Err("libkeyfinder could not process audio".into())
        }
    }

    pub fn finish(&mut self) -> Result<String, String> {
        let key = unsafe { openbox_key_finish(self.0.as_ptr()) };
        const NAMES: [&str; 25] = [
            "A", "Am", "Bb", "Bbm", "B", "Bm", "C", "Cm", "Db", "Dbm", "D", "Dm", "Eb", "Ebm", "E",
            "Em", "F", "Fm", "Gb", "Gbm", "G", "Gm", "Ab", "Abm", "",
        ];
        NAMES
            .get(key as usize)
            .map(|s| s.to_string())
            .ok_or("libkeyfinder could not finish analysis".into())
    }
}

impl Drop for Key {
    fn drop(&mut self) {
        unsafe { openbox_key_delete(self.0.as_ptr()) };
    }
}
