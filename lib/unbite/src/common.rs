macro_rules! common_impl {
    (
        self: $self:ident ;
        inner: $inner:expr ;
        $( len: $len:expr ; )?
        $( const_capacity: $const_capacity:expr ; )?
        $( capacity: $capacity:expr ; )?
    ) => {
        #[inline]
        pub fn as_non_null(&$self) -> std::ptr::NonNull<u8> {
            $inner.bytes
        }

        #[inline]
        pub fn as_mut_ptr(&$self) -> *mut u8 {
            $self.as_non_null().as_ptr()
        }

        #[inline]
        pub fn as_ptr(&$self) -> *const u8 {
            $self.as_mut_ptr().cast_const()
        }

        $(
            #[inline]
            pub fn len(&$self) -> usize {
                $len
            }

            #[inline]
            pub fn as_slice(&$self) -> &[u8] {
                unsafe { std::slice::from_raw_parts($self.as_ptr(), $self.len()) }
            }

            #[inline]
            pub fn as_mut_slice(&mut $self) -> &mut [u8] {
                unsafe { std::slice::from_raw_parts_mut($self.as_mut_ptr(), $self.len()) }
            }

            #[inline]
            pub fn spare_capacity_non_null(&$self) -> NonNull<u8> {
                unsafe { $self.as_non_null().add($self.len()) }
            }

            #[inline]
            pub fn spare_capacity_mut_ptr(&$self) -> *mut u8 {
                $self.spare_capacity_non_null().as_ptr()
            }

            #[inline]
            pub fn spare_capacity_ptr(&$self) -> *const u8 {
                $self.spare_capacity_mut_ptr().cast_const()
            }

            #[inline]
            pub fn spare_capacity_len(&$self) -> usize {
                $self.capacity() - $self.len()
            }

            #[inline]
            pub fn spare_capacity_mut(&mut $self) -> &mut [std::mem::MaybeUninit<u8>] {
                unsafe { std::slice::from_raw_parts_mut(
                    $self.spare_capacity_mut_ptr().cast(),
                    $self.spare_capacity_len(),
                ) }
            }

            #[inline]
            pub fn extend_from_slice(&mut $self, other: &[u8]) {
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
            pub fn extend_from_array<const M: usize>(&mut $self, other: &[u8; M]) {
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
            pub fn capacity(&$self) -> usize {
                $const_capacity
            }

            #[inline]
            pub fn as_mut_uninit_array(&mut $self) -> &mut [std::mem::MaybeUninit<u8>; $const_capacity] {
                unsafe { $self.as_non_null().cast().as_mut() }
            }
        )?

        $(
            #[inline]
            pub fn capacity(&$self) -> usize {
                $capacity
            }

            #[inline]
            pub fn as_mut_uninit_slice(&mut $self) -> &mut [std::mem::MaybeUninit<u8>] {
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
