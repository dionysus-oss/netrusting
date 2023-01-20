use rdns_core::error::{LineCharPos, RDNSError};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, Lines, Read};
use std::iter::Peekable;
use std::path::Path;
use std::vec::IntoIter;

pub fn load_txt_config<P>(path: P) -> Result<Vec<rdns_core::ResourceRecord>, RDNSError>
where
    P: AsRef<Path>,
{
    let mut lines = read_lines(path)?;

    let records = parser::TxtConfigParser::parse(&mut lines, rdns_core::name::Name::root())?;

    Ok(records)
}

fn read_lines<P>(path: P) -> io::Result<Lines<BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(path)?;
    Ok(BufReader::new(file).lines())
}

struct ParserReader<'a, R: Read + BufRead> {
    lines: &'a mut Lines<R>,
    line: Peekable<IntoIter<u8>>,
    line_num: u32,
    char_num: u32,
}

impl<'a, R: Read + BufRead> ParserReader<'a, R> {
    fn new(lines: &'a mut Lines<R>) -> Self {
        let line = if let Some(Ok(line)) = lines.next() {
            line
        } else {
            "".to_string()
        };

        ParserReader {
            lines,
            line: line.into_bytes().into_iter().peekable(),
            line_num: 1,
            char_num: 1,
        }
    }

    fn peek_char(&mut self) -> Option<&u8> {
        self.line.peek()
    }

    fn next_char(&mut self) -> Option<u8> {
        self.char_num += 1;
        self.line.next()
    }

    fn borrow(&mut self) -> &mut Peekable<IntoIter<u8>> {
        &mut self.line
    }

    fn next_line(&mut self) -> bool {
        let line = if let Some(Ok(line)) = self.lines.next() {
            line
        } else {
            return false;
        };

        self.line_num += 1;
        self.char_num = 1;
        self.line = line.into_bytes().into_iter().peekable();

        true
    }

    fn current_position(&self) -> LineCharPos {
        LineCharPos {
            line: self.line_num,
            char: self.char_num,
        }
    }
}

mod parser {
    use crate::txt_config::{read_lines, ParserReader};
    use rdns_core::error::RDNSError;
    use std::collections::HashSet;
    use std::io::{BufRead, Lines, Read};
    use std::net::Ipv4Addr;
    use std::path::PathBuf;
    use std::rc::Rc;
    use std::str::FromStr;

