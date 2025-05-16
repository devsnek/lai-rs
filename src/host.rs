use super::helper::*;
use crate::sys;
use core::{
    alloc::Layout,
    ffi::{c_char, c_int, c_void},
};
use spin::Once;

static LAI_HOST: Once<&'static dyn Host> = Once::new();

fn get_laihost() -> &'static dyn Host {
    *LAI_HOST.get().expect("lai: host not initialized")
}

#[derive(Debug)]
pub enum LogLevel {
    Debug,
    Warn,
}

pub trait Host: Send + Sync {
    fn scan(&self, _signature: &str, _index: usize) -> *mut u8;
    fn sleep(&self, _ms: u64);

    // Port I/O functions:
    fn outb(&self, _port: u16, _value: u8);
    fn outw(&self, _port: u16, _value: u16);
    fn outd(&self, _port: u16, _value: u32);

    fn inb(&self, _port: u16) -> u8;
    fn inw(&self, _port: u16) -> u16;
    fn ind(&self, _port: u16) -> u32;

    // PCI functions:
    fn pci_writeb(&self, _seg: u16, _bus: u8, _slot: u8, _fun: u8, _offset: u16, _value: u8);
    fn pci_writew(&self, _seg: u16, _bus: u8, _slot: u8, _fun: u8, _offset: u16, _value: u16);
    fn pci_writed(&self, _seg: u16, _bus: u8, _slot: u8, _fun: u8, _offset: u16, _value: u32);

    fn pci_readb(&self, _seg: u16, _bus: u8, _slot: u8, _fun: u8, _offset: u16) -> u8;
    fn pci_readw(&self, _seg: u16, _bus: u8, _slot: u8, _fun: u8, _offset: u16) -> u16;
    fn pci_readd(&self, _seg: u16, _bus: u8, _slot: u8, _fun: u8, _offset: u16) -> u32;

    // Maps count bytes from the given physical address and returns
    // a pointer that can be used to access the memory.
    fn map(&self, _address: usize, _count: usize) -> *mut u8;
    fn unmap(&self, _address: usize, _count: usize);

    fn timer(&self) -> u64;

    fn log(&self, level: LogLevel, message: &str) {
        match level {
            LogLevel::Debug => log::debug!("{message}"),
            LogLevel::Warn => log::warn!("{message}"),
        };
    }

    unsafe fn alloc(&self, size: usize) -> *mut u8 {
        let layout = Layout::from_size_align_unchecked(size, 16);
        alloc::alloc::alloc_zeroed(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, size: usize) {
        let layout = Layout::from_size_align_unchecked(size, 16);
        alloc::alloc::dealloc(ptr, layout);
    }

    unsafe fn realloc(&self, ptr: *mut u8, new_size: usize, old_size: usize) -> *mut u8 {
        let layout = Layout::from_size_align_unchecked(old_size, 16);
        alloc::alloc::realloc(ptr, layout, new_size)
    }
}

pub fn init(host: &'static dyn Host) {
    LAI_HOST.call_once(|| host);
}

#[no_mangle]
extern "C" fn laihost_log(level: c_int, message: *const c_char) {
    let message = unsafe { c_str_as_str(message) };
    let level = match level as _ {
        sys::LAI_DEBUG_LOG => LogLevel::Debug,
        sys::LAI_WARN_LOG => LogLevel::Warn,
        _ => unreachable!("undefined log level: (message={message}, level={level})"),
    };
    get_laihost().log(level, message);
}

#[no_mangle]
extern "C" fn laihost_panic(message: *const c_char) -> ! {
    let message = unsafe { c_str_as_str(message) };
    panic!("{message}");
}

#[no_mangle]
extern "C" fn laihost_malloc(size: usize) -> *mut c_void {
    unsafe { get_laihost().alloc(size) as _ }
}

#[no_mangle]
unsafe extern "C" fn laihost_free(ptr: *mut c_void, size: usize) {
    get_laihost().dealloc(ptr as _, size)
}

#[no_mangle]
unsafe extern "C" fn laihost_realloc(
    ptr: *mut c_void,
    new_size: usize,
    old_size: usize,
) -> *mut c_void {
    // lai takes advantage of some realloc behavior that is UB in rust:
    //     - realloc(0, new_size)
    //     - realloc(ptr, 0)
    // these cases are handled explicitly to prevent this UB
    if ptr.is_null() {
        get_laihost().alloc(new_size) as _
    } else if new_size == 0 {
        get_laihost().dealloc(ptr as _, old_size);
        core::ptr::null_mut()
    } else {
        get_laihost().realloc(ptr as _, new_size, old_size) as _
    }
}

#[no_mangle]
unsafe extern "C" fn laihost_scan(signature: *const c_char, index: usize) -> *mut c_void {
    let signature = c_str_as_str(signature);
    get_laihost().scan(signature, index) as _
}

// Port I/O functions:
#[no_mangle]
extern "C" fn laihost_outb(port: u16, value: u8) {
    get_laihost().outb(port, value)
}

#[no_mangle]
extern "C" fn laihost_outw(port: u16, value: u16) {
    get_laihost().outw(port, value)
}

#[no_mangle]
extern "C" fn laihost_outd(port: u16, value: u32) {
    get_laihost().outd(port, value)
}

#[no_mangle]
extern "C" fn laihost_inb(port: u16) -> u8 {
    get_laihost().inb(port)
}

#[no_mangle]
extern "C" fn laihost_inw(port: u16) -> u16 {
    get_laihost().inw(port)
}

#[no_mangle]
extern "C" fn laihost_ind(port: u16) -> u32 {
    get_laihost().ind(port)
}

// Thread functions:
#[no_mangle]
extern "C" fn laihost_sleep(ms: u64) {
    get_laihost().sleep(ms)
}

// PCI read functions:
#[no_mangle]
extern "C" fn laihost_pci_readb(seg: u16, bus: u8, slot: u8, fun: u8, offset: u16) -> u8 {
    get_laihost().pci_readb(seg, bus, slot, fun, offset)
}

#[no_mangle]
extern "C" fn laihost_pci_readw(seg: u16, bus: u8, slot: u8, fun: u8, offset: u16) -> u16 {
    get_laihost().pci_readw(seg, bus, slot, fun, offset)
}

#[no_mangle]
extern "C" fn laihost_pci_readd(seg: u16, bus: u8, slot: u8, fun: u8, offset: u16) -> u32 {
    get_laihost().pci_readd(seg, bus, slot, fun, offset)
}

#[no_mangle]
extern "C" fn laihost_pci_writeb(seg: u16, bus: u8, slot: u8, fun: u8, offset: u16, value: u8) {
    get_laihost().pci_writeb(seg, bus, slot, fun, offset, value)
}

#[no_mangle]
extern "C" fn laihost_pci_writew(seg: u16, bus: u8, slot: u8, fun: u8, offset: u16, value: u16) {
    get_laihost().pci_writew(seg, bus, slot, fun, offset, value)
}

#[no_mangle]
extern "C" fn laihost_pci_writed(seg: u16, bus: u8, slot: u8, fun: u8, offset: u16, value: u32) {
    get_laihost().pci_writed(seg, bus, slot, fun, offset, value)
}

// Memory functions:
#[no_mangle]
extern "C" fn laihost_map(address: *mut c_void, count: usize) -> *mut c_void {
    get_laihost().map(address as _, count) as _
}

#[no_mangle]
extern "C" fn laihost_unmap(address: *mut c_void, count: usize) {
    get_laihost().unmap(address as _, count)
}

#[no_mangle]
extern "C" fn laihost_timer() -> u64 {
    get_laihost().timer()
}
