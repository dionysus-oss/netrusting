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

    parser::TxtConfigParser::parse(&mut lines, core::name::Name::root()).unwrap();

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
    use crate::txt_config::read_lines;
    use core::error::RDNSError;
    use std::collections::HashSet;
    use std::io::{BufRead, Lines, Read};
    use std::iter::Peekable;
    use std::net::Ipv4Addr;
    use std::path::PathBuf;
    use std::rc::Rc;
    use std::str::{Chars, FromStr};

    pub struct TxtConfigParser<'a, R: Read + BufRead> {
        lines: &'a mut Lines<R>,
        current_origin: core::name::Name,
        current_name: Option<core::name::Name>,
        current_class: core::RRClass<u16>,
    }

    impl<'a, R: Read + BufRead> TxtConfigParser<'a, R> {
        fn new(lines: &'a mut Lines<R>, origin: core::name::Name) -> Self {
            TxtConfigParser {
                lines,
                current_origin: origin,
                current_name: None,
                current_class: core::RRClass::UNKNOWN(0),
            }
        }

        pub fn parse(
            lines: &'a mut Lines<R>,
            origin: core::name::Name,
        ) -> Result<Vec<core::ResourceRecord>, RDNSError> {
            let mut parser = TxtConfigParser::new(lines, origin);

            let mut records = Vec::new();
            let mut start_of_line: bool;

            println!("origin is {}", parser.current_origin);

            'lines: while let Some(Ok(next)) = parser.lines.next() {
                let mut line = next.chars().peekable();
                start_of_line = true;

                while let Some(ch) = line.peek() {
                    match ch {
                        ' ' | '\t' => {
                            line.next();
                            start_of_line = false;
                        }
                        ';' => continue 'lines,
                        '$' => parser.parse_control_entry(&mut line)?,
                        _ => {
                            let rr = if start_of_line {
                                parser.parse_name_and_rr(&mut line)?
                            } else {
                                parser.parse_rr(&mut line)?
                            };
                            records.push(rr);
                            break 'lines;
                        }
                    };
                }
            }

            println!(
                "final root is {}",
                <core::name::Name as Into<String>>::into(parser.current_origin.clone())
            );

            Ok(records)
        }

        fn parse_control_entry(&mut self, line: &mut Peekable<Chars>) -> Result<(), RDNSError> {
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

            match control_name.as_str() {
                "ORIGIN" => {
                    self.chomp(line);
                    let name = self.parse_domain_name(line)?;
                    self.current_origin = name;
                }
                "INCLUDE" => {
                    let file_name = self.parse_file_name(line)?;
                    let domain_name = self
                        .maybe_parse_domain_name(line)?
                        .unwrap_or(self.current_origin.clone());

                    let mut sub_lines = read_lines(file_name)?;

                    // TODO capture result and push to current RRs
                    TxtConfigParser::parse(&mut sub_lines, domain_name)?;
                }
                _ => {
                    return Err(RDNSError::MasterFileFormatError(format!(
                        "unknown control directive {}",
                        control_name
                    )));
                }
            };

            Ok(())
        }

        fn parse_name_and_rr(
            &mut self,
            line: &mut Peekable<Chars>,
        ) -> Result<core::ResourceRecord, RDNSError> {
            self.current_name = Some(self.parse_domain_name(line)?);

            self.chomp(line);
            let rr = self.parse_rr(line);

            self.current_name = None;

            rr
        }

        fn parse_rr(
            &mut self,
            line: &mut Peekable<Chars>,
        ) -> Result<core::ResourceRecord, RDNSError> {
            let mut ttl_opt = self.try_parse_ttl(line)?;
            self.chomp(line);

            let text = self.get_text(line)?;
            let class: core::RRClass<u16> = text.as_str().try_into().unwrap();
            self.chomp(line);

            let mut rr_type: Option<core::RRType<u16>> = if class == core::RRClass::UNKNOWN(0) {
                Some(text.as_str().try_into().unwrap())
            } else {
                None
            };

            // TODO constant for unknown
            if rr_type == None || rr_type == Some(core::RRType::UNKNOWN(0)) {
                if ttl_opt == None {
                    ttl_opt = self.try_parse_ttl(line)?;
                    self.chomp(line);
                }

                let text = self.get_text(line)?;
                rr_type = Some(text.as_str().try_into().unwrap());
                self.chomp(line);
            }

            if class == core::RRClass::UNKNOWN(0) {
                return Err(RDNSError::MasterFileFormatError("No class".to_string()));
            } else if self.current_class == core::RRClass::UNKNOWN(0) {
                self.current_class = class.clone();
            } else if self.current_class == class {
                // TODO propagate to included files?
                return Err(RDNSError::MasterFileFormatError(
                    "File must only contain one class".to_string(),
                ));
            }

            let rr_data: Rc<dyn core::record::ResourceData> = match rr_type {
                Some(core::RRType::A) => {
                    let ip_address = self.parse_ip_addr(line)?;
                    Rc::new(core::record::AliasResourceData(ip_address))
                }
                //Some(core::RRType::NS) => {}
                Some(core::RRType::CNAME) => {
                    let name = self.parse_domain_name(line)?;
                    Rc::new(core::record::CNameResourceData(name))
                }
                Some(core::RRType::SOA) => Rc::new(self.parse_soa(line)?),
                _ => {
                    return Err(RDNSError::MasterFileFormatError(
                        "unknown resource record type".to_string(),
                    ));
                }
            };

            Ok(core::ResourceRecord {
                name: self.current_name.as_ref().unwrap().clone(),
                rr_type: rr_type.unwrap(),
                class,
                ttl: ttl_opt.unwrap_or(0),
                rdata: rr_data,
            })
        }

        fn parse_soa(
            &self,
            line: &mut Peekable<Chars>,
        ) -> Result<core::record::SOAResourceData, RDNSError> {
            self.chomp(line);
            let primary_name = self.parse_domain_name(line)?;
            self.chomp(line);
            let responsible_name = self.parse_domain_name(line)?;
            self.chomp(line);
            let serial: u32 = self.parse_number(line)?;
            self.chomp(line);
            let refresh: i32 = self.parse_number(line)?;
            self.chomp(line);
            let retry: i32 = self.parse_number(line)?;
            self.chomp(line);
            let expire: i32 = self.parse_number(line)?;
            self.chomp(line);
            let minimum: u32 = self.parse_number(line)?;

            return Ok(core::record::SOAResourceData {
                primary_name,
                responsible_name,
                serial,
                refresh,
                retry,
                expire,
                minimum,
            });
        }

        fn parse_domain_name(
            &self,
            line: &mut Peekable<Chars>,
        ) -> Result<core::name::Name, RDNSError> {
            let name = if let Some('@') = line.peek() {
                line.next();
                self.current_origin.clone()
            } else {
                core::name::Name::parse(line, HashSet::from([' ', '\t']))?
            };

            println!(
                "found name {}",
                <core::name::Name as Into<String>>::into(name.clone())
            );

            Ok(name)
        }

        fn maybe_parse_domain_name(
            &self,
            line: &mut Peekable<Chars>,
        ) -> Result<Option<core::name::Name>, RDNSError> {
            self.chomp(line);
            match line.peek() {
                Some(';') | None => Ok(None),
                Some(_) => self.parse_domain_name(line).map(|n| Some(n)),
            }
        }

        fn parse_file_name(&self, line: &mut Peekable<Chars>) -> Result<PathBuf, RDNSError> {
            if !self.chomp(line) {
                return Err(RDNSError::MasterFileFormatError(
                    "expected whitespace separating the file name".to_string(),
                ));
            }

            let mut path_builder = String::new();
            while let Some(&ch) = line.peek() {
                if !self.is_whitespace(ch) {
                    path_builder.push(line.next().unwrap())
                }
            }

            Ok(path_builder.into())
        }

        fn try_parse_ttl(&self, line: &mut Peekable<Chars>) -> Result<Option<i32>, RDNSError> {
            let first = line.peek();

            if let Some(&ch) = first {
                if ch.is_digit(10) {
                    return self.parse_number::<i32>(line).map(|ttl| Some(ttl));
                }
            }

            Ok(None)
        }

        fn parse_ip_addr(&self, line: &mut Peekable<Chars>) -> Result<Ipv4Addr, RDNSError> {
            let mut addr: u32 = 0;

            let mut part_number = 0;
            let mut part = String::new();

            loop {
                match line.peek() {
                    Some('0'..='9') => {
                        part.push(line.next().unwrap());
                    }
                    Some('.' | ' ' | '\t') | None => {
                        line.next();
                        println!("parse {}", part);
                        let parsed = part.parse::<u8>();
                        match parsed {
                            Ok(v) => {
                                addr |= (v as u32) << 8 * (3 - part_number);
                                part_number += 1;
                                part = String::new();
                            }
                            _ => {
                                return Err(RDNSError::MasterFileFormatError(
                                    "Invalid part of IP address".to_string(),
                                ))
                            }
                        }
                    }
                    _ => {
                        return Err(RDNSError::MasterFileFormatError(
                            "Invalid IP address format".to_string(),
                        ))
                    }
                }

                if part_number == 4 {
                    break;
                }
            }

            Ok(Ipv4Addr::from(addr))
        }

        fn parse_number<T: FromStr>(&self, line: &mut Peekable<Chars>) -> Result<T, RDNSError> {
            let mut str = String::new();
            while let Some(&ch) = line.peek() {
                if ch.is_digit(10) {
                    str.push(line.next().unwrap());
                } else {
                    break;
                }
            }

            str.parse::<T>()
                .map_err(|_| RDNSError::MasterFileFormatError("Invalid number".to_string()))
        }

        fn get_text(&self, line: &mut Peekable<Chars>) -> Result<String, RDNSError> {
            let mut str = String::new();
            while let Some(&ch) = line.peek() {
                if self.is_character(ch) {
                    str.push(line.next().unwrap());
                } else {
                    break;
                }
            }

            Ok(str)
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
            match ch {
                'a'..='z' | 'A'..='Z' => true,
                _ => false,
            }
        }

        fn is_whitespace(&self, ch: char) -> bool {
            ch == ' ' || ch == '\t'
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::txt_config::parser;
    use core::record::ResourceData;
    use std::borrow::Borrow;
    use std::collections::HashSet;
    use std::io::{BufRead, Cursor, Lines};

    #[test]
    fn parse_comment_on_own_line() {
        parser::TxtConfigParser::parse(
            &mut as_lines("; a comment".to_string()),
            core::name::Name::root(),
        )
        .unwrap();
    }

    #[test]
    fn parse_origin_control_directive() {
        parser::TxtConfigParser::parse(
            &mut as_lines("$ORIGIN example.com".to_string()),
            core::name::Name::root(),
        )
        .unwrap();
    }

    #[test]
    fn parse_origin_control_directive_with_comment() {
        parser::TxtConfigParser::parse(
            &mut as_lines("$ORIGIN example.com ; some information".to_string()),
            core::name::Name::root(),
        )
        .unwrap();
    }

    #[test]
    fn parse_origin_control_directive_with_comment_tabs() {
        parser::TxtConfigParser::parse(
            &mut as_lines("$ORIGIN\texample.com\t; some information".to_string()),
            core::name::Name::root(),
        )
        .unwrap();
    }

    #[test]
    fn parse_cname_rr() {
        let records = parser::TxtConfigParser::parse(
            &mut as_lines("exemplar.com. IN 300 CNAME example.com".to_string()),
            core::name::Name::root(),
        )
        .unwrap();

        assert_eq!(1, records.len());

        let first_record = records.get(0).unwrap().clone();
        assert_eq!(
            "exemplar.com.",
            <core::name::Name as Into<String>>::into(first_record.name.clone())
        );
        assert_eq!(core::RRType::CNAME, first_record.rr_type);
        assert_eq!(core::RRClass::IN, first_record.class);
        assert_eq!(300, first_record.ttl);
        assert_eq!(
            core::name::Name::parse(&mut "example.com".chars().peekable(), HashSet::new())
                .unwrap()
                .raw(),
            first_record.rdata.serialise()
        );
    }

    #[test]
    fn parse_alias_rr() {
        let records = parser::TxtConfigParser::parse(
            &mut as_lines("exemplar.com. IN 300 A 1.2.3.4".to_string()),
            core::name::Name::root(),
        )
        .unwrap();

        assert_eq!(1, records.len());

        let first_record = records.get(0).unwrap().clone();
        assert_eq!(
            "exemplar.com.",
            <core::name::Name as Into<String>>::into(first_record.name.clone())
        );
        assert_eq!(core::RRType::A, first_record.rr_type);
        assert_eq!(core::RRClass::IN, first_record.class);
        assert_eq!(300, first_record.ttl);
        assert_eq!(vec![1, 2, 3, 4], first_record.rdata.serialise());
    }

    #[test]
    fn use_at_symbol_in_place_of_owner_name() {
        let records = parser::TxtConfigParser::parse(
            &mut as_lines("$ORIGIN exemplar.com.\n@ IN 300 CNAME example.com".to_string()),
            core::name::Name::root(),
        )
        .unwrap();

        assert_eq!(1, records.len());

        let first_record = records.get(0).unwrap().clone();
        assert_eq!(
            "exemplar.com.",
            <core::name::Name as Into<String>>::into(first_record.name.clone())
        );
        assert_eq!(core::RRType::CNAME, first_record.rr_type);
        assert_eq!(core::RRClass::IN, first_record.class);
        assert_eq!(300, first_record.ttl);
        assert_eq!(
            core::name::Name::parse(&mut "example.com".chars().peekable(), HashSet::new())
                .unwrap()
                .raw(),
            first_record.rdata.serialise()
        );
    }

    #[test]
    fn parse_soa_rr_on_single_line() {
        let records = parser::TxtConfigParser::parse(
            &mut as_lines("@ IN SOA nameserver1 owner 20 7200 600 3600000 60".to_string()),
            core::name::Name::root(),
        )
        .unwrap();

        assert_eq!(1, records.len());

        let first_record = records.get(0).unwrap().clone();
        assert_eq!(
            ".",
            <core::name::Name as Into<String>>::into(first_record.name.clone())
        );
        assert_eq!(core::RRType::SOA, first_record.rr_type);
        assert_eq!(core::RRClass::IN, first_record.class);
        assert_eq!(0, first_record.ttl);
        assert_eq!(
            core::record::SOAResourceData {
                primary_name: core::name::Name::parse(
                    &mut "nameserver1".chars().peekable(),
                    HashSet::new()
                )
                .unwrap(),
                responsible_name: core::name::Name::parse(
                    &mut "owner".chars().peekable(),
                    HashSet::new()
                )
                .unwrap(),
                serial: 20,
                refresh: 7200,
                retry: 600,
                expire: 3600000,
                minimum: 60,
            }
            .serialise(),
            first_record.rdata.serialise()
        );
    }

    fn as_lines(input: String) -> Lines<Cursor<String>> {
        Cursor::new(input).lines()
    }
}