    pub struct TxtConfigParser<'a, R: Read + BufRead> {
        state: ParserReader<'a, R>,
        current_origin: rdns_core::name::Name,
        current_name: Option<rdns_core::name::Name>,
        current_class: rdns_core::RRClass<u16>,
        multiline: bool,
    }

    impl<'a, R: Read + BufRead> TxtConfigParser<'a, R> {
        fn new(lines: &'a mut Lines<R>, origin: rdns_core::name::Name) -> Self {
            TxtConfigParser {
                state: ParserReader::new(lines),
                current_origin: origin,
                current_name: None,
                current_class: rdns_core::RRClass::UNKNOWN(0),
                multiline: false,
            }
        }

        pub fn parse(
            lines: &'a mut Lines<R>,
            origin: rdns_core::name::Name,
        ) -> Result<Vec<rdns_core::ResourceRecord>, RDNSError> {
            let mut parser = TxtConfigParser::new(lines, origin);

            let mut records = Vec::new();
            let mut start_of_line: bool;

            println!("origin is {}", parser.current_origin);

            'lines: loop {
                start_of_line = true;

                while let Some(ch) = parser.state.peek_char() {
                    match ch {
                        b' ' | b'\t' => {
                            parser.state.next_char();
                            start_of_line = false;
                        }
                        b';' => break,
                        b'$' => parser.parse_control_entry()?,
                        _ => {
                            let rr = if start_of_line {
                                parser.parse_name_and_rr()?
                            } else {
                                parser.parse_rr()?
                            };
                            records.push(rr);
                            break;
                        }
                    };
                }

                if !parser.state.next_line() {
                    break 'lines;
                }
            }

            println!(
                "final root is {}",
                <rdns_core::name::Name as Into<String>>::into(parser.current_origin.clone())
            );

            Ok(records)
        }

        fn parse_control_entry(&mut self) -> Result<(), RDNSError> {
            self.state.next_char();

            let mut control_name = String::new();
            while let Some(&ch) = self.state.peek_char() {
                if ch == b' ' || ch == b'\t' {
                    break;
                } else if self.is_character(ch) {
                    control_name.push(self.state.next_char().unwrap() as char);
                } else {
                    return Err(RDNSError::MasterFileFormatError(
                        "a control directive should only contain letters".to_string(),
                        self.state.current_position(),
                    ));
                }
            }

            match control_name.as_str() {
                "ORIGIN" => {
                    self.chomp();
                    let name = self.parse_domain_name()?;
                    self.current_origin = name;
                }
                "INCLUDE" => {
                    let file_name = self.parse_file_name()?;
                    let domain_name = self
                        .maybe_parse_domain_name()?
                        .unwrap_or(self.current_origin.clone());

                    let mut sub_lines = read_lines(file_name)?;

                    // TODO capture result and push to current RRs
                    TxtConfigParser::parse(&mut sub_lines, domain_name)?;
                }
                _ => {
                    return Err(RDNSError::MasterFileFormatError(
                        format!("unknown control directive {}", control_name),
                        self.state.current_position(),
                    ));
                }
            };

            Ok(())
        }

        fn parse_name_and_rr(&mut self) -> Result<rdns_core::ResourceRecord, RDNSError> {
            self.current_name = Some(self.parse_domain_name()?);

            self.chomp();
            self.parse_rr()
        }

        fn parse_rr(&mut self) -> Result<rdns_core::ResourceRecord, RDNSError> {
            let mut ttl_opt = self.try_parse_ttl()?;
            self.chomp();

            let text = self.get_text()?;
            let mut class: rdns_core::RRClass<u16> = text.as_str().try_into().unwrap();
            self.chomp();

            let mut rr_type: Option<rdns_core::RRType<u16>> =
                if class == rdns_core::RRClass::UNKNOWN(0) {
                    Some(text.as_str().try_into().unwrap())
                } else {
                    None
                };

            // TODO constant for unknown
            if rr_type == None || rr_type == Some(rdns_core::RRType::UNKNOWN(0)) {
                if ttl_opt == None {
                    ttl_opt = self.try_parse_ttl()?;
                    self.chomp();
                }

                let text = self.get_text()?;
                rr_type = Some(text.as_str().try_into().unwrap());
                self.chomp();
            }

            if class == rdns_core::RRClass::UNKNOWN(0) {
                if self.current_class != rdns_core::RRClass::UNKNOWN(0) {
                    class = self.current_class.clone();
                } else {
                    return Err(RDNSError::MasterFileFormatError(
                        "No class".to_string(),
                        self.state.current_position(),
                    ));
                }
            } else if self.current_class == rdns_core::RRClass::UNKNOWN(0) {
                self.current_class = class.clone();
            }

            if class != self.current_class {
                // TODO propagate to included files?
                return Err(RDNSError::MasterFileFormatError(
                    "File must only contain one class".to_string(),
                    self.state.current_position(),
                ));
            }

            let rr_data: Rc<dyn rdns_core::record::ResourceData> = match rr_type {
                Some(rdns_core::RRType::A) => {
                    let ip_address = self.parse_ip_addr()?;
                    Rc::new(rdns_core::record::AliasResourceData(ip_address))
                }
                Some(rdns_core::RRType::NS) => {
                    let name = self.parse_domain_name()?;
                    Rc::new(rdns_core::record::NameServerResourceData(name))
                }
                Some(rdns_core::RRType::CNAME) => {
                    let name = self.parse_domain_name()?;
                    Rc::new(rdns_core::record::CNameResourceData(name))
                }
                Some(rdns_core::RRType::SOA) => Rc::new(self.parse_soa()?),
                Some(rr_type) => {
                    return Err(RDNSError::MasterFileFormatError(
                        format!("unknown resource record type '{:?}'", rr_type),
                        self.state.current_position(),
                    ));
                }
                None => {
                    return Err(RDNSError::MasterFileFormatError(
                        format!("missing resource record type"),
                        self.state.current_position(),
                    ));
                }
            };

            Ok(rdns_core::ResourceRecord {
                name: self.current_name.as_ref().unwrap().clone(),
                rr_type: rr_type.unwrap(),
                class,
                ttl: ttl_opt.unwrap_or(0),
                rdata: rr_data,
            })
        }

        fn parse_soa(&mut self) -> Result<rdns_core::record::SOAResourceData, RDNSError> {
            self.parse_common_in_rr()?;
            let primary_name = self.parse_domain_name()?;
            self.parse_common_in_rr()?;
            let responsible_name = self.parse_domain_name()?;
            self.parse_common_in_rr()?;
            let serial: u32 = self.parse_number()?;
            self.parse_common_in_rr()?;
            let refresh: i32 = self.parse_number()?;
            self.parse_common_in_rr()?;
            let retry: i32 = self.parse_number()?;
            self.parse_common_in_rr()?;
            let expire: i32 = self.parse_number()?;
            self.parse_common_in_rr()?;
            let minimum: u32 = self.parse_number()?;

            return Ok(rdns_core::record::SOAResourceData {
                primary_name,
                responsible_name,
                serial,
                refresh,
                retry,
                expire,
                minimum,
            });
        }

        fn parse_domain_name(&mut self) -> Result<rdns_core::name::Name, RDNSError> {
            let name = if let Some(b'@') = self.state.peek_char() {
                self.state.next_char();
                self.current_origin.clone()
            } else {
                let result = rdns_core::name::Name::parse(
                    &mut self.state.borrow(),
                    HashSet::from([b' ', b'\t']),
                );
                match result {
                    Ok(name) => name,
                    Err(e) => {
                        return Err(RDNSError::MasterFileFormatError(
                            e.to_string(),
                            self.state.current_position(),
                        ))
                    }
                }
            };

            println!(
                "found name {}",
                <rdns_core::name::Name as Into<String>>::into(name.clone())
            );

            Ok(name)
        }

        fn maybe_parse_domain_name(&mut self) -> Result<Option<rdns_core::name::Name>, RDNSError> {
            self.chomp();
            match self.state.peek_char() {
                Some(b';') | None => Ok(None),
                Some(_) => self.parse_domain_name().map(|n| Some(n)),
            }
        }

        fn parse_file_name(&mut self) -> Result<PathBuf, RDNSError> {
            if !self.chomp() {
                return Err(RDNSError::MasterFileFormatError(
                    "expected whitespace separating the file name".to_string(),
                    self.state.current_position(),
                ));
            }

            let mut path_builder = String::new();
            while let Some(&ch) = self.state.peek_char() {
                if !self.is_whitespace(ch) {
                    path_builder.push(self.state.next_char().unwrap() as char)
                }
            }

            Ok(path_builder.into())
        }

        fn try_parse_ttl(&mut self) -> Result<Option<i32>, RDNSError> {
            let first = self.state.peek_char();

            if let Some(&ch) = first {
                if ch.is_ascii_digit() {
                    return self.parse_number::<i32>().map(|ttl| Some(ttl));
                }
            }

            Ok(None)
        }

        fn parse_ip_addr(&mut self) -> Result<Ipv4Addr, RDNSError> {
            let mut addr: u32 = 0;

            let mut part_number = 0;
            let mut part = String::new();

            loop {
                match self.state.peek_char() {
                    Some(b'0'..=b'9') => {
                        part.push(self.state.next_char().unwrap() as char);
                    }
                    Some(b'.' | b' ' | b'\t') | None => {
                        self.state.next_char();
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
                                    self.state.current_position(),
                                ));
                            }
                        }
                    }
                    _ => {
                        return Err(RDNSError::MasterFileFormatError(
                            "Invalid IP address format".to_string(),
                            self.state.current_position(),
                        ));
                    }
                }

                if part_number == 4 {
                    break;
                }
            }

            Ok(Ipv4Addr::from(addr))
        }

        fn parse_common_in_rr(&mut self) -> Result<(), RDNSError> {
            loop {
                self.chomp();
                match self.state.peek_char() {
                    Some(b'(') => {
                        if self.multiline {
                            return Err(RDNSError::MasterFileFormatError(
                                "Cannot nest multi-line blocks".to_string(),
                                self.state.current_position(),
                            ));
                        }

                        self.multiline = true;
                        self.state.next_char();
                    }
                    Some(b')') => {
                        if !self.multiline {
                            return Err(RDNSError::MasterFileFormatError(
                                "Not in a multi-line block".to_string(),
                                self.state.current_position(),
                            ));
                        }

                        self.multiline = false;
                        self.state.next_char();
                        break;
                    }
                    Some(b';') | None => {
                        self.state.next_line();
                        self.chomp();
                        break;
                    }
                    Some(_) => break,
                }
            }

            Ok(())
        }

        fn parse_number<T: FromStr>(&mut self) -> Result<T, RDNSError> {
            let mut str = String::new();
            while let Some(&ch) = self.state.peek_char() {
                if ch.is_ascii_digit() {
                    str.push(self.state.next_char().unwrap() as char);
                } else {
                    break;
                }
            }

            println!("parse number {}", str);

            str.parse::<T>().map_err(|_| {
                RDNSError::MasterFileFormatError(
                    "Invalid number".to_string(),
                    self.state.current_position(),
                )
            })
        }

        fn get_text(&mut self) -> Result<String, RDNSError> {
            let mut str = String::new();
            while let Some(&ch) = self.state.peek_char() {
                if self.is_character(ch) {
                    str.push(self.state.next_char().unwrap() as char);
                } else {
                    break;
                }
            }

            Ok(str)
        }

        fn chomp(&mut self) -> bool {
            let mut any_taken = false;
            while let Some(b' ' | b'\t') = self.state.peek_char() {
                self.state.next_char();
                any_taken = true;
            }

            any_taken
        }

        fn is_character(&self, ch: u8) -> bool {
            match ch {
                b'a'..=b'z' | b'A'..=b'Z' => true,
                _ => false,
            }
        }

        fn is_whitespace(&self, ch: u8) -> bool {
            match ch {
                b' ' | b'\t' => true,
                _ => false,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::txt_config::parser;
    use rdns_core::record::ResourceData;
    use std::collections::HashSet;
    use std::io::{BufRead, Cursor, Lines};

    #[test]
    fn parse_comment_on_own_line() {
        parser::TxtConfigParser::parse(
            &mut as_lines("; a comment".to_string()),
            rdns_core::name::Name::root(),
        )
        .unwrap();
    }

    #[test]
    fn parse_origin_control_directive() {
        parser::TxtConfigParser::parse(
            &mut as_lines("$ORIGIN example.com".to_string()),
            rdns_core::name::Name::root(),
        )
        .unwrap();
    }

    #[test]
    fn parse_origin_control_directive_with_comment() {
        parser::TxtConfigParser::parse(
            &mut as_lines("$ORIGIN example.com ; some information".to_string()),
            rdns_core::name::Name::root(),
        )
        .unwrap();
    }

    #[test]
    fn parse_origin_control_directive_with_comment_tabs() {
        parser::TxtConfigParser::parse(
            &mut as_lines("$ORIGIN\texample.com\t; some information".to_string()),
            rdns_core::name::Name::root(),
        )
        .unwrap();
    }

    #[test]
    fn parse_cname_rr() {
        let records = parser::TxtConfigParser::parse(
            &mut as_lines("exemplar.com. IN 300 CNAME example.com".to_string()),
            rdns_core::name::Name::root(),
        )
        .unwrap();

        assert_eq!(1, records.len());

        let first_record = records.get(0).unwrap().clone();
        assert_eq!(
            "exemplar.com.",
            <rdns_core::name::Name as Into<String>>::into(first_record.name.clone())
        );
        assert_eq!(rdns_core::RRType::CNAME, first_record.rr_type);
        assert_eq!(rdns_core::RRClass::IN, first_record.class);
        assert_eq!(300, first_record.ttl);
        assert_eq!(
            rdns_core::name::Name::parse(
                &mut "example.com"
                    .to_string()
                    .into_bytes()
                    .into_iter()
                    .peekable(),
                HashSet::new(),
            )
            .unwrap()
            .raw(),
            first_record.rdata.serialise()
        );
    }

    #[test]
    fn parse_alias_rr() {
        let records = parser::TxtConfigParser::parse(
            &mut as_lines("exemplar.com. IN 300 A 1.2.3.4".to_string()),
            rdns_core::name::Name::root(),
        )
        .unwrap();

        assert_eq!(1, records.len());

        let first_record = records.get(0).unwrap().clone();
        assert_eq!(
            "exemplar.com.",
            <rdns_core::name::Name as Into<String>>::into(first_record.name.clone())
        );
        assert_eq!(rdns_core::RRType::A, first_record.rr_type);
        assert_eq!(rdns_core::RRClass::IN, first_record.class);
        assert_eq!(300, first_record.ttl);
        assert_eq!(vec![1, 2, 3, 4], first_record.rdata.serialise());
    }

    #[test]
    fn parse_name_server_rr() {
        let records = parser::TxtConfigParser::parse(
            &mut as_lines("exemplar.com. IN NS hosting".to_string()),
            rdns_core::name::Name::root(),
        )
        .unwrap();

        assert_eq!(1, records.len());

        let first_record = records.get(0).unwrap().clone();
        assert_eq!(
            "exemplar.com.",
            <rdns_core::name::Name as Into<String>>::into(first_record.name.clone())
        );
        assert_eq!(rdns_core::RRType::NS, first_record.rr_type);
        assert_eq!(rdns_core::RRClass::IN, first_record.class);
        assert_eq!(0, first_record.ttl);
        assert_eq!(
            rdns_core::name::Name::parse(
                &mut "hosting".to_string().into_bytes().into_iter().peekable(),
                HashSet::new(),
            )
            .unwrap()
            .raw(),
            first_record.rdata.serialise()
        );
    }

    #[test]
    fn use_at_symbol_in_place_of_owner_name() {
        let records = parser::TxtConfigParser::parse(
            &mut as_lines("$ORIGIN exemplar.com.\n@ IN 300 CNAME example.com".to_string()),
            rdns_core::name::Name::root(),
        )
        .unwrap();

        assert_eq!(1, records.len());

        let first_record = records.get(0).unwrap().clone();
        assert_eq!(
            "exemplar.com.",
            <rdns_core::name::Name as Into<String>>::into(first_record.name.clone())
        );
        assert_eq!(rdns_core::RRType::CNAME, first_record.rr_type);
        assert_eq!(rdns_core::RRClass::IN, first_record.class);
        assert_eq!(300, first_record.ttl);
        assert_eq!(
            rdns_core::name::Name::parse(
                &mut "example.com"
                    .to_string()
                    .into_bytes()
                    .into_iter()
                    .peekable(),
                HashSet::new(),
            )
            .unwrap()
            .raw(),
            first_record.rdata.serialise()
        );
    }

    #[test]
    fn parse_soa_rr_on_single_line() {
        let records = parser::TxtConfigParser::parse(
            &mut as_lines("@ IN SOA nameserver1 owner 20 7200 600 3600000 60".to_string()),
            rdns_core::name::Name::root(),
        )
        .unwrap();

        assert_eq!(1, records.len());

        let first_record = records.get(0).unwrap().clone();
        assert_eq!(
            ".",
            <rdns_core::name::Name as Into<String>>::into(first_record.name.clone())
        );
        assert_eq!(rdns_core::RRType::SOA, first_record.rr_type);
        assert_eq!(rdns_core::RRClass::IN, first_record.class);
        assert_eq!(0, first_record.ttl);
        assert_eq!(
            rdns_core::record::SOAResourceData {
                primary_name: rdns_core::name::Name::parse(
                    &mut "nameserver1"
                        .to_string()
                        .into_bytes()
                        .into_iter()
                        .peekable(),
                    HashSet::new(),
                )
                .unwrap(),
                responsible_name: rdns_core::name::Name::parse(
                    &mut "owner".to_string().into_bytes().into_iter().peekable(),
                    HashSet::new(),
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

    #[test]
    fn parse_soa_rr_on_multiple_lines() {
        let records = parser::TxtConfigParser::parse(
            &mut as_lines(
                "@ IN SOA nameserver1 owner (20\n 7200\n 600\n 3600000\n 60)".to_string(),
            ),
            rdns_core::name::Name::root(),
        )
        .unwrap();

        assert_eq!(1, records.len());

        let first_record = records.get(0).unwrap().clone();
        assert_eq!(
            ".",
            <rdns_core::name::Name as Into<String>>::into(first_record.name.clone())
        );
        assert_eq!(rdns_core::RRType::SOA, first_record.rr_type);
        assert_eq!(rdns_core::RRClass::IN, first_record.class);
        assert_eq!(0, first_record.ttl);
        assert_eq!(
            rdns_core::record::SOAResourceData {
                primary_name: rdns_core::name::Name::parse(
                    &mut "nameserver1"
                        .to_string()
                        .into_bytes()
                        .into_iter()
                        .peekable(),
                    HashSet::new(),
                )
                .unwrap(),
                responsible_name: rdns_core::name::Name::parse(
                    &mut "owner".to_string().into_bytes().into_iter().peekable(),
                    HashSet::new(),
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
