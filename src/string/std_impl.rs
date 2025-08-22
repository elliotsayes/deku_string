//! Additional "transparency" shim implementations for `StringDeku`.

use core::ops::Deref;

use crate::StringDeku;
use alloc::borrow::Cow;
#[cfg(not(feature = "bstr"))]
use alloc::string::String;
#[cfg(feature = "bstr")]
use bstr::{BString, ByteVec};

impl From<&str> for StringDeku {
    fn from(value: &str) -> StringDeku {
        StringDeku(
            #[cfg(not(feature = "bstr"))]
            String::from(value),
            #[cfg(feature = "bstr")]
            BString::from(value),
        )
    }
}

impl From<Cow<'_, str>> for StringDeku {
    fn from(value: Cow<'_, str>) -> StringDeku {
        StringDeku(String::from(value).into())
    }
}

impl PartialEq<StringDeku> for &str {
    fn eq(&self, other: &StringDeku) -> bool {
        self == &other.0
    }
}

impl<'a> PartialEq<&'a str> for StringDeku {
    fn eq(&self, other: &&'a str) -> bool {
        &self.0 == other
    }
}

impl<'a> PartialEq<Cow<'a, str>> for StringDeku {
    fn eq(&self, other: &Cow<'a, str>) -> bool {
        #[cfg(not(feature = "bstr"))]
        let left = &self.0;
        #[cfg(feature = "bstr")]
        let left = &self.0.deref().to_vec().into_string_lossy();
        left == other
    }
}

impl PartialEq<StringDeku> for Cow<'_, str> {
    fn eq(&self, other: &StringDeku) -> bool {
        #[cfg(not(feature = "bstr"))]
        let right = &other.0;
        #[cfg(feature = "bstr")]
        let right = &other.0.deref().to_vec().into_string_lossy();
        self == right
    }
}

#[cfg(test)]
mod test {
    use alloc::borrow::Cow;

    use crate::StringDeku;
    use rstest::rstest;

    #[rstest]
    #[case::str("from str")]
    #[case::str(Cow::from("from str"))]
    fn test_from_eq<T>(#[case] value: T)
    where
        T: Into<StringDeku> + PartialEq<StringDeku> + std::fmt::Debug + Clone,
        StringDeku: PartialEq<T>,
    {
        let str_deku: StringDeku = value.clone().into();

        assert_eq!(value, str_deku);
        assert_eq!(str_deku, value);
    }

    #[rstest]
    #[case::str("from str")]
    #[case::str(Cow::from("from str"))]
    fn test_from_ne<T>(#[case] value: T)
    where
        T: Into<StringDeku> + PartialEq<StringDeku> + std::fmt::Debug + Clone,
        StringDeku: PartialEq<T>,
    {
        let str_deku: StringDeku = "other value".into();

        assert_ne!(value, str_deku);
        assert_ne!(str_deku, value);
    }
}
