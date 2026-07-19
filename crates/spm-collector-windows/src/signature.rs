//! Authenticode verification via `WinVerifyTrust`. Returns whether an
//! executable is signed and whether the signing chain validates, matching
//! the spec's "Digital signature information (Windows)" / "Code signing
//! status" requirement.
//!
//! Extracting the human-readable signer name requires walking the
//! certificate returned by `CryptQueryObject`/`CertGetNameString`; that is
//! a documented follow-up (see `docs/collector-architecture.md`) — this
//! module honestly reports `signer: None` rather than fabricating a value.

use spm_core::SignatureStatus;
use windows::core::{GUID, PCWSTR, PWSTR};
use windows::Win32::Foundation::{HANDLE, HWND};
use windows::Win32::Security::WinTrust::{
    WinVerifyTrust, WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_DATA_0, WINTRUST_DATA_UICONTEXT,
    WINTRUST_FILE_INFO, WTD_CACHE_ONLY_URL_RETRIEVAL, WTD_CHOICE_FILE, WTD_REVOKE_NONE, WTD_STATEACTION_VERIFY,
    WTD_UI_NONE,
};

use crate::util::to_wide;

pub fn check_signature(path: &str) -> SignatureStatus {
    let wide_path = to_wide(path);
    let file_path = PCWSTR::from_raw(wide_path.as_ptr());

    let mut file_info = WINTRUST_FILE_INFO {
        cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: file_path,
        hFile: HANDLE::default(),
        pgKnownSubject: std::ptr::null_mut(),
    };

    let mut data = WINTRUST_DATA {
        cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
        pPolicyCallbackData: std::ptr::null_mut(),
        pSIPClientData: std::ptr::null_mut(),
        dwUIChoice: WTD_UI_NONE,
        fdwRevocationChecks: WTD_REVOKE_NONE,
        dwUnionChoice: WTD_CHOICE_FILE,
        Anonymous: WINTRUST_DATA_0 { pFile: &mut file_info as *mut _ },
        dwStateAction: WTD_STATEACTION_VERIFY,
        hWVTStateData: HANDLE::default(),
        pwszURLReference: PWSTR::null(),
        dwProvFlags: WTD_CACHE_ONLY_URL_RETRIEVAL,
        dwUIContext: WINTRUST_DATA_UICONTEXT(0),
        pSignatureSettings: std::ptr::null_mut(),
    };

    let mut action: GUID = WINTRUST_ACTION_GENERIC_VERIFY_V2;
    let status = unsafe { WinVerifyTrust(HWND::default(), &mut action, &mut data as *mut _ as *mut _) };

    // WinVerifyTrust returns a Win32 error code as its "result"; 0 means
    // ERROR_SUCCESS (trusted). TRUST_E_NOSIGNATURE and friends indicate an
    // unsigned or otherwise untrusted file rather than a hard failure.
    match status {
        0 => SignatureStatus::Signed,
        _ => {
            // Distinguish "no signature at all" from "signed but the
            // chain didn't validate" using the well-known error range;
            // anything else is reported as unknown rather than guessed.
            const TRUST_E_NOSIGNATURE: i32 = 0x800B0100u32 as i32;
            const TRUST_E_SUBJECT_NOT_TRUSTED: i32 = 0x800B0004u32 as i32;
            const TRUST_E_BAD_DIGEST: i32 = 0x80096010u32 as i32;
            const CERT_E_UNTRUSTEDROOT: i32 = 0x800B0109u32 as i32;
            const CERT_E_EXPIRED: i32 = 0x800B0101u32 as i32;
            match status {
                TRUST_E_NOSIGNATURE => SignatureStatus::Unsigned,
                TRUST_E_SUBJECT_NOT_TRUSTED | TRUST_E_BAD_DIGEST | CERT_E_UNTRUSTEDROOT | CERT_E_EXPIRED => {
                    SignatureStatus::SignedUntrusted
                }
                _ => SignatureStatus::Unknown,
            }
        }
    }
}
