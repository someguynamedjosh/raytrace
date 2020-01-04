use std::ffi::CString;

extern {
    fn crendTest();
    fn crendInit(
        required_extensions: *const *const libc::c_char, 
        required_extensions_len: u32,
        use_validation_layers: bool,
    ) -> u32;
    fn crendDestroy();
}

pub struct Renderer {

}

impl Renderer {
    pub fn new<T: AsRef<str>>(required_extensions: &[T]) -> Result<Renderer, ()> {
        let extension_cstrings: Vec<CString> = required_extensions
            .into_iter()
            .map(|extension| CString::new(extension.as_ref()).expect("Extension name contained null character."))
            .collect();
        let cstring_pointers: Vec<*const libc::c_char> = extension_cstrings
            .iter()
            .map(|cstring| cstring.as_ptr())
            .collect();
        let result = unsafe {
            crendInit(
                cstring_pointers.as_ptr(), 
                extension_cstrings.len() as u32, 
                cfg!(debug_assertions)
            )
        };
        if result == 0 {
            Ok(Renderer { })
        } else {
            Err(())
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            crendDestroy();
        }
    }
}