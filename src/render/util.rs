#[macro_use]
pub mod macros {
    #[repr(C)] // guarantee 'bytes' comes after '_align'
    pub struct AlignedAs<Align, Bytes: ?Sized> {
        pub _align: [Align; 0],
        pub bytes: Bytes,
    }

    #[macro_export]
    macro_rules! include_bytes_align_as {
        ($align_ty:ty, $path:literal) => {
            {  // const block expression to encapsulate the static
                use $crate::render::util::macros::AlignedAs;

                // this assignment is made possible by CoerceUnsized
                static ALIGNED: &AlignedAs::<$align_ty, [u8]> = &AlignedAs {
                    _align: [],
                    bytes: *include_bytes!($path),
                };

                let (prefix, payload, suffix) = unsafe {ALIGNED.bytes.align_to::<$align_ty>()};

                assert!(prefix.len() == 0);
                assert!(suffix.len() == 0);

                payload
            }
        };
    }
}