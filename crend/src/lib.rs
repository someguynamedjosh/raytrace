use std::ffi::CString;

extern {
    fn crendTest();
    fn crendInit(
        required_extensions: *const *const libc::c_char, 
        required_extensions_len: libc::uint32_t
    );
}

pub fn init<T: AsRef<str>>(required_extensions: &[T]) {
    let extension_cstrings: Vec<CString> = required_extensions
        .into_iter()
        .map(|extension| CString::new(extension.as_ref()).expect("Extension name contained null character."))
        .collect();
    let cstring_pointers: Vec<*const libc::c_char> = extension_cstrings
        .iter()
        .map(|cstring| cstring.as_ptr())
        .collect();
    unsafe {
        crendInit(cstring_pointers.as_ptr(), extension_cstrings.len() as libc::uint32_t);
    }
}