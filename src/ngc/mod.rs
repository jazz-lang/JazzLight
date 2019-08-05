pub mod bump;
pub mod copying;

pub const K: usize = 1024;
pub const M: usize = K * K;
use std::cmp::{Ord, Ordering, PartialOrd};
use std::fmt;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Address(usize);

impl Address {
    #[inline(always)]
    pub fn from(val: usize) -> Address {
        Address(val)
    }

    #[inline(always)]
    pub fn region_start(self, size: usize) -> Region {
        Region::new(self, self.offset(size))
    }

    #[inline(always)]
    pub fn offset_from(self, base: Address) -> usize {
        debug_assert!(self >= base);

        self.to_usize() - base.to_usize()
    }

    #[inline(always)]
    pub fn offset(self, offset: usize) -> Address {
        Address(self.0 + offset)
    }

    #[inline(always)]
    pub fn sub(self, offset: usize) -> Address {
        Address(self.0 - offset)
    }

    #[inline(always)]
    pub fn add_ptr(self, words: usize) -> Address {
        Address(self.0 + words * std::mem::size_of::<usize>())
    }

    #[inline(always)]
    pub fn sub_ptr(self, words: usize) -> Address {
        Address(self.0 - words * std::mem::size_of::<usize>())
    }

    #[inline(always)]
    pub fn to_usize(self) -> usize {
        self.0
    }

    #[inline(always)]
    pub fn from_ptr<T>(ptr: *const T) -> Address {
        Address(ptr as usize)
    }

    #[inline(always)]
    pub fn to_ptr<T>(&self) -> *const T {
        self.0 as *const T
    }

    #[inline(always)]
    pub fn to_mut_ptr<T>(&self) -> *mut T {
        self.0 as *const T as *mut T
    }

    #[inline(always)]
    pub fn null() -> Address {
        Address(0)
    }

    #[inline(always)]
    pub fn is_null(self) -> bool {
        self.0 == 0
    }

    #[inline(always)]
    pub fn is_non_null(self) -> bool {
        self.0 != 0
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x{:x}", self.to_usize())
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x{:x}", self.to_usize())
    }
}

impl PartialOrd for Address {
    fn partial_cmp(&self, other: &Address) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Address {
    fn cmp(&self, other: &Address) -> Ordering {
        self.to_usize().cmp(&other.to_usize())
    }
}

impl From<usize> for Address {
    fn from(val: usize) -> Address {
        Address(val)
    }
}

#[derive(Copy, Clone)]
pub struct Region {
    pub start: Address,
    pub end: Address,
}

impl Region {
    pub fn new(start: Address, end: Address) -> Region {
        debug_assert!(start <= end);

        Region {
            start: start,
            end: end,
        }
    }

    #[inline(always)]
    pub fn contains(&self, addr: Address) -> bool {
        self.start <= addr && addr < self.end
    }

    #[inline(always)]
    pub fn valid_top(&self, addr: Address) -> bool {
        self.start <= addr && addr <= self.end
    }

    #[inline(always)]
    pub fn size(&self) -> usize {
        self.end.to_usize() - self.start.to_usize()
    }

    #[inline(always)]
    pub fn empty(&self) -> bool {
        self.start == self.end
    }

    #[inline(always)]
    pub fn disjunct(&self, other: &Region) -> bool {
        self.end <= other.start || self.start >= other.end
    }

    #[inline(always)]
    pub fn overlaps(&self, other: &Region) -> bool {
        !self.disjunct(other)
    }

    #[inline(always)]
    pub fn fully_contains(&self, other: &Region) -> bool {
        self.contains(other.start) && self.valid_top(other.end)
    }
}

impl Default for Region {
    fn default() -> Region {
        Region {
            start: Address::null(),
            end: Address::null(),
        }
    }
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}-{}", self.start, self.end)
    }
}

struct FormattedSize {
    size: usize,
}

impl fmt::Display for FormattedSize {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ksize = (self.size as f64) / 1024f64;

        if ksize < 1f64 {
            return write!(f, "{}B", self.size);
        }

        let msize = ksize / 1024f64;

        if msize < 1f64 {
            return write!(f, "{:.1}K", ksize);
        }

        let gsize = msize / 1024f64;

        if gsize < 1f64 {
            write!(f, "{:.1}M", msize)
        } else {
            write!(f, "{:.1}G", gsize)
        }
    }
}

fn formatted_size(size: usize) -> FormattedSize {
    FormattedSize { size }
}

pub(crate) use self::ProtType::*;

#[cfg(not(target_family = "windows"))]
use libc;

use std::ptr;

static mut PAGE_SIZE: u32 = 0;
static mut PAGE_SIZE_BITS: u32 = 0;

pub fn init_page_size() {
    unsafe {
        PAGE_SIZE = determine_page_size();
        assert!((PAGE_SIZE & (PAGE_SIZE - 1)) == 0);

        PAGE_SIZE_BITS = log2(PAGE_SIZE);
    }
}

#[cfg(target_family = "unix")]
fn determine_page_size() -> u32 {
    let val = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };

    if val <= 0 {
        panic!("could not determine page size.");
    }

    val as u32
}

