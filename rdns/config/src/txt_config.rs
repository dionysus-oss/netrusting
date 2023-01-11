use core::error::RDNSError;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn load_txt_config<P>(path: P) -> Result<(), RDNSError>
where
    P: AsRef<Path>,
{
    let mut lines = read_lines(path)?;

    parser::TxtConfigParser::parse(&mut lines).unwrap();

    Ok(())
}

fn read_lines<P>(path: P) -> io::Result<io::Lines<BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(path)?;
    Ok(BufReader::new(file).lines())
}

mod parser {
    use core::error::RDNSError;
    use std::collections::HashSet;
    use std::io::{BufRead, Lines, Read};
    use std::iter::Peekable;
    use std::str::Chars;

    pub struct TxtConfigParser<'a, R: Read + BufRead> {
        lines: &'a mut Lines<R>,
    }

    impl<'a, R: Read + BufRead> TxtConfigParser<'a, R> {
        fn new(lines: &'a mut Lines<R>) -> Self {
            TxtConfigParser { lines }
        }

        pub fn parse(lines: &'a mut Lines<R>) -> Result<(), RDNSError> {
            let parser = TxtConfigParser::new(lines);

            'lines: while let Some(Ok(next)) = parser.lines.next() {
                let mut line = next.chars().peekable();

                while let Some(ch) = line.peek() {
                    match ch {
                        ' ' | '\t' => {
                            line.next();
                        }
                        ';' => continue 'lines,
                        '$' => parser.parse_control_entry(&mut line)?,
                        _ => {
                            panic!("unrecognised character {}", ch);
                        }
                    };
                }
            }

            Ok(())
        }

        fn parse_control_entry(&self, line: &mut Peekable<Chars>) -> Result<(), RDNSError> {
            line.next();

            let mut control_name = String::new();
            while let Some(&ch) = line.peek() {
                if ch == ' ' || ch == '\t' {
                    break;
                } else if self.is_character(ch) {
                    control_name.push(line.next().unwrap());
                } else {
                    return Err(RDNSError::MasterFileFormatError(
                        "a control directive should only contain letters".to_string(),
                    ));
                }
            }

            println!("control name {}", control_name);

            match control_name.as_str() {
                "ORIGIN" => self.parse_domain_name(line)?,
                _ => {
                    return Err(RDNSError::MasterFileFormatError(format!(
                        "unknown control directive {}",
                        control_name
                    )))
                }
            };

            Ok(())
        }

        fn parse_domain_name(
            &self,
            line: &mut Peekable<Chars>,
        ) -> Result<core::name::Name, RDNSError> {
            if !self.chomp(line) {
                return Err(RDNSError::MasterFileFormatError(
                    "expected whitespace separating the domain name".to_string(),
                ));
            };

            let name = core::name::Name::parse(line, HashSet::from([' ', '\t']))?;
            println!(
                "found name {}",
                <core::name::Name as Into<String>>::into(name.clone())
            );

            Ok(name)
        }

        fn chomp(&self, line: &mut Peekable<Chars>) -> bool {
            let mut any_taken = false;
            while let Some(' ' | '\t') = line.peek() {
                line.next();
                any_taken = true;
            }

            any_taken
        }

        fn is_character(&self, ch: char) -> bool {
            (ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z')
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::txt_config::parser;
    use std::io::{BufRead, Cursor, Lines};

    #[test]
    fn parse_comment_on_own_line() {
        parser::TxtConfigParser::parse(&mut as_lines("; a comment".to_string())).unwrap();
    }

    #[test]
    fn parse_origin_control_directive() {
        parser::TxtConfigParser::parse(&mut as_lines("$ORIGIN example.com".to_string())).unwrap();
    }

    #[test]
    fn parse_origin_control_directive_with_comment() {
        parser::TxtConfigParser::parse(&mut as_lines(
            "$ORIGIN example.com ; some information".to_string(),
        ))
        .unwrap();
    }

    #[test]
    fn parse_origin_control_directive_with_comment_tabs() {
        parser::TxtConfigParser::parse(&mut as_lines(
            "$ORIGIN\texample.com\t; some information".to_string(),
        ))
        .unwrap();
    }

    fn as_lines(input: String) -> Lines<Cursor<String>> {
        Cursor::new(input).lines()
    }
}
