use std::alloc::{Layout, alloc, dealloc};
use std::mem::forget;
use std::num::NonZero;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};

macro_rules! cold_panic {
    ( $func:ident => $( $args:tt )* ) => {
        #[cold]
        #[track_caller]
        #[inline(never)]
        fn $func() -> ! {
            panic!( $( $args )* );
        }

        $func()
    };
}

macro_rules! safe_abort {
    ( $func:ident => $s:literal ) => {
        #[cold]
        #[track_caller]
        #[inline(never)]
        fn $func() -> ! {
            const BUF: &'static [u8] = concat!("\n", $s, "\n").as_bytes();

            let mut stderr = std::io::stderr().lock();
            let _ = std::io::Write::write_all(&mut stderr, BUF);
            std::process::abort();
        }

        $func()
    };
}

#[repr(C)]
struct Alloc {
    ref_count: AtomicUsize,
    bytes_ptr: NonNull<u8>,
    bytes_cap: usize,
}

#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq)]
struct AllocPtr(NonNull<Alloc>);

pub(crate) struct Inner {
    alloc: AllocPtr,
    pub(crate) bytes: NonNull<u8>,
}

impl Alloc {
    const _ALIGNMENT: () = assert!(align_of::<Alloc>() & !AllocPtr::MASK == 0);

    #[inline(always)]
    fn layout(bytes_cap: usize) -> Layout {
        const MAX_BYTES_CAP: usize =
            isize::MAX as usize + 1 - align_of::<Alloc>() - size_of::<Alloc>();

        if bytes_cap > MAX_BYTES_CAP {
            cold_panic!(layout_size_overflow => "layout size overflow");
        }

        // Round size up to the nearest multiple of align.
        let size =
            (size_of::<Alloc>() + bytes_cap + align_of::<Alloc>() - 1) & !(align_of::<Alloc>() - 1);

        // SAFETY: size does not overflow `isize::MAX as usize`.
        unsafe { Layout::from_size_align_unchecked(size, align_of::<Alloc>()) }
    }

    #[inline(always)]
    fn new(bytes_ptr: NonNull<u8>, bytes_cap: usize) -> Self {
        Self {
            ref_count: AtomicUsize::new(1),
            bytes_ptr,
            bytes_cap,
        }
    }
}

impl AllocPtr {
    const EXTERNAL: usize = 1;

    const MASK: usize = !0b1;

    #[inline(always)]
    fn as_non_null(self) -> NonNull<Alloc> {
        self.0.map_addr(|addr| {
            let addr = addr.get() & Self::MASK;

            // SAFETY: `Alloc::_ALIGNMENT` ensures `addr` alignment.
            unsafe { NonZero::new_unchecked(addr) }
        })
    }

    #[inline(always)]
    fn as_ptr(self) -> *mut Alloc {
        self.as_non_null().as_ptr()
    }

    #[inline(always)]
    fn is_embedded(self) -> bool {
        self.0.addr().get() & Self::EXTERNAL == 0
    }

    #[inline(always)]
    #[expect(unused)]
    fn is_external(self) -> bool {
        self.0.addr().get() & Self::EXTERNAL != 0
    }

    #[inline(always)]
    fn new_embedded(cap: usize) -> (Self, NonNull<u8>) {
        #[cfg(feature = "debug")]
        println!("UNBITE > Inner::new_embedded(cap: {cap})");

        let layout = Alloc::layout(cap);

        // SAFETY: `layout` size is at least `size_of::<Alloc>()`.
        let alloc_ptr = NonNull::new(unsafe { alloc(layout) }).unwrap();

        // SAFETY: `ptr` is derived from the same memory allocation.
        let ptr = unsafe { alloc_ptr.add(size_of::<Alloc>()) };

        let alloc_ptr = alloc_ptr.cast::<Alloc>();

        // SAFETY: `alloc_ptr` is properly aligned and valid for writes.
        unsafe { alloc_ptr.write(Alloc::new(ptr, cap)) }

        (Self(alloc_ptr), ptr)
    }

