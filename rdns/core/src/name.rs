use crate::error::RDNSError;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::iter::Peekable;
use std::vec::IntoIter;

#[derive(Debug, Clone)]
pub struct Name(Vec<u8>);

impl Name {
    pub fn root() -> Self {
        Name(vec![0])
    }

    pub fn parse(
        repr: &mut Peekable<IntoIter<u8>>,
        stop_chars: HashSet<u8>,
    ) -> Result<Name, RDNSError> {
        let result = parser::NameParser::parse(repr, stop_chars)?;
        Ok(Name(result))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_relative(&self) -> bool {
        // Null terminated because of ending a '.'
        self.0.last() == Some(&0)
    }

    pub fn raw(&self) -> Vec<u8> {
        self.0.clone()
    }
}

impl<'a> TryFrom<String> for Name {
    type Error = RDNSError;

    fn try_from(repr: String) -> Result<Self, Self::Error> {
        parser::NameParser::parse_repr(repr).map(|v| Name(v))
    }
}

impl Display for Name {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", <Name as Into<String>>::into(self.clone()))
    }
}

impl Into<String> for Name {
    fn into(self) -> String {
        if self.0.len() == 1 && *self.0.get(0).unwrap() == 0 {
            return ".".to_string();
        }

        let mut bytes = self.0.clone();
        let mut pos = 0;

        while let Some(&len) = bytes.get(pos) {
            *bytes.get_mut(pos).unwrap() = b'.';
            if len == 0 {
                break;
            }
            pos += len as usize + 1;
        }

        String::from_utf8(bytes[1..].to_owned()).unwrap()
    }
}

impl Into<Vec<u8>> for Name {
    fn into(self) -> Vec<u8> {
        self.0
    }
}

pub mod parser {
    use crate::error::RDNSError;
    use std::collections::HashSet;
    use std::iter::Peekable;
    use std::vec::IntoIter;

