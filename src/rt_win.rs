/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this file,
 * You can obtain one at http://mozilla.org/MPL/2.0/. */

use winapi::um::avrt::*;
use winapi::shared::ntdef::HANDLE;
use winapi::shared::minwindef::DWORD;
use kernel32::GetLastError;

#[derive(Debug)]
pub struct RtPriorityHandleInternal {
  mmcss_task_index: DWORD,
  task_handle: HANDLE
}

impl RtPriorityHandleInternal {
    pub fn new() -> RtPriorityHandleInternalInternal {
        return RtPriorityHandleInternal {
           mmcss_task_index: 0 as DWORD,
           task_handle: 0 as HANDLE
        }
    }
}

pub fn demote_current_thread_from_real_time_internal(rt_priority_handle: RtPriorityHandleInternal)
                                            -> Result<(), ()> {
    unsafe {
        let rv = AvRevertMmThreadCharacteristics(rt_priority_handle.task_handle);
        if rv == 0 {
            error!("Unable to restor the thread priority ({})", GetLastError());
            return Err(())
        }
    }

    info!("task {} priority restored.", handle.mmcss_task_index);

    return Ok(())
}

pub fn promote_current_thread_to_real_time_internal(_audio_buffer_frames: u32,
                                           audio_samplerate_hz: u32)
                                           -> Result<RtPriorityHandleInternal, ()> {
    let mut handle = RtPriorityHandleInternal::new();

    unsafe {
        handle.task_handle = AvSetMmThreadCharacteristicsA("Audio".as_ptr() as _, &mut handle.mmcss_task_index);

        if handle.task_handle.is_null() {
            error!("Unable to use mmcss to bump the render thread priority ({})", GetLastError());
            return Err(())
        }
    }

    info!("task {} bumped to real time priority.", handle.mmcss_task_index);

    Ok(handle)
}
