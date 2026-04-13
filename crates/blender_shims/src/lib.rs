use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

unsafe extern "C" {
    fn blender_shim_version_major() -> c_int;
    fn blender_shim_version_minor() -> c_int;
    fn blender_shim_version_patch() -> c_int;
    fn blender_shim_version_string(out: *mut c_char, out_size: c_int) -> c_int;

    fn blender_shim_normalize_vec3(input: *const f32, output: *mut f32) -> f32;
    fn blender_shim_dot_vec3(a: *const f32, b: *const f32) -> f32;
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

pub fn normalize_vec3(input: [f32; 3]) -> ([f32; 3], f32) {
    let mut out = [0.0_f32; 3];
    let len = unsafe { blender_shim_normalize_vec3(input.as_ptr(), out.as_mut_ptr()) };
    (out, len)
}

pub fn dot_vec3(a: [f32; 3], b: [f32; 3]) -> f32 {
    unsafe { blender_shim_dot_vec3(a.as_ptr(), b.as_ptr()) }
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


    #[test]
    fn normalize_vec3_works() {
        let (out, len) = normalize_vec3([3.0, 4.0, 0.0]);

        approx_eq(len, 5.0, 1e-6);
        approx_eq(out[0], 0.6, 1e-6);
        approx_eq(out[1], 0.8, 1e-6);
        approx_eq(out[2], 0.0, 1e-6);
    }

    #[test]
    fn dot_vec3_works() {
        let dot = dot_vec3([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]);
        approx_eq(dot, 32.0, 1e-6);
    }

    fn approx_eq(a: f32, b: f32, eps: f32) {
        assert!(
            (a - b).abs() <= eps,
            "expected {a} ~= {b} (eps={eps})"
        );
    }
}
