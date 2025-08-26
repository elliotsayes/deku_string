use core::ops::Deref as _;

use alloc::{string::String, vec, vec::Vec};

#[cfg(feature = "bstr")]
use bstr::{BString, ByteVec};
use deku::ctx::{Endian, Limit};
use deku::reader::Reader;
use deku::writer::Writer;
use deku::{DekuError, DekuReader, DekuWriter, deku_error, no_std_io};

use crate::{Encoding, InternalValue, SevenBitU32, Size, StringDeku, StringLayout};

impl StringDeku {
    pub(self) fn from_reader_with_ctx_impl<R>(
        reader: &mut Reader<R>,
        endian: Endian,
        encoding: Encoding,
        layout: StringLayout,
    ) -> Result<Self, DekuError>
    where
        R: no_std_io::Read + no_std_io::Seek,
    {
        let (null_requirement, limit_u8, limit_u16, limit_u32): ReadRequirements =
            read_requirements(reader, endian, layout)?;

        // other limits are the same, but have different types.
        // This won't ever match for zero-ended strings
        if limit_u8 == Limit::Count(0) {
            // if requested length is 0, skip the data
            #[cfg(not(feature = "bstr"))]
            return Ok(StringDeku::from(""));
            #[cfg(feature = "bstr")]
            return Ok(StringDeku::from(BString::from("")));
        }

        match encoding {
            Encoding::Utf8 => {
                read_string(reader, &null_requirement, limit_u8, endian, false, |buf| {
                    String::from_utf8(buf.to_vec())
                        .map_err(|_| deku_error!(DekuError::Parse, "Invalid UTF-8"))
                        .map(Into::into)
                })
            }
            Encoding::Utf16 => {
                read_string(reader, &null_requirement, limit_u16, endian, false, |buf| {
                    String::from_utf16(buf)
                        .map_err(|_| deku_error!(DekuError::Parse, "Invalid UTF-16"))
                        .map(Into::into)
                })
            }
            Encoding::Utf32 => {
                read_string(reader, &null_requirement, limit_u32, endian, false, |buf| {
                    let mut result: Vec<char> = vec![];
                    buf.iter().try_fold((), |_, value| {
                        let Some(ch) = char::from_u32(*value) else {
                            return Err(deku_error!(
                                DekuError::Parse,
                                "Invalid UTF-32"
                            ));
                        };
                        result.push(ch);
                        Ok(())
                    })?;
                    Ok(result.into_iter().collect())
                })
            }
            #[cfg(feature = "bstr")]
            Encoding::BinUtf8 => {
                read_string(reader, &null_requirement, limit_u8, endian, true, |buf| {
                    Ok(bstr::BString::new(buf.to_vec()))
                })
            }
        }
    }

    pub(self) fn to_writer_impl<W: no_std_io::Write + no_std_io::Seek>(
        &self,
        writer: &mut Writer<W>,
        endian: Endian,
        encoding: Encoding,
        layout: StringLayout,
    ) -> Result<(), DekuError> {
        match encoding {
            Encoding::Utf8 => {
                #[cfg(not(feature = "bstr"))]
                let mut buf = self.internal_ref().as_bytes().to_vec();
                #[cfg(feature = "bstr")]
                let mut buf = self.internal_ref().deref().to_vec();
                write_string(writer, endian, layout, false, &mut buf)
            }
            Encoding::Utf16 => {
                #[cfg(not(feature = "bstr"))]
                let mut buf = self.internal_ref().encode_utf16().collect::<Vec<u16>>();
                #[cfg(feature = "bstr")]
                let mut buf = unsafe {
                    self.internal_ref()
                        .deref()
                        .to_vec()
                        .into_string_unchecked()
                        .encode_utf16()
                        .collect::<Vec<u16>>()
                };
                write_string(writer, endian, layout, false, &mut buf)
            }
            Encoding::Utf32 => {
                #[cfg(not(feature = "bstr"))]
                let mut buf = self
                    .internal_ref()
                    .chars()
                    .map(|ch| ch.into())
                    .collect::<Vec<u32>>();
                #[cfg(feature = "bstr")]
                let mut buf = unsafe {
                    self.internal_ref()
                        .deref()
                        .to_vec()
                        .into_string_unchecked()
                        .chars()
                        .map(|ch| ch.into())
                        .collect::<Vec<u32>>()
                };
                write_string(writer, endian, layout, false, &mut buf)
            }
            #[cfg(feature = "bstr")]
            Encoding::BinUtf8 => {
                let mut buf = self.internal_ref().to_vec();
                write_string(writer, endian, layout, true, &mut buf)
            }
        }
    }
}

