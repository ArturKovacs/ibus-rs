//! IBusText
//!
//! Contains a UTF8 string, along with a some attributes that describe
//! underlining, foreground and background color
//!

use std::{any::Any, borrow::Cow, os::raw::c_uint};

use log::{debug, warn};

use dbus::arg::{Append, Arg, ArgType, Get, PropMap, RefArg, Variant};

const ATTRIBUTE_NAME: &'static str = "IBusAttribute";
const ATTRIBUTE_LIST_NAME: &'static str = "IBusAttrList";
const TEXT_NAME: &'static str = "IBusText";

#[derive(Debug, Clone, Copy)]
pub enum UnderlineKind {
    None,
    Single,
    Double,
    Low,
    Error,
}
impl UnderlineKind {
    fn to_value(self) -> c_uint {
        match self {
            Self::None => 0,
            Self::Single => 1,
            Self::Double => 2,
            Self::Low => 3,
            Self::Error => 4,
        }
    }

    fn from_value(v: u32) -> Option<Self> {
        match v {
            0 => Some(Self::None),
            1 => Some(Self::Single),
            2 => Some(Self::Double),
            3 => Some(Self::Low),
            4 => Some(Self::Error),
            _ => None,
        }
    }
}

/// A string attribute kind
#[derive(Debug, Clone, Copy)]
pub enum AttributeKind {
    Underline(UnderlineKind),

    /// The value it contains is the foreground color
    ///
    /// All that the official documentation says about the format is that
    /// it's in RGB (yes, that's not helpful at all)
    ///
    /// My best guess is that it's either of the following:
    ///
    /// The most significant byte is the Red channel, so Red would be   0xff000000
    /// The least significant byte is the Blue channel, so Red would be 0x00ff0000
    ///
    /// Maybe it's in reverse byte order relative to what I just described.
    Foreground(u32),

    /// The value it contains is the background color
    ///
    /// See: `Foreground`
    Background(u32),
}

/// A string attribute
#[derive(Debug, Clone)]
pub struct Attribute {
    pub kind: AttributeKind,

    /// Zero based index of the first character to which this should be applied.
    /// Counting in UTF32 characters.
    /// (Not sure because the official documentation doesn't specify)
    pub start_index: u32,

    /// One plus the zero based index of the last character to which this should be applied.
    /// In other words this is non-inclusive.
    /// Counting in UTF32 characters.
    /// (Not sure because the official documentation doesn't specify)
    pub end_index: u32,
}
type SerializedAttribute<'a> = (&'a str, PropMap, u32, u32, u32, u32);
impl RefArg for Attribute {
    fn arg_type(&self) -> ArgType {
        ArgType::Variant
    }

    fn signature(&self) -> dbus::Signature<'static> {
        <Self as Arg>::signature()
    }

    fn append(&self, i: &mut dbus::arg::IterAppend) {
        let type_: c_uint;
        let value: c_uint;
        match self.kind {
            AttributeKind::Underline(v) => {
                type_ = 1;
                value = v.to_value();
            }
            AttributeKind::Foreground(c) => {
                type_ = 2;
                value = c as c_uint;
            }
            AttributeKind::Background(c) => {
                type_ = 3;
                value = c as c_uint;
            }
        }
        i.append(Variant((
            ATTRIBUTE_NAME,
            PropMap::new(),
            type_,
            value,
            self.start_index as c_uint,
            self.end_index as c_uint,
        )))
    }

    fn as_any(&self) -> &dyn Any
    where
        Self: 'static,
    {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any
    where
        Self: 'static,
    {
        self
    }

    fn box_clone(&self) -> Box<dyn RefArg + 'static> {
        Box::new(Self {
            kind: self.kind,
            start_index: self.start_index,
            end_index: self.end_index,
        })
    }
}
impl Arg for Attribute {
    const ARG_TYPE: ArgType = ArgType::Variant;

    fn signature() -> dbus::Signature<'static> {
        dbus::Signature::from("v\u{0}")
    }
}
impl Append for Attribute {
    fn append_by_ref(&self, i: &mut dbus::arg::IterAppend) {
        <Self as RefArg>::append(self, i);
    }
}
impl<'a> Get<'a> for Attribute {
    fn get(i: &mut dbus::arg::Iter<'a>) -> Option<Self> {
        let mut attrib_var: Variant<dbus::arg::Iter<'a>> = i.get()?;
        let attrib_struct: SerializedAttribute<'a> = attrib_var.0.get()?;
        let struct_name = attrib_struct.0;
        if struct_name != ATTRIBUTE_NAME {
            debug!(
                "Attribute didn't have the expected name. {}",
                ATTRIBUTE_NAME
            );
            return None;
        }

        let type_ = attrib_struct.2;
        let value = attrib_struct.3;
        let start_index = attrib_struct.4;
        let end_index = attrib_struct.5;

        let kind = match type_ {
            1 => AttributeKind::Underline(UnderlineKind::from_value(value)?),
            2 => AttributeKind::Foreground(value),
            3 => AttributeKind::Background(value),
            _ => {
                warn!(
                    "Unexpected attribute type `{}` for {}",
                    type_, ATTRIBUTE_NAME
                );
                return None;
            }
        };
        Some(Attribute {
            kind,
            start_index,
            end_index,
        })
    }
}

