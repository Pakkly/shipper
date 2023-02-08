use std::{ops::DerefMut, sync::Mutex};

use chrono::prelude::*;
use log::{Level, Metadata, Record};
pub struct SimpleLogger {
    ring_buffer: Mutex<Ringbuffer>,
}
struct Ringbuffer {
    buffer: Vec<u8>,
    ptr: usize,
    closed: bool,
}
impl SimpleLogger {
    pub fn new(bufsize: usize) -> Self {
        if bufsize < 128 {
            panic!("Bufsize must be at least 128!");
        }
        Self { ring_buffer: Mutex::new(Ringbuffer { buffer: vec![0; bufsize], ptr: 0, closed: false }) }
    }
    pub fn ring_display_and_close(&self) -> Vec<u8> {
        let mut rb_lock = self.ring_buffer.lock().unwrap();
        let rb = rb_lock.deref_mut();
        let mut final_buf: Vec<u8> = vec![0; rb.buffer.len()];
        let buflen = final_buf.len();
        final_buf[0..(buflen - rb.ptr)].copy_from_slice(&rb.buffer[rb.ptr..]);
        final_buf[(buflen - rb.ptr)..].copy_from_slice(&rb.buffer[0..rb.ptr]);

        rb.closed = true;
        //trim 0s
        for i in 0..buflen {
            if final_buf[i] != 0x00 {
                return final_buf[i..].to_vec();
            }
        }
        return [].to_vec();
    }
    fn ring_write(&self, message: &str) {
        let msg_newline = format!("{}\n", message);
        let log_message_bytes = msg_newline.as_bytes();
        let mut rb_lock = self.ring_buffer.lock().unwrap();
        let mut rb = rb_lock.deref_mut();
        if rb.closed {
            return;
        }

        if log_message_bytes.len() > rb.buffer.len() {
            let warning = format!("Warning: message larger than buffer! Len:{}", log_message_bytes.len());
            drop(rb);
            drop(rb_lock);
            self.ring_write(&warning);
            return;
        }
        if rb.ptr + log_message_bytes.len() > rb.buffer.len() {
            //message does not fit, split into two and write remainder to start of buffer
            let first_half = &log_message_bytes[0..(rb.buffer.len() - rb.ptr)];
            let second_half = &log_message_bytes[first_half.len()..];
            rb.buffer[rb.ptr..].copy_from_slice(first_half);
            rb.buffer[0..second_half.len()].copy_from_slice(second_half);
            rb.ptr = second_half.len();
            return;
        }
        rb.buffer[rb.ptr..rb.ptr + log_message_bytes.len()].copy_from_slice(log_message_bytes);
        rb.ptr += log_message_bytes.len();
    }
}
impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let utc: DateTime<Utc> = Utc::now();
            let log_message = format!("[{:?}] - {} - {}", utc, record.level(), record.args());
            #[cfg(debug_assertions)]
            {
                println!("{}", log_message);
            }
            self.ring_write(&log_message);
        }
    }

    fn flush(&self) {}
}
