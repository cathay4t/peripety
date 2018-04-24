// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate libc;

use libc::{c_int, c_void, size_t, ENOENT};

use std::ffi::CString;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::slice;
use std::u64;
use std::fmt;
use std::ptr;
use std::collections::HashMap;

use std::os::unix::io::{AsRawFd, RawFd};

// Opaque data type for journal handle for use in ffi calls
pub enum SdJournal {}

enum SdJournalOpen {
    LocalOnly = 1 << 0,

    // The following are not being utilized at the moment, just here for
    // documentation
    #[allow(dead_code)]
    RuntimeOnly = 1 << 1,

    #[allow(dead_code)]
    System = 1 << 2,

    #[allow(dead_code)]
    CurrentUser = 1 << 3,

    #[allow(dead_code)]
    OsRoot = 1 << 4,
}

#[derive(Clone, Copy)]
pub enum JournalPriority {
    Emergency = 0,
    Alert = 1,
    Critical = 2,
    Error = 3,
    Warning = 4,
    Notice = 5,
    Info = 6,
    Debug = 7,
}

#[derive(Debug)]
pub struct ClibraryError {
    pub message: String,
    pub return_code: c_int,
    pub err_reason: String,
}

impl fmt::Display for ClibraryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "{} (rc={}, errno msg={})",
            self.message, self.return_code, self.err_reason
        )
    }
}

impl std::error::Error for ClibraryError {
    fn description(&self) -> &str {
        &self.message
    }
}

impl ClibraryError {
    pub fn new(error_msg: String, return_code: i32) -> ClibraryError {
        ClibraryError {
            message: error_msg,
            return_code: return_code,
            err_reason: error_string(-return_code),
        }
    }
}

#[derive(Debug)]
pub enum SdJournalError {
    CError(ClibraryError),
    NulError(std::ffi::NulError),
    Utf8(std::string::FromUtf8Error),
}

impl From<std::ffi::NulError> for SdJournalError {
    fn from(err: std::ffi::NulError) -> SdJournalError {
        SdJournalError::NulError(err)
    }
}

impl From<std::string::FromUtf8Error> for SdJournalError {
    fn from(err: std::string::FromUtf8Error) -> SdJournalError {
        SdJournalError::Utf8(err)
    }
}

// Wakeup event types
enum SdJournalWait {
    Nop = 0,
    Append = 1,
    Invalidate = 2,
}

#[link(name = "systemd")]
extern "C" {
    fn sd_journal_open(ret: *mut *mut SdJournal, flags: c_int) -> c_int;
    fn sd_journal_next(j: *mut SdJournal) -> c_int;
    fn sd_journal_get_data(
        j: *mut SdJournal,
        field: *const c_char,
        data: *mut *mut c_void,
        length: *mut size_t,
    ) -> c_int;
    fn sd_journal_close(j: *mut SdJournal);
    fn sd_journal_wait(j: *mut SdJournal, timeout_usec: u64) -> c_int;
    fn sd_journal_seek_tail(j: *mut SdJournal) -> c_int;
    fn sd_journal_previous_skip(j: *mut SdJournal, skip: u64) -> c_int;

    fn sd_journal_send(message: *const u8, ...) -> c_int;

    fn sd_journal_restart_data(j: *mut SdJournal);
    fn sd_journal_enumerate_data(
        j: *mut SdJournal,
        data: *mut *mut c_void,
        length: *mut size_t,
    ) -> c_int;
    fn sd_journal_get_realtime_usec(j: *mut SdJournal, usec: *mut u64)
        -> c_int;

    fn sd_journal_get_fd(j: *mut SdJournal) -> c_int;

    fn sd_journal_get_events(j: *mut SdJournal) -> c_int;
}

// Copied and pasted from https://github.com/rust-lang/rust/blob/master/src/libstd/sys/unix/os.rs
// if I can figure out how to call it I will delete this!!!
pub fn error_string(errno: i32) -> String {
    extern "C" {
        #[cfg_attr(any(target_os = "linux", target_env = "newlib"),
                   link_name = "__xpg_strerror_r")]
        fn strerror_r(
            errnum: c_int,
            buf: *mut c_char,
            buflen: libc::size_t,
        ) -> c_int;
    }

    let mut buf = [0 as c_char; 128];

    let p = buf.as_mut_ptr();
    unsafe {
        if strerror_r(errno as c_int, p, buf.len()) < 0 {
            panic!("strerror_r failure");
        }

        let p = p as *const _;
        std::str::from_utf8(CStr::from_ptr(p).to_bytes())
            .unwrap()
            .to_owned()
    }
}

