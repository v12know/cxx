use alloc::borrow::Cow;
use alloc::string::String;
use core::borrow::Borrow;
use core::cmp::Ordering;
use core::convert::AsRef;
use core::fmt::{self, Debug, Display};
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::slice;
use core::str::{self, Utf8Error};

use crate::CxxString;

extern "C" {
    #[link_name = "cxxbridge1$cxx_string_view$init"]
    fn string_view_init(this: &mut MaybeUninit<CxxStringView>, data: *const u8, len: usize);
    #[link_name = "cxxbridge1$cxx_string_view$data"]
    fn string_view_data(this: &CxxStringView) -> *const u8;
    #[link_name = "cxxbridge1$cxx_string_view$length"]
    fn string_view_length(this: &CxxStringView) -> usize;
}

/// Binding to a C++ `std::string_view`
#[repr(C)]
pub struct CxxStringView<'a> {
    // Static asserts in cxx.cc ensure this size is correct.
    _storage: MaybeUninit<[usize; 2]>,
    _phantom: PhantomData<&'a [u8]>,
}

impl CxxStringView<'static> {
    /// Constructs an empty string view.
    ///
    /// Similar to the behavior of C++ [std::string_view::string_view][ctors] #1 (the default constructor),
    /// but does not guarantee that `data` will be `nullptr`.
    ///
    /// [ctors]: https://en.cppreference.com/w/cpp/string/basic_string_view/basic_string_view
    pub fn empty() -> Self {
        Self::new(&[])
    }
}

impl<'a> CxxStringView<'a> {
    /// Constructs a string view containing the first `len` bytes of the array starting at `data`.
    ///
    /// Matches the behavior of C++ [std::string_view::string_view][ctors] #3.
    ///
    /// SAFETY:
    ///   Either `len` must be 0, or `data` and `len` must satisfy the safety invariants of [`core::slice::from_raw_parts<'a, u8>`][slice].
    ///
    /// [ctors]: https://en.cppreference.com/w/cpp/string/basic_string_view/basic_string_view
    /// [slice]: core::slice::from_raw_parts
    unsafe fn from_raw_parts(data: *const u8, len: usize) -> Self {
        let mut result = Self {
            _storage: MaybeUninit::uninit(),
            _phantom: PhantomData,
        };
        let this = result
            ._storage
            .as_mut_ptr()
            .cast::<MaybeUninit<CxxStringView>>();
        unsafe { string_view_init(&mut *this, data, len) };
        result
    }

    /// Constructs a string view from a reference to a `[u8]`. The string view is
    /// live as long as the backing slice.
    ///
    /// Loosely matches the behavior of C++ [std::string_view::string_view][ctors] #3.
    ///
    /// [ctors]: https://en.cppreference.com/w/cpp/string/basic_string_view/basic_string_view
    pub fn new<T: AsRef<[u8]> + ?Sized>(slice: &'a T) -> Self {
        let slice = slice.as_ref();
        let data = slice.as_ptr();
        let len = slice.len();
        unsafe { Self::from_raw_parts(data, len) }
    }

    /// Returns the length of the string view in bytes.
    ///
    /// Matches the behavior of C++ [std::string_view::size][size].
    ///
    /// [size]: https://en.cppreference.com/w/cpp/string/basic_string_view/size
    pub fn len(&self) -> usize {
        unsafe { string_view_length(self) }
    }

    /// Returns true if `self` has a length of zero bytes.
    ///
    /// Matches the behavior of C++ [std::string_view::empty][empty].
    ///
    /// [empty]: https://en.cppreference.com/w/cpp/string/basic_string_view/empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Produces a pointer to the first character of the string.
    ///
    /// Matches the behavior of C++ [std::string_view::data][data].
    ///
    /// Note that the return type may look like `const char *` but is not a
    /// `const char *` in the typical C sense, as C++ string views may contain
    /// internal null bytes. As such, the returned pointer only makes sense as a
    /// string in combination with the length returned by [`len()`][len].
    ///
    /// [data]: https://en.cppreference.com/w/cpp/string/basic_string_view/data
    /// [len]: #method.len
    pub fn as_ptr(&self) -> *const u8 {
        unsafe { string_view_data(self) }
    }

    /// Returns a byte slice of this string view's contents.
    pub fn as_bytes(&self) -> &[u8] {
        let data = self.as_ptr();
        let len = self.len();

        // string_view's data can be nullptr if its size is zero, but
        // a slice's data isn't allowed to be null.
        let data = if !data.is_null() {
            data
        } else {
            debug_assert_eq!(len, 0);
            core::ptr::NonNull::dangling().as_ptr()
        };

        // Safety:
        //   * `data` can't be null because of the check above
        //   * If `len` is non-zero, `data` came either from a valid `[u8]`, or from a C++ `string_view`
        unsafe { slice::from_raw_parts(data, len) }
    }

    /// Validates that the C++ string view contains UTF-8 data and produces a view of
    /// it as a Rust &amp;str, otherwise an error.
    pub fn to_str(&self) -> Result<&str, Utf8Error> {
        str::from_utf8(self.as_bytes())
    }

    /// If the contents of the C++ string view are valid UTF-8, this function returns
    /// a view as a Cow::Borrowed &amp;str. Otherwise replaces any invalid UTF-8
    /// sequences with the U+FFFD [replacement character] and returns a
    /// Cow::Owned String.
    ///
    /// [replacement character]: https://doc.rust-lang.org/std/char/constant.REPLACEMENT_CHARACTER.html
    pub fn to_string_lossy(&self) -> Cow<str> {
        String::from_utf8_lossy(self.as_bytes())
    }
}

impl<'a> Display for CxxStringView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(self.to_string_lossy().as_ref(), f)
    }
}

impl<'a> Debug for CxxStringView<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self.to_string_lossy().as_ref(), f)
    }
}

impl<'a> PartialEq for CxxStringView<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl<'a> PartialEq<CxxStringView<'a>> for str {
    fn eq(&self, other: &CxxStringView<'a>) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl<'a> PartialEq<str> for CxxStringView<'a> {
    fn eq(&self, other: &str) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl<'a> PartialEq<CxxStringView<'a>> for CxxString {
    fn eq(&self, other: &CxxStringView<'a>) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl<'a> PartialEq<CxxString> for CxxStringView<'a> {
    fn eq(&self, other: &CxxString) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl<'a> Eq for CxxStringView<'a> {}

impl<'a> PartialOrd for CxxStringView<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_bytes().partial_cmp(other.as_bytes())
    }
}

impl<'a> Ord for CxxStringView<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_bytes().cmp(other.as_bytes())
    }
}

impl<'a> Hash for CxxStringView<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_bytes().hash(state);
    }
}

impl<'a> AsRef<[u8]> for CxxStringView<'a> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<'a> Borrow<[u8]> for CxxStringView<'a> {
    fn borrow(&self) -> &[u8] {
        self.as_bytes()
    }
}