impl DekuReader<'_, (Endian, Encoding, StringLayout)> for StringDeku {
    /// Read string from reader
    fn from_reader_with_ctx<R: no_std_io::Read + no_std_io::Seek>(
        reader: &mut Reader<R>,
        ctx: (Endian, Encoding, StringLayout),
    ) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        let (endian, encoding, layout) = ctx;
        Self::from_reader_with_ctx_impl(reader, endian, encoding, layout)
    }
}

impl DekuReader<'_, (Endian, (Encoding, StringLayout))> for StringDeku {
    /// Read string from reader
    fn from_reader_with_ctx<R: no_std_io::Read + no_std_io::Seek>(
        reader: &mut Reader<R>,
        ctx: (Endian, (Encoding, StringLayout)),
    ) -> Result<Self, DekuError>
    where
        Self: Sized,
    {
        let (endian, (encoding, layout)) = ctx;
        Self::from_reader_with_ctx_impl(reader, endian, encoding, layout)
    }
}

impl DekuWriter<(Endian, Encoding, StringLayout)> for StringDeku {
    /// Write string to the writer.
    fn to_writer<W: no_std_io::Write + no_std_io::Seek>(
        &self,
        writer: &mut Writer<W>,
        ctx: (Endian, Encoding, StringLayout),
    ) -> Result<(), DekuError> {
        let (endian, encoding, layout) = ctx;
        self.to_writer_impl(writer, endian, encoding, layout)
    }
}

impl DekuWriter<(Endian, (Encoding, StringLayout))> for StringDeku {
    /// Write string to the writer.
    fn to_writer<W: no_std_io::Write + no_std_io::Seek>(
        &self,
        writer: &mut Writer<W>,
        ctx: (Endian, (Encoding, StringLayout)),
    ) -> Result<(), DekuError> {
        let (endian, (encoding, layout)) = ctx;
        self.to_writer_impl(writer, endian, encoding, layout)
    }
}

/// Read Requirements tuple.
///
/// Type is defined for convenience.
/// There's no option to define it just with size only.
type ReadRequirements = (
    NullRequirement,
    Limit<u8, fn(&u8) -> bool>,
    Limit<u16, fn(&u16) -> bool>,
    Limit<u32, fn(&u32) -> bool>,
);

/// Read limit and null placement requirements from layout and reader (if prefixed)
///
/// Zero-ended gives another limit kind that based on size, thus result can't be just a size.
fn read_requirements<R: no_std_io::Read + no_std_io::Seek>(
    reader: &mut Reader<R>,
    endian: Endian,
    layout: StringLayout,
) -> Result<ReadRequirements, DekuError> {
    match layout {
        StringLayout::FixedLength {
            size,
            allow_no_null,
        } => {
            let null_requirement = if allow_no_null {
                NullRequirement::Accepted
            } else {
                NullRequirement::Required
            };
            Ok((
                null_requirement,
                Limit::from(size),
                Limit::from(size),
                Limit::from(size),
            ))
        }
        StringLayout::ZeroEnded => Ok((
            // zero is already at the end by how deku reads data
            NullRequirement::Accepted,
            Limit::new_until(|v: &u8| *v == 0),
            Limit::new_until(|v: &u16| *v == 0),
            Limit::new_until(|v: &u32| *v == 0),
        )),
        StringLayout::LengthPrefix(prefix) => {
            let size: usize = match prefix {
                Size::U8 => <u8>::from_reader_with_ctx(reader, endian)? as usize,
                Size::U16 => <u16>::from_reader_with_ctx(reader, endian)? as usize,
                Size::U32 => <u32>::from_reader_with_ctx(reader, endian)? as usize,
                Size::U32_7Bit => {
                    let length: u32 =
                        <SevenBitU32>::from_reader_with_ctx(reader, ())?.into();
                    length as usize
                }
            };
            Ok((
                NullRequirement::Rejected,
                Limit::from(size),
                Limit::from(size),
                Limit::from(size),
            ))
        }
    }
}

/// Common implementation to read String from stream.
///
/// Read data from reader, check null character presence
/// and placement and converts to a string.
fn read_string<'a, R, T>(
    reader: &mut Reader<R>,
    null_requirement: &NullRequirement,
    limit: Limit<T, fn(&T) -> bool>,
    endian: Endian,
    read_to_last_null: bool,
    #[cfg(not(feature = "bstr"))] convert: fn(&[T]) -> Result<String, DekuError>,
    #[cfg(feature = "bstr")] convert: fn(&[T]) -> Result<bstr::BString, DekuError>,
) -> Result<StringDeku, DekuError>
where
    R: no_std_io::Read + no_std_io::Seek,
    T: Default + Clone + PartialEq + DekuReader<'a, Endian>,
{
    let zero = T::default();
    let buf = <Vec<T>>::from_reader_with_ctx(reader, (limit, endian))?;

    // let first_null = buf.iter().position(|x| *x == zero).unwrap_or(buf.len());
    let check_null_pos = match read_to_last_null {
        true => buf.iter().rposition(|x| *x != zero).map(|i| i + 1).unwrap_or(buf.len()),
        false => buf.iter().position(|x| *x == zero).unwrap_or(buf.len()),
    };

    match null_requirement {
        NullRequirement::Accepted => {}
        NullRequirement::Required => {
            if check_null_pos == buf.len() {
                return Err(deku_error!(
                    DekuError::Assertion,
                    "Null must be present in the buffer"
                ));
            }
        }
        NullRequirement::Rejected => {
            if check_null_pos != buf.len() {
                return Err(deku_error!(
                    DekuError::Assertion,
                    "Null must be present in the buffer"
                ));
            }
        }
    }

    convert(&buf[..check_null_pos]).map(Into::into)
}