    pub struct NameParser<'a> {
        repr: &'a mut Peekable<IntoIter<u8>>,
        pos: u8,
        label_pos: u8,
        result: Vec<u8>,
    }

    impl<'a> NameParser<'_> {
        fn new(repr: &'a mut Peekable<IntoIter<u8>>) -> Result<NameParser<'a>, RDNSError> {
            Ok(NameParser {
                repr,
                pos: 0,
                label_pos: 0,
                result: Vec::new(),
            })
        }

        pub fn parse_repr(repr: String) -> Result<Vec<u8>, RDNSError> {
            if repr.len() > 255 {
                return Err(RDNSError::NameTooLong(repr.len()));
            }

            NameParser::parse(
                &mut repr.into_bytes().into_iter().peekable(),
                HashSet::new(),
            )
        }

        pub fn parse(
            repr: &mut Peekable<IntoIter<u8>>,
            stop_chars: HashSet<u8>,
        ) -> Result<Vec<u8>, RDNSError> {
            let mut parser = NameParser::new(repr)?;

            match parser.repr.peek() {
                Some(b'.') => {
                    parser.result.push(0);
                }
                Some(_) => parser.parse_subdomain(stop_chars)?,
                None => return Err(RDNSError::NameInvalid()),
            };

            if parser.result.len() > 255 {
                return Err(RDNSError::NameTooLong(parser.result.len()));
            }

            Ok(parser.result)
        }

        fn parse_subdomain(&mut self, stop_chars: HashSet<u8>) -> Result<(), RDNSError> {
            self.parse_label()?;

            match self.repr.peek() {
                Some(b'.') => {
                    self.pos += 1;
                    self.repr.next();

                    if self.repr.peek().is_none() || stop_chars.contains(self.repr.peek().unwrap())
                    {
                        self.result.push(0);
                    } else {
                        self.parse_subdomain(stop_chars)?;
                    }

                    Ok(())
                }
                Some(x) => {
                    if stop_chars.contains(x) {
                        Ok(())
                    } else {
                        Err(RDNSError::NameLabelInvalid(self.pos))
                    }
                }
                None => Ok(()),
            }
        }

        fn parse_label(&mut self) -> Result<(), RDNSError> {
            // Placeholder length
            self.result.push(0);

            let mut prev;
            if self.current_is_letter() {
                self.pos += 1;
                self.label_pos += 1;
                prev = self.repr.next().unwrap();
                self.result.push(prev as u8);
            } else {
                return Err(RDNSError::NameLabelInvalid(self.pos));
            }

            while self.current_is_letter_digit_hyphen() {
                self.pos += 1;
                self.label_pos += 1;
                prev = self.repr.next().unwrap();
                self.result.push(prev as u8);
            }

            if NameParser::is_hyphen(prev) {
                return Err(RDNSError::NameLabelInvalid(self.pos - 1));
            }

            if self.label_pos > 63 {
                return Err(RDNSError::NameLabelTooLong(self.label_pos));
            }

            *self
                .result
                .get_mut((self.pos - self.label_pos) as usize)
                .unwrap() = self.label_pos;
            self.label_pos = 0;

            Ok(())
        }

        fn current_is_letter(&mut self) -> bool {
            if let Some(&ch) = self.repr.peek() {
                if NameParser::is_letter(ch) {
                    return true;
                }
            }

            false
        }

        fn current_is_letter_digit_hyphen(&mut self) -> bool {
            if let Some(&ch) = self.repr.peek() {
                if NameParser::is_letter(ch)
                    || NameParser::is_digit(ch)
                    || NameParser::is_hyphen(ch)
                {
                    return true;
                }
            }

            false
        }

        #[inline]
        fn is_letter(ch: u8) -> bool {
            match ch {
                b'a'..=b'z' | b'A'..=b'Z' => true,
                _ => false,
            }
        }

        #[inline]
        fn is_digit(ch: u8) -> bool {
            match ch {
                b'0'..=b'9' => true,
                _ => false,
            }
        }

        #[inline]
        fn is_hyphen(ch: u8) -> bool {
            ch == b'-'
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::error::RDNSError;
    use crate::name::{parser, Name};
    use crate::test;
    use std::collections::HashSet;

    #[test]
    fn root_name() {
        let name = Name::try_from(".".to_string()).unwrap();
        assert_eq!(vec![0; 1], name.0);
    }

    #[test]
    fn example_dot_com_relative() {
        let test_name = "example.com".to_string();
        let name = Name::try_from(test_name.clone()).unwrap();

        let expected = test::dirty_to_bytes(test_name);
        assert_eq!(expected, name.0);
    }

    #[test]
    fn example_dot_com_absolute() {
        let test_name = "example.com.".to_string();
        let name = Name::try_from(test_name.clone()).unwrap();

        let expected = test::dirty_to_bytes(test_name);
        assert_eq!(expected, name.0);
    }

    #[test]
    fn four_part_name() {
        let test_name = "a.b.example.com".to_string();
        let name = Name::try_from(test_name.clone()).unwrap();

        let expected = test::dirty_to_bytes(test_name);
        assert_eq!(expected, name.0);
    }

    #[test]
    fn hyphen_inside_label() {
        let test_name = "a-b.example.com".to_string();
        let name = Name::try_from(test_name.clone()).unwrap();

        let expected = test::dirty_to_bytes(test_name);
        assert_eq!(expected, name.0);
    }

    #[test]
    fn label_must_not_start_with_hyphen() {
        let test_name = "-a.example.com".to_string();
        let name = Name::try_from(test_name).unwrap_err();

        assert!(matches!(name, RDNSError::NameLabelInvalid(0)));
    }

    #[test]
    fn label_must_not_end_with_hyphen() {
        let test_name = "a-.example.com".to_string();
        let name = Name::try_from(test_name).unwrap_err();

        assert!(matches!(name, RDNSError::NameLabelInvalid(1)));
    }

    #[test]
    fn label_must_not_contain_invalid_characters() {
        let test_name = "a.ex@mple.com".to_string();
        let name = Name::try_from(test_name).unwrap_err();

        assert!(matches!(name, RDNSError::NameLabelInvalid(4)));
    }

    #[test]
    fn empty_string_not_a_valid_name() {
        let test_name = String::new();
        let name = Name::try_from(test_name).unwrap_err();

        assert!(matches!(name, RDNSError::NameInvalid()));
    }

    #[test]
    fn name_too_long() {
        let label = vec!['a' as u8; 63];
        let mut name = Vec::new();
        for _ in 0..5 {
            name.extend(label.iter());
            name.push('.' as u8)
        }
        let test_name = String::from_utf8(name).unwrap();
        let name = Name::try_from(test_name).unwrap_err();

        assert!(matches!(name, RDNSError::NameTooLong(320)));
    }

    #[test]
    fn name_label_too_long() {
        let mut name = vec!['a' as u8; 65];
        name.extend_from_slice(".com".as_bytes());
        let test_name = String::from_utf8(name).unwrap();
        let name = Name::try_from(test_name).unwrap_err();

        assert!(matches!(name, RDNSError::NameLabelTooLong(65)));
    }

    #[test]
    fn name_label_would_overflow_byte() {
        let mut name = vec!['a' as u8; 260];
        name.extend_from_slice(".com".as_bytes());
        let test_name = String::from_utf8(name).unwrap();
        let name = Name::try_from(test_name).unwrap_err();

        assert!(matches!(name, RDNSError::NameTooLong(264)));
    }

    #[test]
    fn round_trip_example_dot_com_relative() {
        let test_name = "example.com".to_string();
        let name = Name::try_from(test_name.clone()).unwrap();

        assert_eq!(test_name, <Name as Into<String>>::into(name));
    }

    #[test]
    fn round_trip_example_dot_com_absolute() {
        let test_name = "example.com.".to_string();
        let name = Name::try_from(test_name.clone()).unwrap();

        assert_eq!(test_name, <Name as Into<String>>::into(name));
    }

    #[test]
    fn parse_with_stop_pattern_for_example_dot_com_absolute() {
        let test_name = "example.com. ".to_string();
        let name = Name(
            parser::NameParser::parse(
                &mut test_name.clone().into_bytes().into_iter().peekable(),
                HashSet::from([b' ']),
            )
            .unwrap(),
        );

        assert_eq!(
            test_name.trim_end().to_string(),
            <Name as Into<String>>::into(name)
        );
    }

    #[test]
    fn parse_with_stop_pattern_for_example_dot_com_relative() {
        let test_name = "example.com ".to_string();
        let name = Name(
            parser::NameParser::parse(
                &mut test_name.clone().into_bytes().into_iter().peekable(),
                HashSet::from([b' ']),
            )
            .unwrap(),
        );

        assert_eq!(
            test_name.trim_end().to_string(),
            <Name as Into<String>>::into(name)
        );
    }

    #[test]
    fn parse_with_multi_stop_pattern() {
        let test_name = "example.com\t".to_string();
        let name = Name(
            parser::NameParser::parse(
                &mut test_name.clone().into_bytes().into_iter().peekable(),
                HashSet::from([b' ', b'\t']),
            )
            .unwrap(),
        );

        assert_eq!(
            test_name.trim_end().to_string(),
            <Name as Into<String>>::into(name)
        );
    }
}