pub struct Journal {
    handle: *mut SdJournal,
    pub timeout_us: u64,
}

// Close the handle when we go out of scope and get cleaned up
impl Drop for Journal {
    fn drop(&mut self) {
        unsafe { sd_journal_close(self.handle) };
    }
}

impl Journal {
    pub fn new() -> Result<Journal, SdJournalError> {
        let mut tmp_handle = 0 as *mut SdJournal;

        let rc = unsafe {
            sd_journal_open(
                (&mut tmp_handle) as *mut _ as *mut *mut SdJournal,
                SdJournalOpen::LocalOnly as c_int,
            )
        };
        if rc != 0 {
            Err(SdJournalError::CError(ClibraryError::new(
                String::from("Error on sd_journal_open"),
                rc,
            )))
        } else {
            Ok(Journal {
                handle: tmp_handle,
                timeout_us: std::u64::MAX,
            })
        }
    }

    #[allow(dead_code)]
    fn get_log_entry(
        &mut self,
        key: &'static str,
    ) -> Result<String, SdJournalError> {
        let mut x = 0 as *mut c_void;
        let mut len = 0 as size_t;
        let field = CString::new(key)?;

        let log_msg: String;
        let rc = unsafe {
            sd_journal_get_data(
                self.handle,
                field.as_ptr(),
                (&mut x) as *mut _ as *mut *mut c_void,
                &mut len,
            )
        };
        if rc == 0 {
            let slice = unsafe { slice::from_raw_parts(x as *const u8, len) };
            log_msg = String::from_utf8(slice[key.len()..len].to_vec())?;
        } else {
            if rc == -ENOENT {
                // TODO: Is there a better way to handle a key not being found?
                log_msg = String::from("");
            } else {
                return Err(SdJournalError::CError(ClibraryError::new(
                    String::from("Error on sd_journal_get_data"),
                    rc,
                )));
            }
        }

        Ok(log_msg)
    }

    fn get_log_entry_map(
        &mut self,
    ) -> Result<HashMap<String, String>, SdJournalError> {
        let mut result = HashMap::new();

        // Re-set for the enumerator
        unsafe { sd_journal_restart_data(self.handle) };

        loop {
            let mut x = 0 as *mut c_void;
            let mut len = 0 as size_t;

            let rc = unsafe {
                sd_journal_enumerate_data(
                    self.handle,
                    (&mut x) as *mut _ as *mut *mut c_void,
                    &mut len,
                )
            };

            if rc > 0 {
                let slice =
                    unsafe { slice::from_raw_parts(x as *const u8, len) };
                let log_msg = String::from_utf8(slice[0..len].to_vec())?;

                if let Some(m) = log_msg.find('=') {
                    let key = String::from_utf8(slice[0..m].to_vec())?;
                    let value =
                        String::from_utf8(slice[((m + 1)..len)].to_vec())?;
                    result.insert(key, value);
                }
            } else if rc == 0 {
                // No more "key:value" pairs to process, we are done looping
                break;
            } else {
                // negative integer expressing error reason
                return Err(SdJournalError::CError(ClibraryError::new(
                    String::from("Error on sd_journal_enumerate_data"),
                    rc,
                )));
            }
        }
        let mut usec: u64 = 0;
        let rc =
            unsafe { sd_journal_get_realtime_usec(self.handle, &mut usec) };
        if rc == 0 {
            result.insert("__REALTIME_TIMESTAMP".to_string(), usec.to_string());
        } else {
            return Err(SdJournalError::CError(ClibraryError::new(
                String::from("Error on sd_journal_get_realtime_usec"),
                rc,
            )));
        }

        Ok(result)
    }

    pub fn get_events_bit_mask(&mut self) -> i16 {
        let x = unsafe { sd_journal_get_events(self.handle) };
        return x as i16;
    }
    pub fn seek_tail(&mut self) -> Result<bool, SdJournalError> {
        let rc = unsafe { sd_journal_seek_tail(self.handle) };
        if rc < 0 {
            return Err(SdJournalError::CError(ClibraryError::new(
                String::from("Error on sd_journal_seek_tail"),
                rc,
            )));
        }
        let rc = unsafe { sd_journal_previous_skip(self.handle, 1) };
        // Workaround as sd_journal_seek_tail() dose not get you to the
        // very end.
        if rc < 0 {
            return Err(SdJournalError::CError(ClibraryError::new(
                String::from("Error on sd_journal_previous_skip"),
                rc,
            )));
        }
        Ok(true)
    }