type SerializedAttrList<'a> = (&'a str, PropMap, Vec<Attribute>);

fn serialize_attribute_list(
    attributes: &[Attribute],
) -> Variant<(&'static str, PropMap, Vec<Attribute>)> {
    Variant((
        ATTRIBUTE_LIST_NAME,
        PropMap::new(),
        attributes.iter().map(|a| a.clone()).collect::<Vec<_>>(),
    ))
}

fn deserialize_attribute_list<'a>(
    variant: &mut Variant<dbus::arg::Iter<'a>>,
) -> Option<Vec<Attribute>> {
    let list_struct: SerializedAttrList<'a> = match variant.0.get() {
        Some(s) => s,
        None => {
            debug!("Couldn't deserialize attribute list {:?}", variant.0);
            return None;
        }
    };
    let struct_name: &str = list_struct.0;
    if struct_name != ATTRIBUTE_LIST_NAME {
        debug!("Attribute list didn't have the correct name.");
        return None;
    }
    Some(list_struct.2)
}

/// Contains a string and a list of attributes
#[derive(Debug, Clone)]
pub struct Text<'a> {
    string: Cow<'a, str>,
    attributes: Vec<Attribute>,
}
type SerializedText<'a> = (&'a str, PropMap, &'a str, Variant<dbus::arg::Iter<'a>>);
impl<'a> Text<'a> {
    /// Takes a string and a list of attributes
    pub fn new<S, A>(string: S, attributes: A) -> Self
    where
        S: Into<Cow<'a, str>>,
        A: Into<Vec<Attribute>>,
    {
        Self {
            string: string.into(),
            attributes: attributes.into(),
        }
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        self.string.as_ref()
    }

    #[inline]
    pub fn attributes(&self) -> &[Attribute] {
        &self.attributes
    }

    /// See also: `as_str`
    #[inline]
    pub fn into_string(self) -> String {
        self.string.into_owned()
    }
}
impl<'a> From<&'a str> for Text<'a> {
    #[inline]
    fn from(s: &'a str) -> Self {
        Text {
            string: Cow::Borrowed(s),
            attributes: Vec::new(),
        }
    }
}
impl From<String> for Text<'static> {
    #[inline]
    fn from(s: String) -> Self {
        Text {
            string: Cow::Owned(s),
            attributes: Vec::new(),
        }
    }
}
impl<'a> From<Text<'a>> for String {
    #[inline]
    fn from(t: Text<'a>) -> Self {
        t.string.into_owned()
    }
}
impl<'a> AsRef<str> for Text<'a> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<'a> RefArg for Text<'a> {
    fn arg_type(&self) -> ArgType {
        ArgType::Variant
    }

    fn signature(&self) -> dbus::Signature<'static> {
        <Self as Arg>::signature()
    }

    fn append(&self, i: &mut dbus::arg::IterAppend) {
        i.append(Variant((
            TEXT_NAME,
            PropMap::new(),
            self.string.as_ref(),
            serialize_attribute_list(&self.attributes),
        )))
    }

    fn as_any(&self) -> &dyn Any
    where
        Self: 'static,
    {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any
    where
        Self: 'static,
    {
        self
    }

    fn box_clone(&self) -> Box<dyn RefArg + 'static> {
        Box::new(Text::<'static> {
            string: Cow::Owned(self.string.clone().into_owned()),
            attributes: self.attributes.clone(),
        })
    }
}
impl<'a> Append for Text<'a> {
    fn append_by_ref(&self, i: &mut dbus::arg::IterAppend) {
        <Text as RefArg>::append(self, i);
    }
}
impl<'a> Arg for Text<'a> {
    const ARG_TYPE: ArgType = ArgType::Variant;

    fn signature() -> dbus::Signature<'static> {
        // basically just "v" but terminated with 0
        dbus::Signature::from("v\u{0}")
    }
}
impl<'a> Get<'a> for Text<'static> {
    fn get(i: &mut dbus::arg::Iter<'a>) -> Option<Self> {
        let mut text_var: Variant<dbus::arg::Iter<'a>> = i.get()?;

        let mut text_struct: SerializedText<'a> = match text_var.0.get() {
            Some(s) => s,
            None => {
                debug!("Could not get the name. It was {:?}", text_var.0);
                return None;
            }
        };

        let attributes = deserialize_attribute_list(&mut text_struct.3)?;
        Some(Text {
            string: Cow::Owned(text_struct.2.to_owned()),
            attributes,
        })
    }
}
