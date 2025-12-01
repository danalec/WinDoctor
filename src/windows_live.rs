use std::ptr;
use std::time::Duration as StdDuration;
use chrono::{DateTime, Utc};
use crate::{EventItem, parse_event_xml};
use windows_sys::Win32::System::EventLog::*;
use windows_sys::Win32::Foundation::GetLastError;

struct Handle(EVT_HANDLE);
impl Drop for Handle { fn drop(&mut self) { unsafe { EvtClose(self.0); } } }

fn w(s: &str) -> Vec<u16> { let mut v = s.encode_utf16().collect::<Vec<u16>>(); v.push(0); v }

pub fn query_live_events(channels: &[String], since: DateTime<Utc>) -> Vec<EventItem> {
    let mut out = Vec::new();
    for ch in channels {
        unsafe {
            let ts = since.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
            let q = format!("<QueryList><Query Id=\"0\"><Select Path=\"{}\">*[System[TimeCreated[@SystemTime &gt;= '{}']]]</Select></Query></QueryList>", ch, ts);
            let h = EvtQuery(0, std::ptr::null(), w(&q).as_ptr(), 0);
            if h == 0 {
                let code = GetLastError();
                let h2 = EvtQuery(0, w(ch).as_ptr(), std::ptr::null(), EvtQueryChannelPath);
                if h2 == 0 { log::error!("EvtQuery failed for {}: {}", ch, code); continue; }
                let h = Handle(h2);
                let mut arr: [EVT_HANDLE; 64] = [0; 64];
                loop {
                    let mut returned: u32 = 0;
                    let ok = EvtNext(h.0, arr.len() as u32, arr.as_mut_ptr(), 100, 0, &mut returned);
                    if ok == 0 { let code = GetLastError(); if code != 259 && code != 0 { log::error!("EvtNext error: {}", code); } break; }
                    if returned == 0 { break; }
                    for &ev in arr.iter().take(returned as usize) {
                        if let Some(xml) = render_xml(ev) && let Some(mut item) = parse_event_xml(&xml, ch) {
                            if let Some(msg) = crate::decoder::decode_event(&item.provider, item.event_id, &xml) { item.content = msg; }
                            item.raw_xml = Some(xml.clone());
                            out.push(item);
                        }
                        EvtClose(ev);
                    }
                }
                continue;
            }
            let h = Handle(h);
            let mut arr: [EVT_HANDLE; 64] = [0; 64];
            loop {
                let mut returned: u32 = 0;
                let ok = EvtNext(h.0, arr.len() as u32, arr.as_mut_ptr(), 100, 0, &mut returned);
                if ok == 0 {
                    let code = GetLastError();
                    if code != 259 && code != 0 { log::error!("EvtNext error: {}", code); }
                    break;
                }
                if returned == 0 { break; }
                for &ev in arr.iter().take(returned as usize) {
                    if let Some(xml) = render_xml(ev) && let Some(mut item) = parse_event_xml(&xml, ch) {
                        if let Some(msg) = crate::decoder::decode_event(&item.provider, item.event_id, &xml) { item.content = msg; }
                        item.raw_xml = Some(xml.clone());
                        out.push(item);
                    }
                    EvtClose(ev);
                }
            }
        }
    }
    out
}

unsafe fn render_xml(ev: EVT_HANDLE) -> Option<String> {
    let mut used: u32 = 0;
    let mut count: u32 = 0;
    let ok = unsafe { EvtRender(0, ev, EvtRenderEventXml, 0, ptr::null_mut(), &mut used, &mut count) };
    let need = if ok == 0 { used } else { 0 };
    if need == 0 { return None; }
    let mut buf: Vec<u16> = vec![0u16; (need as usize).div_ceil(2)];
    if unsafe { EvtRender(0, ev, EvtRenderEventXml, need, buf.as_mut_ptr() as *mut _, &mut used, &mut count) } != 0 {
        let s = String::from_utf16_lossy(&buf);
        Some(s.trim_matches(char::from(0)).to_string())
    } else { None }
}

pub fn subscribe_events(channels: &[String], duration_secs: u64) -> Vec<EventItem> {
    use std::sync::mpsc::{channel, Sender};
    let (tx, rx) = channel::<(String, String)>();
    #[repr(C)]
    struct CallbackCtx { tx: Sender<(String, String)>, ch: String }
    let mut subs: Vec<Handle> = vec![];
    let mut ctx_ptrs: Vec<*mut CallbackCtx> = vec![];
    unsafe extern "system" fn callback(action: EVT_SUBSCRIBE_NOTIFY_ACTION, user: *const core::ffi::c_void, event: EVT_HANDLE) -> u32 {
        if action == EvtSubscribeActionDeliver
            && let Some(xml) = unsafe { crate::windows_live::render_xml(event) } {
            let c = unsafe { &*(user as *const CallbackCtx) };
            let _ = c.tx.send((c.ch.clone(), xml));
        }
        0
    }
    unsafe {
        for ch in channels {
            let path_w = w(ch);
            let ctx = Box::into_raw(Box::new(CallbackCtx { tx: tx.clone(), ch: ch.clone() }));
            ctx_ptrs.push(ctx);
            let h = EvtSubscribe(0, std::ptr::null_mut(), path_w.as_ptr(), w("*").as_ptr(), 0, ctx as *const _, Some(callback), EvtSubscribeToFutureEvents);
            if h == 0 { continue; }
            subs.push(Handle(h));
        }
    }
    std::thread::sleep(StdDuration::from_secs(duration_secs));
    let mut out = vec![];
    while let Ok((ch, xml)) = rx.try_recv() {
        if let Some(mut item) = parse_event_xml(&xml, if ch.is_empty() { "" } else { &ch }) {
            if let Some(msg) = crate::decoder::decode_event(&item.provider, item.event_id, &xml) { item.content = msg; }
            item.raw_xml = Some(xml.clone());
            out.push(item);
        }
    }
    for ptr in ctx_ptrs { unsafe { let _ = Box::from_raw(ptr); } }
    out
}
