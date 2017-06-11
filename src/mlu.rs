use super::*;
use std::fmt;
use std::ffi::CString;
use ffi::wchar_t;
use std::iter::repeat;
use std::ptr;
use std::mem;
use std::char::{decode_utf16, REPLACEMENT_CHARACTER};
use foreign_types::ForeignTypeRef;
use foreign_types::ForeignType;

foreign_type! {
    type CType = ffi::MLU;
    fn drop = ffi::cmsMLUfree;
    pub struct MLU;
    pub struct MLURef;
}

impl MLU {
    pub fn new(items: usize) -> Self {
        unsafe {
            MLU::from_ptr(ffi::cmsMLUalloc(ptr::null_mut(), items as u32))
        }
    }
}

impl MLURef {
    pub fn set_text_ascii(&mut self, text: &str, locale: Locale) -> bool {
        unsafe {
            ffi::cmsMLUsetASCII(self.as_ptr(),
                locale.language_ptr(),
                locale.country_ptr(),
                CString::new(text).unwrap().as_ptr()) != 0
        }
    }

    pub fn set_text(&mut self, text: &str, locale: Locale) -> bool {
        let chars: Vec<_> = text.chars().map(|c| c as wchar_t).chain(repeat(0 as wchar_t).take(1)).collect();

        unsafe {
            ffi::cmsMLUsetWide(self.as_ptr(),
                locale.language_ptr(),
                locale.country_ptr(),
                chars[..].as_ptr()) != 0
        }
    }

    pub fn text_ascii(&self, locale: Locale) -> Option<CString> {
        let len = unsafe {
            ffi::cmsMLUgetASCII(self.as_ptr(),
                locale.language_ptr(),
                locale.country_ptr(),
                ptr::null_mut(), 0)
        };
        if len == 0 {
            return None;
        }
        let mut buf = vec![0u8; len as usize];
        unsafe {
            ffi::cmsMLUgetASCII(self.as_ptr(),
                locale.language_ptr(),
                locale.country_ptr(),
                buf[..].as_ptr() as *mut i8, len);
            if let Some(0) = buf.pop() { // terminating zero
                for c in &mut buf {
                    if *c > 127 {*c = '?' as u8}
                }
                CString::new(buf).ok()
            } else {
                None
            }
        }
    }

    pub fn text(&self, locale: Locale) -> Option<String> {
        let len_bytes = unsafe {
            ffi::cmsMLUgetWide(self.as_ptr(),
                locale.language_ptr(),
                locale.country_ptr(),
                ptr::null_mut(), 0)
        };
        let len_wchars = len_bytes as usize / mem::size_of::<wchar_t>();
        if len_wchars == 0 || (len_bytes&1) != 0 {
            return None;
        }
        let mut buf = vec![0 as wchar_t; len_wchars];
        unsafe {
            ffi::cmsMLUgetWide(self.as_ptr(),
                locale.language_ptr(),
                locale.country_ptr(),
                buf[..].as_ptr() as *mut wchar_t, len_bytes);
            if let Some(0) = buf.pop() { // terminating zero
                Some(decode_utf16(buf.into_iter().map(|c| c as u16))
                   .map(|r| r.unwrap_or(REPLACEMENT_CHARACTER))
                   .collect())
            } else {
                None
            }
        }
    }

    pub fn tanslations(&self) -> Vec<Locale> {
        let count = unsafe { ffi::cmsMLUtranslationsCount(self.as_ptr()) };
        let mut out = Vec::with_capacity(count as usize);
        for i in 0..count {
            let mut locale = Locale::none();
            if unsafe {
                ffi::cmsMLUtranslationsCodes(self.as_ptr(), i,
                    locale.language_ptr_mut(),
                    locale.country_ptr_mut()) != 0
            } {
                out.push(locale);
            }
        }
        out
    }

    /// Obtains the translation rule for given multilocalized unicode object.
    pub fn tanslation(&self, locale: Locale) -> Option<Locale> {
        let mut out = Locale::none();
        if unsafe {
            ffi::cmsMLUgetTranslation(self.as_ptr(),
                locale.language_ptr(),
                locale.country_ptr(),
                out.language_ptr_mut(),
                out.country_ptr_mut()) != 0
        } {
            Some(out)
        } else {
            None
        }
    }
}

impl fmt::Debug for MLURef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let t = self.text(Locale::none());
        write!(f, "MLU({:?} {:?})", if let Some(ref t) = t {&t} else {"None"}, self.tanslations())
    }
}

#[test]
fn mlu() {
    let mut m = MLU::new(1);
    assert!(m.set_text("Hello 世界！", Locale::none()));
    assert_eq!(Some("Hello 世界！".to_owned()), m.text(Locale::none()));
    assert!(!m.set_text_ascii("エッロル", Locale::none()));

    let mut m = MLU::new(1);
    assert!(m.set_text_ascii("OK", Locale::none()));
    assert_eq!(Some(CString::new("OK").unwrap()), m.text_ascii(Locale::none()));
}