macro_rules! common_impl {
    (
        self: $self:ident ;
        inner: $inner:expr ;
        $( len: $len:expr ; )?
        $( const_capacity: $const_capacity:expr ; )?
        $( capacity: $capacity:expr ; )?
    ) => {
        #[inline]
        pub const fn as_non_null(&$self) -> std::ptr::NonNull<u8> {
            $inner.bytes
        }

        #[inline]
        pub const fn as_mut_ptr(&$self) -> *mut u8 {
            $self.as_non_null().as_ptr()
        }

        #[inline]
        pub const fn as_ptr(&$self) -> *const u8 {
            $self.as_mut_ptr().cast_const()
        }

        $(
            #[inline]
            pub const fn len(&$self) -> usize {
                $len
            }

            #[inline]
            pub const fn is_empty(&$self) -> bool {
                $self.len() == 0
            }

            /// # Safety
            ///
            /// * Data in the advanced region will be unitialized.
            #[inline]
            pub const unsafe fn set_len(&mut $self, new_len: usize) {
                $len = new_len;
            }

            /// # Safety
            ///
            /// * Data in the advanced region will be unitialized.
            ///
            /// # Panics
            ///
            /// * If buffer does not have at least `n` bytes of spare capacity.
            #[inline]
            #[track_caller]
            pub const unsafe fn advance(&mut $self, n: usize) {
                assert!(n <= $self.spare_capacity_len());

                unsafe { $self.advance_unchecked(n) };
            }

            /// # Safety
            ///
            /// * Buffer must have at least `n` bytes of spare capacity.
            /// * Data in the advanced region will be unitialized.
            #[inline]
            pub const unsafe fn advance_unchecked(&mut $self, n: usize) {
                unsafe { $self.set_len($self.len() + n) };
            }

            /// # Safety
            ///
            /// * Returned slice must be valid: it must not exceed the capacity.
            ///
            /// # Panics
            ///
            /// * If provided slice does not start at the spare capacity.
            pub fn init_with<F: FnOnce(&mut [std::mem::MaybeUninit<u8>]) -> &[u8]>(
                &mut $self,
                f: F
            ) {
                let slice = f($self.spare_capacity_mut());

                let ptr = slice.as_ptr();
                let len = slice.len();

                assert_eq!(ptr, $self.spare_capacity_ptr());

                // SAFETY: slice is valid and belongs to the buffer.
                unsafe { $self.advance_unchecked(len) };
            }

            /// # Safety
            ///
            /// * Returned slice must be valid: it must not exceed the capacity.
            ///
            /// # Panics
            ///
            /// * If provided slice does not start at the spare capacity.
            pub fn try_init_with<E, F: FnOnce(&mut [std::mem::MaybeUninit<u8>]) -> Result<&[u8], E>>(
                &mut $self,
                f: F
            ) -> Result<(), E> {
                let slice = f($self.spare_capacity_mut())?;

                let ptr = slice.as_ptr();
                let len = slice.len();

                assert_eq!(ptr, $self.spare_capacity_ptr());

                // SAFETY: slice is valid and belongs to the buffer.
                unsafe { $self.advance_unchecked(len) };

                Ok(())
            }

            /// # Safety
            ///
            /// * The [`ReadBuf`] must not contain uninitialized data.
            ///
            /// # Panics
            ///
            /// * If provided [`ReadBuf`] does not start at the spare capacity.
            ///
            /// [`ReadBuf`]: tokio::io::ReadBuf
            #[cfg(feature = "read-buf")]
            pub fn read_with<T, F: FnOnce(&mut tokio::io::ReadBuf) -> T>(
                &mut $self,
                f: F
            ) -> T {
                let mut read_buf = tokio::io::ReadBuf::uninit($self.spare_capacity_mut());

                let value = f(&mut read_buf);

                let ptr = read_buf.filled().as_ptr();
                let len = read_buf.filled().len();

                assert_eq!(ptr, $self.spare_capacity_ptr());

                // SAFETY: slice is valid and belongs to the buffer.
                unsafe { $self.advance_unchecked(len) };

                value
            }

            #[inline]
            pub const fn truncate(&mut $self, new_len: usize) {
                if $self.len() > new_len {
                    // SAFETY: truncating will not expose uninitialized data.
                    unsafe { $self.set_len(new_len) }
                }
            }

            #[inline]
            pub const fn clear(&mut $self) {
                // SAFETY: truncating will not expose uninitialized data.
                unsafe { $self.set_len(0) }
            }

            #[inline]
            pub const fn as_slice(&$self) -> &[u8] {
                unsafe { std::slice::from_raw_parts($self.as_ptr(), $self.len()) }
            }

            #[inline]
            pub const fn as_mut_slice(&mut $self) -> &mut [u8] {
                unsafe { std::slice::from_raw_parts_mut($self.as_mut_ptr(), $self.len()) }
            }

            #[inline]
            pub const fn spare_capacity_non_null(&$self) -> NonNull<u8> {
                unsafe { $self.as_non_null().add($self.len()) }
            }

            #[inline]
            pub const fn spare_capacity_mut_ptr(&$self) -> *mut u8 {
                $self.spare_capacity_non_null().as_ptr()
            }

            #[inline]
            pub const fn spare_capacity_ptr(&$self) -> *const u8 {
                $self.spare_capacity_mut_ptr().cast_const()
            }

            #[inline]
            pub const fn spare_capacity_len(&$self) -> usize {
                unsafe { $self.capacity().unchecked_sub($self.len()) }
            }

            #[inline]
            pub const fn has_spare_capacity(&$self) -> bool {
                $self.len() < $self.capacity()
            }

            #[inline]
            pub const fn spare_capacity_mut(&mut $self) -> &mut [std::mem::MaybeUninit<u8>] {
                unsafe { std::slice::from_raw_parts_mut(
                    $self.spare_capacity_mut_ptr().cast(),
                    $self.spare_capacity_len(),
                ) }
            }

            #[inline]
            pub const fn extend_from_slice(&mut $self, other: &[u8]) {
                let len = other.len();

                assert!(len <= $self.spare_capacity_len());

                let src = std::ptr::NonNull::from_ref(other).cast();

                unsafe {
                    $self.spare_capacity_non_null()
                        .copy_from_nonoverlapping(src, other.len())
                };

                $len += other.len();
            }

            #[inline]
            pub const fn extend_from_array<const M: usize>(&mut $self, other: &[u8; M]) {
                assert!(M <= $self.spare_capacity_len());

                let src = std::ptr::NonNull::from_ref(other).cast();

                unsafe {
                    $self.spare_capacity_non_null()
                        .copy_from_nonoverlapping(src, other.len())
                };

                $len += M;
            }
        )?

        $(
            #[inline]
            pub const fn capacity(&$self) -> usize {
                $const_capacity
            }

            #[inline]
            pub const fn as_mut_uninit_array(&mut $self) -> &mut [std::mem::MaybeUninit<u8>; $const_capacity] {
                unsafe { $self.as_non_null().cast().as_mut() }
            }
        )?

        $(
            #[inline]
            pub const fn capacity(&$self) -> usize {
                $capacity
            }

            #[inline]
            pub const fn as_mut_uninit_slice(&mut $self) -> &mut [std::mem::MaybeUninit<u8>] {
                unsafe { std::slice::from_raw_parts_mut($self.as_mut_ptr().cast(), $self.capacity()) }
            }
        )?

        #[inline]
        pub fn can_unsplit_raw_back<const R: usize>(&$self, r: &crate::Raw<R>) -> bool {
            unsafe { $inner.can_unsplit($self.capacity(), &r.inner) }
        }

        #[inline]
        pub fn can_unsplit_raw_front<const L: usize>(&$self, l: &crate::Raw<L>) -> bool {
            unsafe { l.inner.can_unsplit(L, &$inner) }
        }

        #[inline]
        pub fn can_unsplit_buf_back<const R: usize>(&$self, r: &crate::Buf<R>) -> bool {
            unsafe { $inner.can_unsplit($self.capacity(), &r.raw.inner) }
        }

        #[inline]
        pub fn can_unsplit_buf_front<const L: usize>(&$self, l: &crate::Buf<L>) -> bool {
            unsafe { l.raw.inner.can_unsplit(L, &$inner) }
        }

        #[inline]
        pub fn can_unsplit_dyn_raw_back(&$self, r: &crate::DynRaw) -> bool {
            unsafe { $inner.can_unsplit($self.capacity(), &r.inner) }
        }

        #[inline]
        pub fn can_unsplit_dyn_raw_front(&$self, l: &crate::DynRaw) -> bool {
            unsafe { l.inner.can_unsplit(l.capacity, &$inner) }
        }

        #[inline]
        pub fn can_unsplit_dyn_buf_back(&$self, r: &crate::DynBuf) -> bool {
            unsafe { $inner.can_unsplit($self.capacity(), &r.raw.inner) }
        }

        #[inline]
        pub fn can_unsplit_dyn_buf_front(&$self, l: &crate::DynBuf) -> bool {
            unsafe { l.raw.inner.can_unsplit(l.raw.capacity, &$inner) }
        }
    };
}

pub(crate) use common_impl;
