use std::ffi::{CStr};
use std::os::raw::c_char;

pub fn convert_raw_cstring(raw_cstring: &[c_char]) -> String {
    let cstring = unsafe { CStr::from_ptr(raw_cstring.as_ptr()) };
    cstring
        .to_str()
        .expect("Failed to convert C string.")
        .to_owned()
}