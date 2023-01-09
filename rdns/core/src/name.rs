use crate::error::RDNSError;

#[derive(Debug, Clone)]
pub struct Name(Vec<u8>);

impl Name {
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'a> TryFrom<&'a str> for Name {
    type Error = RDNSError;

    fn try_from(repr: &str) -> Result<Self, Self::Error> {
        parser::NameParser::parse(repr).map(|v| Name(v))
    }
}

impl Into<String> for Name {
    fn into(self) -> String {
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

mod parser {
    use crate::error::RDNSError;
    use std::iter::Peekable;
    use std::str::Chars;

    pub struct NameParser<'a> {
        repr: Peekable<Chars<'a>>,
        pos: u8,
        label_pos: u8,
        result: Vec<u8>,
    }

    impl<'a> NameParser<'_> {
        fn new(repr: &'a str) -> Result<NameParser<'a>, RDNSError> {
            if repr.len() > 255 {
                return Err(RDNSError::NameTooLong(repr.len()));
            }

            Ok(NameParser {
                repr: repr.chars().peekable(),
                pos: 0,
                label_pos: 0,
                result: Vec::with_capacity(repr.len()),
            })
        }

        pub fn parse(repr: &'a str) -> Result<Vec<u8>, RDNSError> {
            let mut parser = NameParser::new(repr)?;

            match parser.repr.peek() {
                Some('.') => {
                    parser.result.push(0);
                }
                Some(_) => parser.parse_subdomain()?,
                None => return Err(RDNSError::NameInvalid()),
            };

            Ok(parser.result)
        }

        fn parse_subdomain(&mut self) -> Result<(), RDNSError> {
            self.parse_label()?;

            match self.repr.peek() {
                Some('.') => {
                    self.pos += 1;
                    self.repr.next();

                    if self.repr.peek().is_none() {
                        self.result.push(0);
                    } else {
                        self.parse_subdomain()?;
                    }

                    Ok(())
                }
                Some(_) => Err(RDNSError::NameLabelInvalid(self.pos)),
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
        fn is_letter(ch: char) -> bool {
            (ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z')
        }

        #[inline]
        fn is_digit(ch: char) -> bool {
            ch >= '0' && ch <= '9'
        }

        #[inline]
        fn is_hyphen(ch: char) -> bool {
            ch == '-'
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::error::RDNSError;
    use crate::name::Name;
    use crate::test;

    #[test]
    fn root_name() {
        let name = Name::try_from(".").unwrap();
        assert_eq!(vec![0; 1], name.0);
    }

    #[test]
    fn example_dot_com_relative() {
        let test_name = "example.com";
        let name = Name::try_from(test_name).unwrap();

        let expected = test::dirty_to_bytes(test_name);
        assert_eq!(expected, name.0);
    }

    #[test]
    fn example_dot_com_absolute() {
        let test_name = "example.com.";
        let name = Name::try_from(test_name).unwrap();

        let expected = test::dirty_to_bytes(test_name);
        assert_eq!(expected, name.0);
    }

    #[test]
    fn four_part_name() {
        let test_name = "a.b.example.com";
        let name = Name::try_from(test_name).unwrap();

        let expected = test::dirty_to_bytes(test_name);
        assert_eq!(expected, name.0);
    }

    #[test]
    fn hyphen_inside_label() {
        let test_name = "a-b.example.com";
        let name = Name::try_from(test_name).unwrap();

        let expected = test::dirty_to_bytes(test_name);
        assert_eq!(expected, name.0);
    }

    #[test]
    fn label_must_not_start_with_hyphen() {
        let test_name = "-a.example.com";
        let name = Name::try_from(test_name).unwrap_err();

        assert!(matches!(name, RDNSError::NameLabelInvalid(0)));
    }

    #[test]
    fn label_must_not_end_with_hyphen() {
        let test_name = "a-.example.com";
        let name = Name::try_from(test_name).unwrap_err();

        assert!(matches!(name, RDNSError::NameLabelInvalid(1)));
    }

    #[test]
    fn label_must_not_contain_invalid_characters() {
        let test_name = "a.ex@mple.com";
        let name = Name::try_from(test_name).unwrap_err();

        assert!(matches!(name, RDNSError::NameLabelInvalid(4)));
    }

    #[test]
    fn empty_string_not_a_valid_name() {
        let test_name = "";
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
        let name = Name::try_from(test_name.as_str()).unwrap_err();

        assert!(matches!(name, RDNSError::NameTooLong(320)));
    }

    #[test]
    fn name_label_too_long() {
        let mut name = vec!['a' as u8; 65];
        name.extend_from_slice(".com".as_bytes());
        let test_name = String::from_utf8(name).unwrap();
        let name = Name::try_from(test_name.as_str()).unwrap_err();

        assert!(matches!(name, RDNSError::NameLabelTooLong(65)));
    }

    #[test]
    fn name_label_would_overflow_byte() {
        let mut name = vec!['a' as u8; 260];
        name.extend_from_slice(".com".as_bytes());
        let test_name = String::from_utf8(name).unwrap();
        let name = Name::try_from(test_name.as_str()).unwrap_err();

        assert!(matches!(name, RDNSError::NameTooLong(264)));
    }

    #[test]
    fn round_trip_example_dot_com_relative() {
        let test_name = "example.com";
        let name = Name::try_from(test_name).unwrap();

        assert_eq!(test_name, <Name as Into<String>>::into(name));
    }

    #[test]
    fn round_trip_example_dot_com_absolute() {
        let test_name = "example.com.";
        let name = Name::try_from(test_name).unwrap();

        assert_eq!(test_name, <Name as Into<String>>::into(name));
    }
}