    #[inline(always)]
    fn new_external(ptr: NonNull<u8>, cap: usize) -> Self {
        #[cfg(feature = "debug")]
        println!("UNBITE > Inner::new_external(ptr: {ptr:?}, cap: {cap})");

        let alloc = Box::new(Alloc::new(ptr, cap));

        let alloc_ptr = NonNull::from_mut(Box::leak(alloc));

        Self(alloc_ptr.map_addr(|addr| addr | Self::EXTERNAL))
    }

    #[inline(always)]
    fn inc_ref_count(self) {
        #[cfg(feature = "debug")]
        println!("UNBITE > Inner::inc_ref_count()");

        unsafe { (*self.as_ptr()).ref_count.fetch_add(1, Ordering::Relaxed) };
    }

    fn dec_ref_count<const DEALLOCATE: bool>(self) {
        #[cfg(feature = "debug")]
        println!("UNBITE > Inner::dec_ref_count::<{DEALLOCATE}>()");

        let alloc_ptr = self.as_ptr();

        let previous = unsafe { (*alloc_ptr).ref_count.fetch_sub(1, Ordering::AcqRel) };

        #[cfg(feature = "debug")]
        println!("UNBITE > previous = {previous}");

        if previous > 1 {
            return;
        }

        if previous == 0 {
            safe_abort!(ref_count_underflow => "ref_count underflow");
        }

        if const { !DEALLOCATE } {
            safe_abort!(ref_count_reached_0 => "ref_count reached 0 during non-deallocating drop");
        }

        #[cfg(feature = "debug")]
        println!("UNBITE > dealloc");

        if self.is_embedded() {
            let layout = Alloc::layout(unsafe { (*alloc_ptr).bytes_cap });

            unsafe { dealloc(alloc_ptr.cast(), layout) };

            return;
        }

        let alloc = unsafe { Box::from_raw(alloc_ptr) };

        unsafe {
            let layout = Layout::from_size_align_unchecked(alloc.bytes_cap, 1);

            dealloc(alloc.bytes_ptr.as_ptr(), layout);
        }
    }
}

impl Inner {
    #[inline(always)]
    pub(crate) fn new_embedded(cap: usize) -> Self {
        let (alloc, bytes) = AllocPtr::new_embedded(cap);

        Self { alloc, bytes }
    }

    #[inline(always)]
    #[expect(unused)]
    pub(crate) fn new_external(ptr: NonNull<u8>, cap: usize) -> Self {
        let alloc = AllocPtr::new_external(ptr, cap);

        Self { alloc, bytes: ptr }
    }

    #[inline(always)]
    pub(crate) fn drop_non_deallocating(self) {
        #[cfg(feature = "debug")]
        println!("UNBITE > Inner::drop_non_deallocating()");

        self.alloc.dec_ref_count::<false>();

        forget(self);
    }

    #[inline(always)]
    pub(crate) unsafe fn split_off_unchecked(&mut self, at: usize) -> Inner {
        #[cfg(feature = "debug")]
        println!("UNBITE > Inner::split_off_unchecked(at: {at})");

        self.alloc.inc_ref_count();

        Inner {
            alloc: self.alloc,
            bytes: unsafe { self.bytes.add(at) },
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn split_to_unchecked(&mut self, at: usize) -> Inner {
        #[cfg(feature = "debug")]
        println!("UNBITE > Inner::split_to_unchecked(at: {at})");

        self.alloc.inc_ref_count();

        let bytes = self.bytes;

        self.bytes = unsafe { self.bytes.add(at) };

        Inner {
            alloc: self.alloc,
            bytes,
        }
    }

    #[inline(always)]
    pub(crate) unsafe fn can_unsplit(&self, cap: usize, r: &Inner) -> bool {
        let end = unsafe { self.bytes.add(cap) };

        end == r.bytes
    }
}

impl Drop for Inner {
    #[inline(always)]
    fn drop(&mut self) {
        #[cfg(feature = "debug")]
        println!("UNBITE > Inner::drop()");
        self.alloc.dec_ref_count::<true>();
    }
}