/// Common implementation to write Vec<u8> and Vec<u16>
fn write_string<W, T>(
    writer: &mut Writer<W>,
    endian: Endian,
    layout: StringLayout,
    write_to_last_null: bool,
    buf: &mut Vec<T>,
) -> Result<(), DekuError>
where
    W: no_std_io::Write + no_std_io::Seek,
    T: Default + Clone + PartialEq + DekuWriter<Endian>,
{
    let zero = T::default();
    // don't write shady strings with null character in the middle

    let check_null_pos: usize = match write_to_last_null {
        true => buf.iter().rposition(|x| *x != zero).map(|i| i + 1).unwrap_or(buf.len()),
        false => {
            let first = buf.iter().position(|x| *x == zero).unwrap_or(buf.len());
            if first != buf.len() {
                return Err(deku_error!(
                    DekuError::Assertion,
                    "Null MUST NOT be present in the binary representation"
                ));
            }
            first
        },
    };

    match layout {
        StringLayout::LengthPrefix(prefix_size) => {
            write_string_length_prefix(writer, endian, buf, prefix_size)
        }
        StringLayout::ZeroEnded => {
            buf.to_writer(writer, endian)?;
            zero.to_writer(writer, endian)
        }

        StringLayout::FixedLength {
            size,
            allow_no_null,
        } => write_string_fixed_length(
            writer,
            endian,
            buf,
            size,
            allow_no_null,
            check_null_pos,
            zero,
        ),
    }
}

fn write_string_length_prefix<W, T>(
    writer: &mut Writer<W>,
    endian: Endian,
    buf: &mut Vec<T>,
    prefix_size: Size,
) -> Result<(), DekuError>
where
    W: no_std_io::Write + no_std_io::Seek,
    T: Default + Clone + PartialEq + DekuWriter<Endian>,
{
    let max_size: usize = match prefix_size {
        Size::U8 => u8::MAX as usize,
        Size::U16 => u16::MAX as usize,
        Size::U32 => u32::MAX as usize,
        Size::U32_7Bit => u32::MAX as usize,
    };

    if buf.len() > max_size {
        return Err(deku_error!(
            DekuError::Assertion,
            "Encoded string length cannot exceed {max_size} bytes"
        ));
    }

    // buffer len is not above corresponding type size,
    // so truncation is safe
    #[allow(clippy::cast_possible_truncation)]
    match prefix_size {
        Size::U8 => ((buf.len() & 0xFF) as u8).to_writer(writer, endian),
        Size::U16 => (buf.len() as u16).to_writer(writer, endian),
        Size::U32 => (buf.len() as u32).to_writer(writer, endian),
        Size::U32_7Bit => {
            let length: SevenBitU32 = (buf.len() as u32).into();
            (length).to_writer(writer, ())
        }
    }?;

    buf.to_writer(writer, endian)
}

fn write_string_fixed_length<W, T>(
    writer: &mut Writer<W>,
    endian: Endian,
    buf: &mut Vec<T>,
    size: usize,
    allow_no_null: bool,
    check_null_pos: usize,
    zero: T,
) -> Result<(), DekuError>
where
    W: no_std_io::Write + no_std_io::Seek,
    T: Default + Clone + PartialEq + DekuWriter<Endian>,
{
    if buf.len() > size {
        return Err(deku_error!(
            DekuError::Assertion,
            "Encoded string length cannot exceed {size} elements"
        ));
    }

    if !allow_no_null && check_null_pos == size {
        return Err(deku_error!(
            DekuError::Assertion,
            "String fills whole output buffer, while Null character must be written"
        ));
    }

    buf.to_writer(writer, endian)?;

    for _ in buf.len()..size {
        zero.to_writer(writer, endian)?;
    }

    Ok(())
}

/// Requirement for null character presence
#[derive(Debug, PartialEq, PartialOrd)]
enum NullRequirement {
    /// Null character is required to be somewhere in a buffer
    Required,

    /// Null character is accepted to be or not to be in a buffer
    Accepted,

    /// Null character is no accepted to be in a buffer
    Rejected,
}
