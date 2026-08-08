//! Stores API keys in the current Windows user's Credential Manager vault.

#[cfg(target_os = "windows")]
mod windows_store {
    use std::{ffi::c_void, ptr};

    use windows::{
        core::{HRESULT, PCWSTR, PWSTR},
        Win32::{
            Foundation::ERROR_NOT_FOUND,
            Security::Credentials::{
                CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW,
                CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
            },
        },
    };
    use zeroize::Zeroize;

    const QWEN_TARGET: &str = "ListenToMe/QwenApiKey";
    const USER_NAME: &str = "ListenToMe";

    pub struct CredentialStore;

    impl CredentialStore {
        pub fn save_qwen_api_key(api_key: &str) -> Result<(), String> {
            if api_key.trim().is_empty() {
                return Err("API key cannot be empty".to_owned());
            }

            let mut target = wide(QWEN_TARGET);
            let mut user_name = wide(USER_NAME);
            let mut blob = api_key.as_bytes().to_vec();
            let blob_size = u32::try_from(blob.len())
                .map_err(|_| "API key is too large to store securely".to_owned())?;

            let credential = CREDENTIALW {
                Type: CRED_TYPE_GENERIC,
                TargetName: PWSTR(target.as_mut_ptr()),
                CredentialBlobSize: blob_size,
                CredentialBlob: blob.as_mut_ptr(),
                Persist: CRED_PERSIST_LOCAL_MACHINE,
                UserName: PWSTR(user_name.as_mut_ptr()),
                ..Default::default()
            };

            // SAFETY: All pointers reference live buffers for the duration of the
            // call. CredWriteW copies the credential before returning.
            let result = unsafe { CredWriteW(&credential, 0) }
                .map_err(|_| "Windows could not store the API key securely".to_owned());
            blob.zeroize();
            result
        }

        pub fn qwen_api_key() -> Result<String, String> {
            let target = wide(QWEN_TARGET);
            let mut raw = ptr::null_mut();

            // SAFETY: target is null terminated and raw is a valid output pointer.
            unsafe { CredReadW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, None, &mut raw) }
                .map_err(|error| {
                    if is_not_found(error.code()) {
                        "Qwen API key is not configured".to_owned()
                    } else {
                        "Windows could not read the stored API key".to_owned()
                    }
                })?;

            let credential = CredentialGuard(raw);
            // SAFETY: CredentialGuard owns the block returned by CredReadW, and
            // the credential blob remains valid until the guard is dropped.
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    (*credential.0).CredentialBlob,
                    (*credential.0).CredentialBlobSize as usize,
                )
            };
            String::from_utf8(bytes.to_vec())
                .map_err(|_| "The stored Qwen API key is invalid".to_owned())
        }

        pub fn has_qwen_api_key() -> Result<bool, String> {
            match Self::qwen_api_key() {
                Ok(mut key) => {
                    wipe_string(&mut key);
                    Ok(true)
                }
                Err(error) if error == "Qwen API key is not configured" => Ok(false),
                Err(error) => Err(error),
            }
        }

        pub fn delete_qwen_api_key() -> Result<(), String> {
            let target = wide(QWEN_TARGET);
            // SAFETY: target is a live null-terminated string.
            match unsafe { CredDeleteW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, None) } {
                Ok(()) => Ok(()),
                Err(error) if is_not_found(error.code()) => Ok(()),
                Err(_) => Err("Windows could not remove the stored API key".to_owned()),
            }
        }
    }

    struct CredentialGuard(*mut CREDENTIALW);

    impl Drop for CredentialGuard {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: The pointer was allocated by CredReadW and is freed once.
                unsafe { CredFree(self.0.cast::<c_void>()) };
            }
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn is_not_found(code: HRESULT) -> bool {
        code == HRESULT::from_win32(ERROR_NOT_FOUND.0)
    }

    pub fn wipe_string(value: &mut String) {
        value.zeroize();
    }
}

#[cfg(target_os = "windows")]
pub use windows_store::{wipe_string, CredentialStore};

#[cfg(not(target_os = "windows"))]
pub struct CredentialStore;

#[cfg(not(target_os = "windows"))]
impl CredentialStore {
    pub fn save_qwen_api_key(_api_key: &str) -> Result<(), String> {
        Err("Secure credential storage is currently supported on Windows only".to_owned())
    }

    pub fn qwen_api_key() -> Result<String, String> {
        Err("Secure credential storage is currently supported on Windows only".to_owned())
    }

    pub fn has_qwen_api_key() -> Result<bool, String> {
        Ok(false)
    }

    pub fn delete_qwen_api_key() -> Result<(), String> {
        Ok(())
    }
}
