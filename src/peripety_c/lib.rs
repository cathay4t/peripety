// Copyright (C) 2018 Red Hat, Inc.
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//
// Author: Gris Ge <fge@redhat.com>

extern crate peripety;

use peripety::{
    LogSeverity, PeripetyError, StorageEvent, StorageEventFilterType,
    StorageEventIter,
};
use std::ffi::CStr;
use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::ptr::{null, null_mut};

const PERIPETY_OK: c_int = 0;
const PERIPETY_ERR_LOG_SEVERITY_PARSE_ERROR: c_int = 1;
const PERIPETY_ERR_CONF_ERROR: c_int = 2;
const PERIPETY_ERR_JSON_SERIALIZE_ERROR: c_int = 3;
const PERIPETY_ERR_JSON_DESERIALIZE_ERROR: c_int = 4;
const PERIPETY_ERR_NO_SUPPORT: c_int = 5;
const PERIPETY_ERR_INTERNAL_BUG: c_int = 6;
const PERIPETY_ERR_BLOCK_NO_EXISTS: c_int = 7;
const PERIPETY_ERR_STORAGE_SUBSYSTEM_PARSE_ERROR: c_int = 8;
const PERIPETY_ERR_INVALID_ARGUMENT: c_int = 9;
const PERIPETY_ERR_LOG_ACCESS_ERROR: c_int = 10;

pub struct _PeripetyError {
    msg: *mut c_char,
    code: c_int,
}

pub struct _PeripetyEventIter {
    iter: StorageEventIter,
}

pub struct _PeripetyEvent {
    event: StorageEvent,
}

#[repr(C)]
enum PeripetyEventFilterType {
    PERIPETY_EVENT_FILTER_TYPE_WWID = 0,
    PERIPETY_EVENT_FILTER_TYPE_EVEN_TYPE = 1,
    PERIPETY_EVENT_FILTER_TYPE_SEVERITY = 2,
    // ^ Equal or higher severity will match.
    PERIPETY_EVENT_FILTER_TYPE_SUBSYSTEM = 3,
    PERIPETY_EVENT_FILTER_TYPE_SINCE = 4,
    PERIPETY_EVENT_FILTER_TYPE_EVENTID = 5,
}

#[repr(C)]
enum PeripetySeverity {
    PERIPETY_SEVEITY_EMERGENCY = 0,
    PERIPETY_SEVEITY_ALERT = 1,
    PERIPETY_SEVEITY_CTRITICAL = 2,
    PERIPETY_SEVEITY_ERROR = 3,
    PERIPETY_SEVEITY_WARNING = 4,
    PERIPETY_SEVEITY_NOTICE = 5,
    PERIPETY_SEVEITY_INFO = 6,
    PERIPETY_SEVEITY_DEBUG = 7,
    PERIPETY_SEVEITY_UNKNOWN = 255,
}

fn peripety_error_to_c(error: &PeripetyError) -> _PeripetyError {
    let err_msg_rust = format!("{}", error);
    let msg = match CString::new(err_msg_rust) {
        Ok(c) => c.into_raw(),
        Err(_) => {
            // CString found null byte in the middle.
            // This should never happen.
            CString::new("BUG: error msg contain null in the middle")
                .unwrap()
                .into_raw()
        }
    };
    let code: c_int = match error {
        PeripetyError::LogSeverityParseError(_) => 1,
        PeripetyError::ConfError(_) => 2,
        PeripetyError::JsonSerializeError(_) => 3,
        PeripetyError::JsonDeserializeError(_) => 4,
        PeripetyError::NoSupport(_) => 5,
        PeripetyError::InternalBug(_) => 6,
        PeripetyError::BlockNoExists(_) => 7,
        PeripetyError::StorageSubSystemParseError(_) => 8,
        PeripetyError::InvalidArgument(_) => 9,
        PeripetyError::LogAccessError(_) => 10,
    };
    _PeripetyError { msg, code }
}

#[no_mangle]
pub extern "C" fn peripety_event_iter_new(
    error: *mut *mut _PeripetyError,
) -> *mut _PeripetyEventIter {
    if error.is_null() {
        return null_mut();
    }
    unsafe {
        *error = null_mut();
    }
    match StorageEventIter::new() {
        Ok(iter) => {
            let i = _PeripetyEventIter { iter };
            return Box::into_raw(Box::new(i));
        }
        Err(e) => unsafe {
            let e = peripety_error_to_c(&e);
            *error = Box::into_raw(Box::new(e));
            return null_mut();
        },
    };
}

#[no_mangle]
pub extern "C" fn peripety_event_iter_free(iter: *mut _PeripetyEventIter) {
    if iter.is_null() {
        return;
    }
    let iter = unsafe { &mut *iter };
    unsafe {
        Box::from_raw(iter);
    }
}

#[no_mangle]
pub extern "C" fn peripety_error_free(error: *mut _PeripetyError) {
    if error.is_null() {
        return;
    }
    let error = unsafe { &mut *error };
    unsafe {
        CString::from_raw(error.msg);
        Box::from_raw(error);
    }
}

#[no_mangle]
pub extern "C" fn peripety_error_msg_get(
    error: *mut _PeripetyError,
) -> *const c_char {
    if error.is_null() {
        return null();
    }
    let e = unsafe { &mut *error };
    e.msg
}

#[no_mangle]
pub extern "C" fn peripety_error_code_get(error: *mut _PeripetyError) -> c_int {
    if error.is_null() {
        return PERIPETY_INVALID_ARGUMENT;
    }
    let e = unsafe { &mut *error };
    e.code
}

#[no_mangle]
pub extern "C" fn peripety_event_get_next(
    iter: *mut _PeripetyEventIter,
    pe: *mut *mut _PeripetyError,
    error: *mut *mut _PeripetyError,
) -> c_int {
    if pe.is_null() || error.is_null() {
        return PERIPETY_ERR_INVALID_ARGUMENT;
    }
    unsafe {
        *pe = null_mut();
        *error = null_mut();
    }
    let iter = unsafe { &mut *iter } ;
    match iter.next() {
        Some(Ok(se)) => {
            return PERIPETY_OK;
        },
        Some(Err(e)) => {
            let e = peripety_error_to_c(&e);
            unsafe {
                *error = Box::into_raw(Box::new(e));
            }
            return e.code;
        },
        None => {
            return PERIPETY_OK;
        }
    }
}
