/*
 *  Copyright (c) 2026 Proton AG
 *  This file is part of Proton AG and Proton Pass.
 *
 *  Proton Pass is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Proton Pass is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Proton Pass.  If not, see <https://www.gnu.org/licenses/>.
 *
 */

//! Windows-specific helpers for setting restrictive ACLs on files and named pipes.
//!
//! On Windows, files and named pipes created with default ACLs may be readable
//! by other users on the same system. These helpers replace the DACL with one
//! that grants only the current user access, mirroring Unix 0600 semantics.

use anyhow::{Result, anyhow};
use std::path::Path;

use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Security::Authorization::SetNamedSecurityInfoW;
use windows_sys::Win32::Security::{
    ACCESS_ALLOWED_ACE, ACL, ACL_REVISION, AddAccessAllowedAce, DACL_SECURITY_INFORMATION,
    GetLengthSid, GetTokenInformation, InitializeAcl, OpenProcessToken,
    PROTECTED_DACL_SECURITY_INFORMATION, TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

/// Full control — equivalent to Unix `0600` for the owning user.
const GENERIC_ALL: u32 = 0x10000000;

/// RAII guard that closes a Windows HANDLE when dropped.
struct HandleGuard(HANDLE);

impl Drop for HandleGuard {
    fn drop(&mut self) {
        // SAFETY: self.0 is a valid open handle obtained from OpenProcessToken.
        unsafe {
            CloseHandle(self.0);
        }
    }
}

/// Retrieves a heap buffer containing the `TOKEN_USER` structure for the current process.
///
/// # Safety
/// The returned buffer must outlive any pointer derived from it.
unsafe fn current_user_token_buf() -> Result<Vec<u8>> {
    let mut token: HANDLE = INVALID_HANDLE_VALUE;
    if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
        return Err(anyhow!(
            "OpenProcessToken failed (error {})",
            GetLastError()
        ));
    }
    let _guard = HandleGuard(token);

    // First call: retrieve the required buffer size.
    let mut needed = 0u32;
    GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut needed);
    if needed == 0 {
        return Err(anyhow!(
            "GetTokenInformation size query failed (error {})",
            GetLastError()
        ));
    }

    let mut buf = vec![0u8; needed as usize];
    if GetTokenInformation(
        token,
        TokenUser,
        buf.as_mut_ptr().cast(),
        needed,
        &mut needed,
    ) == 0
    {
        return Err(anyhow!(
            "GetTokenInformation failed (error {})",
            GetLastError()
        ));
    }

    Ok(buf)
}

/// Builds an ACL buffer containing a single `ACCESS_ALLOWED_ACE` for the SID
/// embedded at the start of `token_user_buf`, granting `access_mask`.
///
/// Returns `(acl_buf, acl_ptr)` where `acl_ptr` points into `acl_buf`.
///
/// # Safety
/// `token_user_buf` must be a valid `TOKEN_USER` buffer as returned by
/// `GetTokenInformation(TokenUser, …)`.
unsafe fn build_acl(token_user_buf: &[u8], access_mask: u32) -> Result<(Vec<u8>, *mut ACL)> {
    let token_user = &*(token_user_buf.as_ptr() as *const TOKEN_USER);
    let sid = token_user.User.Sid;
    let sid_len = GetLengthSid(sid);

    // ACL size = ACL header + ACCESS_ALLOWED_ACE header + SID bytes.
    // ACCESS_ALLOWED_ACE already contains a `SidStart: u32` field that
    // overlaps the first DWORD of the SID, so subtract sizeof(u32).
    let acl_size = (std::mem::size_of::<ACL>() + std::mem::size_of::<ACCESS_ALLOWED_ACE>()
        - std::mem::size_of::<u32>()
        + sid_len as usize) as u32;

    let mut acl_buf = vec![0u8; acl_size as usize];
    let acl = acl_buf.as_mut_ptr() as *mut ACL;

    if InitializeAcl(acl, acl_size, ACL_REVISION) == 0 {
        return Err(anyhow!("InitializeAcl failed (error {})", GetLastError()));
    }

    if AddAccessAllowedAce(acl, ACL_REVISION, access_mask, sid) == 0 {
        return Err(anyhow!(
            "AddAccessAllowedAce failed (error {})",
            GetLastError()
        ));
    }

    Ok((acl_buf, acl))
}

/// Restrict a file (or directory) so that only the current user has access.
///
/// Replaces the DACL with a single `ACCESS_ALLOWED_ACE` granting the current
/// user `GENERIC_ALL`, and sets `PROTECTED_DACL_SECURITY_INFORMATION` to strip
/// inherited ACEs — mirroring Unix `chmod 0600`.
///
/// Returns an error if the Windows API calls fail; callers may choose to log
/// a warning rather than aborting, since this is a hardening step rather than
/// a functional requirement.
pub fn restrict_file_to_current_user(path: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Security::Authorization::SE_FILE_OBJECT;

    // SAFETY: all pointers are derived from owned buffers that remain live
    // for the duration of the SetNamedSecurityInfoW call.
    unsafe {
        let token_buf = current_user_token_buf()?;
        let (_acl_buf, acl) = build_acl(&token_buf, GENERIC_ALL)?;

        let path_wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION:
        //   replace the DACL and block inheritance from the parent directory.
        let err = SetNamedSecurityInfoW(
            path_wide.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(), // owner: unchanged
            std::ptr::null_mut(), // group: unchanged
            acl,
            std::ptr::null_mut(), // SACL: unchanged
        );

        if err != 0 {
            return Err(anyhow!("SetNamedSecurityInfoW failed (error {})", err));
        }
    }

    Ok(())
}

/// Restrict a named pipe so that only the current user can connect to it.
///
/// `pipe_name` should be the full Win32 pipe path, e.g.
/// `r"\\.\pipe\openssh-ssh-agent"`.
///
/// Replaces the DACL with a single `ACCESS_ALLOWED_ACE` granting the current
/// user `GENERIC_ALL`.  On failure a warning should be logged; the pipe
/// remains functional but without the ACL hardening.
pub fn restrict_pipe_to_current_user(pipe_name: &str) -> Result<()> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Security::Authorization::SE_KERNEL_OBJECT;

    // SAFETY: all pointers are derived from owned buffers that remain live
    // for the duration of the SetNamedSecurityInfoW call.
    unsafe {
        let token_buf = current_user_token_buf()?;
        let (_acl_buf, acl) = build_acl(&token_buf, GENERIC_ALL)?;

        let name_wide: Vec<u16> = OsStr::new(pipe_name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let err = SetNamedSecurityInfoW(
            name_wide.as_ptr(),
            SE_KERNEL_OBJECT,
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            acl,
            std::ptr::null_mut(),
        );

        if err != 0 {
            return Err(anyhow!(
                "SetNamedSecurityInfoW on pipe failed (error {})",
                err
            ));
        }
    }

    Ok(())
}