    fn get_next_entry(
        &mut self,
    ) -> Option<Result<HashMap<String, String>, SdJournalError>> {
        loop {
            let log_entry = unsafe { sd_journal_next(self.handle) };
            if log_entry < 0 {
                return Some(Err(SdJournalError::CError(ClibraryError::new(
                    String::from("Error on sd_journal_next"),
                    log_entry,
                ))));
            }

            if log_entry == 0 {
                // TODO: Figure out how to make a match work when comparing int to enum type.
                let wait_rc =
                    unsafe { sd_journal_wait(self.handle, self.timeout_us) };

                if wait_rc == SdJournalWait::Nop as i32 {
                    return None;
                } else if wait_rc == SdJournalWait::Append as i32
                    || wait_rc == SdJournalWait::Invalidate as i32
                {
                    continue;
                } else {
                    return Some(Err(SdJournalError::CError(
                        ClibraryError::new(
                            String::from("Error on sd_journal_wait"),
                            wait_rc,
                        ),
                    )));
                }
            }

            let result = self.get_log_entry_map();
            match result {
                Ok(result) => return Some(Ok(result)),
                Err(log_retrieve) => return Some(Err(log_retrieve)),
            }
        }
    }

    pub fn get_next(
        &mut self,
    ) -> Option<Result<HashMap<String, String>, SdJournalError>> {
        self.get_next_entry()
    }
}

pub fn send_journal_basic(
    message_id: &'static str,
    message: &str,
    source: &str,
    source_man: &str,
    device: &str,
    device_id: &str,
    state: &str,
    priority: JournalPriority,
    details: String,
) -> Result<bool, SdJournalError> {
    let msg_id = CString::new(format!("MESSAGE_ID={}", message_id))?;
    let device_cstr = CString::new(format!("DEVICE={}", device))?;
    let device_id_cstr = CString::new(format!("DEVICE_ID={}", device_id))?;
    let state_cstr = CString::new(format!("STATE={}", state))?;
    let source_cstr = CString::new(format!("SOURCE={}", source))?;
    let source_man_cstr = CString::new(format!("SOURCE_MAN={}", source_man))?;
    let details_cstr = CString::new(format!("DETAILS={}", details))?;
    let priority_cstr = CString::new(format!("PRIORITY={}", priority as u8))?;

    let priority_desc = match priority {
        JournalPriority::Emergency => "emergency",
        JournalPriority::Alert => "alert",
        JournalPriority::Critical => "critical",
        JournalPriority::Error => "error",
        JournalPriority::Warning => "warning",
        JournalPriority::Notice => "notice",
        JournalPriority::Info => "info",
        JournalPriority::Debug => "debug",
    };

    let priority_desc_cstr =
        CString::new(format!("PRIORITY_DESC={}", priority_desc))?;
    let message_cstr = CString::new(format!("MESSAGE={}", message))?;
    let end_args: *const u8 = ptr::null();

    let rc = unsafe {
        sd_journal_send(
            msg_id.as_ptr() as *const u8, // MESSAGE_ID
            device_cstr.as_ptr(),         // DEVICE
            device_id_cstr.as_ptr(),      // DEVICE_ID
            state_cstr.as_ptr(),          // STATE
            source_cstr.as_ptr(),         // SOURCE
            source_man_cstr.as_ptr(),     // SOURCE_MAN
            details_cstr.as_ptr(),        // DETAILS
            priority_cstr.as_ptr(),       // PRIORITY
            priority_desc_cstr.as_ptr(),  // PRIORITY_DESC
            message_cstr.as_ptr(),        // MESSAGE
            end_args,                     // End the arguments
        )
    };

    if rc < 0 {
        return Err(SdJournalError::CError(ClibraryError::new(
            String::from("Error on sd_journal_send"),
            rc,
        )));
    }
    Ok(true)
}

impl Iterator for Journal {
    type Item = Result<HashMap<String, String>, SdJournalError>;

    fn next(
        &mut self,
    ) -> Option<Result<HashMap<String, String>, SdJournalError>> {
        self.get_next_entry()
    }
}

impl AsRawFd for Journal {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { sd_journal_get_fd(self.handle) }
    }
}