#[cfg(target_family = "windows")]
pub fn determine_page_size() -> u32 {
    use std::mem;
    use winapi::um::sysinfoapi::{GetSystemInfo, SYSTEM_INFO};

    unsafe {
        let mut system_info: SYSTEM_INFO = mem::uninitialized();
        GetSystemInfo(&mut system_info);

        system_info.dwPageSize
    }
}

/// determine log_2 of given value
fn log2(mut val: u32) -> u32 {
    let mut log = 0;

    if (val & 0xFFFF0000) != 0 {
        val >>= 16;
        log += 16;
    }
    if val >= 256 {
        val >>= 8;
        log += 8;
    }
    if val >= 16 {
        val >>= 4;
        log += 4;
    }
    if val >= 4 {
        val >>= 2;
        log += 2;
    }

    log + (val >> 1)
}

#[test]
fn test_log2() {
    for i in 0..32 {
        assert_eq!(i, log2(1 << i));
    }
}

pub fn page_size() -> u32 {
    unsafe { PAGE_SIZE }
}

pub fn page_size_bits() -> u32 {
    unsafe { PAGE_SIZE_BITS }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ProtType {
    None,
    Executable,
    Writable,
}

impl ProtType {
    #[cfg(target_family = "unix")]
    fn to_libc(self) -> libc::c_int {
        match self {
            ProtType::None => 0,
            ProtType::Writable => libc::PROT_READ | libc::PROT_WRITE,
            ProtType::Executable => libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC,
        }
    }
}

#[cfg(target_family = "unix")]
pub fn mmap(size: usize, prot: ProtType) -> *const u8 {
    let ptr = unsafe {
        libc::mmap(
            ptr::null_mut(),
            size,
            prot.to_libc(),
            libc::MAP_PRIVATE | libc::MAP_ANON,
            -1,
            0,
        ) as *mut libc::c_void
    };

    if ptr == libc::MAP_FAILED {
        panic!("mmap failed");
    }

    ptr as *const u8
}

extern "C" {
    fn malloc(_: usize) -> *mut u8;
}

#[cfg(target_family = "windows")]
pub fn mmap(size: usize, exec: ProtType) -> *const u8 {
    use winapi::um::memoryapi::VirtualAlloc;
    use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE, PAGE_READWRITE};

    let prot = if exec == Executable {
        PAGE_EXECUTE_READWRITE
    } else {
        PAGE_READWRITE
    };

    let ptr = unsafe { VirtualAlloc(ptr::null_mut(), size, MEM_COMMIT | MEM_RESERVE, prot) };

    if ptr.is_null() {
        use winapi::um::errhandlingapi::GetLastError;
        panic!(
            "VirtualAlloc failed with error code '{:x}',size '{}'",
            unsafe { GetLastError() },
            size
        );
    }

    ptr as *const u8
}

#[cfg(target_family = "unix")]
pub fn munmap(ptr: *const u8, size: usize) {
    let res = unsafe { libc::munmap(ptr as *mut libc::c_void, size) };

    if res != 0 {
        panic!("munmap failed");
    }
}

#[cfg(target_family = "windows")]
pub fn munmap(ptr: *const u8, _size: usize) {
    use winapi::um::memoryapi::VirtualFree;
    use winapi::um::winnt::MEM_RELEASE;

    let res = unsafe { VirtualFree(ptr as *mut _, 0, MEM_RELEASE) };

    if res == 0 {
        panic!("VirtualFree failed");
    }
}

#[cfg(target_family = "unix")]
pub fn mprotect(ptr: *const u8, size: usize, prot: ProtType) {
    debug_assert!(mem::is_page_aligned(ptr as usize));
    debug_assert!(mem::is_page_aligned(size));

    let res = unsafe { libc::mprotect(ptr as *mut libc::c_void, size, prot.to_libc()) };

    if res != 0 {
        panic!("mprotect() failed");
    }
}

pub use copying::*;

unsafe impl Send for CopyGC {}
unsafe impl Sync for CopyGC {}

use parking_lot::Mutex;

lazy_static::lazy_static!(
    pub static ref GC: Mutex<CopyGC> = Mutex::new(CopyGC::new(Option::None));
);

pub fn gc_collect() {
    std::thread::spawn(|| {
        GC.lock().collect();
    });
}

pub fn gc_collect_not_par() {
    GC.lock().collect();
}

pub fn gc_allocate_sync<T: Collectable + Sized + 'static + Send>(val: T) -> GCValue<T> {
    std::thread::spawn(move || GC.lock().allocate(val))
        .join()
        .unwrap()
}

pub fn gc_allocate<T: Collectable + Sized + 'static>(val: T) -> GCValue<T> {
    //GC.with(|x| {
    GC.lock().allocate(val)
}

pub fn gc_rmroot(val: GCValue<dyn Collectable>) {
    GC.lock().remove_root(val);
}

pub fn gc_enable_stats() {
    //GC.with(|x| {
    let mut lock = GC.lock();
    lock.stats = !lock.stats;
    drop(lock);
    //})
}

pub fn gc_allocated_size() -> usize {
    //GC.with(|x| {
    GC.lock().total_allocated()
    //})
}

pub fn gc_add_root(obj: GCValue<dyn Collectable>) {
    GC.lock().add_root(obj);
}
