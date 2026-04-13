use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

unsafe extern "C" {
    fn blender_shim_version_major() -> c_int;
    fn blender_shim_version_minor() -> c_int;
    fn blender_shim_version_patch() -> c_int;
    fn blender_shim_version_string(out: *mut c_char, out_size: c_int) -> c_int;
}

pub fn blender_version() -> (i32, i32, i32) {
    unsafe {
        (
            blender_shim_version_major(),
            blender_shim_version_minor(),
            blender_shim_version_patch(),
        )
    }
}

pub fn blender_version_string() -> String {
    let mut buf = vec![0u8; 64];
    unsafe {
        let written = blender_shim_version_string(buf.as_mut_ptr() as *mut c_char, buf.len() as c_int);
        if written < 0 {
            panic!("blender_shim_version_string failed");
        }
        CStr::from_ptr(buf.as_ptr() as *const c_char)
            .to_string_lossy()
            .into_owned()
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let (major, minor, patch) = crate::blender_version();
        let version = crate::blender_version_string();

        println!("Blender version from C++ shim: {version} ({major}.{minor}.{patch})");

        assert!(major >= 1);
        assert_eq!(version, format!("{major}.{minor}.{patch}"));
    }
}
