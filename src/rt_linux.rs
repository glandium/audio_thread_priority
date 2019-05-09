/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this file,
 * You can obtain one at http://mozilla.org/MPL/2.0/. */

/* Widely copied from dbus-rs/dbus/examples/rtkit.rs */

extern crate dbus;
extern crate libc;

use std::cmp;

use dbus::{Connection, BusType, Props, MessageItem, Message};

const RT_PRIO_DEFAULT: u32 = 10;

/*#[derive(Debug)]*/
pub struct RtPriorityHandleInternal {
    policy: libc::c_int,
    param: libc::sched_param,
}

impl RtPriorityHandleInternal {
    pub fn new() -> RtPriorityHandleInternal {
        return RtPriorityHandleInternal {
            policy: 0 as libc::c_int,
            param: libc::sched_param { sched_priority: 0 },
        }
    }
}

fn item_as_i64(i: MessageItem) -> Result<i64, Box<std::error::Error>> {
    match i {
        MessageItem::Int32(i) => Ok(i as i64),
        MessageItem::Int64(i) => Ok(i),
        _ => Err(Box::from(&*format!("Property is not integer ({:?})", i)))
    }
}

fn rtkit_set_realtime(c: &Connection, thread: u64, prio: u32) -> Result<(), ::dbus::Error> {
    let mut m = Message::new_method_call("org.freedesktop.RealtimeKit1",
                                         "/org/freedesktop/RealtimeKit1",
                                         "org.freedesktop.RealtimeKit1",
                                         "MakeThreadRealtime").unwrap();
    m.append_items(&[thread.into(), prio.into()]);
    let mut r = try!(c.send_with_reply_and_block(m, 10000));
    r.as_result().map(|_| ())
}

fn make_realtime(max_slice_us: u64, prio: u32) -> Result<u32, Box<std::error::Error>> {
    let c = try!(Connection::get_private(BusType::System));

    let p = Props::new(&c, "org.freedesktop.RealtimeKit1", "/org/freedesktop/RealtimeKit1",
        "org.freedesktop.RealtimeKit1", 10000);

    // Make sure we don't fail by wanting too much
    let max_prio = try!(item_as_i64(try!(p.get("MaxRealtimePriority")))) as u32;
    let prio = cmp::min(prio, max_prio);

    // Enforce RLIMIT_RTPRIO, also a must before asking rtkit for rtprio
    let max_rttime = try!(item_as_i64(try!(p.get("RTTimeUSecMax")))) as u64;

    // Only take what we need, or cap at the system limit, no further.
    let rttime_request = cmp::min(max_slice_us, max_rttime) as u64;

    let new_limit = libc::rlimit64 { rlim_cur: rttime_request,
                                     rlim_max: rttime_request };
    let mut old_limit = new_limit;
    if unsafe { libc::getrlimit64(libc::RLIMIT_RTTIME, &mut old_limit) } < 0 {
        return Err(Box::from("getrlimit failed"));
    }
    if unsafe { libc::setrlimit64(libc::RLIMIT_RTTIME, &new_limit) } < 0 {
        return Err(Box::from("setrlimit failed"));
    }

    // Finally, let's ask rtkit to make us realtime
    let thread_id = unsafe { libc::syscall(libc::SYS_gettid) };
    let r = rtkit_set_realtime(&c, thread_id as u64, prio);

    if r.is_err() {
        unsafe { libc::setrlimit64(libc::RLIMIT_RTTIME, &old_limit) };
    }

    try!(r);
    Ok(prio)
}

pub fn promote_current_thread_to_real_time_internal(audio_buffer_frames: u32,
                                           audio_samplerate_hz: u32)
                                           -> Result<RtPriorityHandleInternal, ()> {
    let mut policy = 0;
    let mut param = libc::sched_param { sched_priority: 0 };
    let mut buffer_frames = audio_buffer_frames;
    if buffer_frames == 0 {
        // 50ms slice. This "ought to be enough for anybody".
        buffer_frames = audio_samplerate_hz / 20;
    }
    let budget_us = (buffer_frames * 1_000_000 / audio_samplerate_hz) as u64;
    if unsafe { libc::pthread_getschedparam(libc::pthread_self(), &mut policy, &mut param) } < 0 {
        error!("pthread_getschedparam error {}", unsafe { libc::pthread_self() });
        return Err(());
    }
    let handle = RtPriorityHandleInternal {policy: policy, param: param};
    let r = make_realtime(budget_us, RT_PRIO_DEFAULT);
    if r.is_err() {
        error!("Could not make thread real-time.");
        return Err(());
    }
    return Ok(handle);
}

pub fn demote_current_thread_from_real_time_internal(rt_priority_handle: RtPriorityHandleInternal)
                                            -> Result<(), ()> {
    if unsafe { libc::pthread_setschedparam(libc::pthread_self(), rt_priority_handle.policy, &rt_priority_handle.param) } < 0 {
        error!("could not demote thread {}", unsafe { libc::pthread_self() } );
        return Err(());
    }
    return Ok(());
}
