use crate::error::RDNSError;
use crate::name::Name;
use std::fmt::Debug;
use std::net::Ipv4Addr;

pub trait ResourceData: Debug {
    fn serialise(&self) -> Vec<u8>;
}

#[derive(Debug)]
struct RawResourceData<'a>(&'a [u8]);

impl<'a> RawResourceData<'a> {
    fn read(source: &'a [u8]) -> Result<Self, RDNSError> {
        Ok(RawResourceData(source))
    }
}

impl<'a> ResourceData for RawResourceData<'a> {
    fn serialise(&self) -> Vec<u8> {
        self.0.to_owned()
    }
}

#[derive(Debug)]
pub struct AliasResourceData(pub Ipv4Addr);

impl AliasResourceData {
    fn read(source: &[u8]) -> Result<Self, RDNSError> {
        Ok(AliasResourceData(Ipv4Addr::new(
            source[0], source[1], source[2], source[3],
        )))
    }
}

impl ResourceData for AliasResourceData {
    fn serialise(&self) -> Vec<u8> {
        self.0.octets().to_vec()
    }
}

#[derive(Debug)]
pub struct CNameResourceData(pub Name);

impl CNameResourceData {
    fn read(source: &[u8]) -> Result<Self, RDNSError> {
        let name_str = String::from_utf8(source.to_owned())?;
        let name = Name::try_from(name_str.as_ref())?;
        Ok(CNameResourceData(name))
    }
}

impl ResourceData for CNameResourceData {
    fn serialise(&self) -> Vec<u8> {
        self.0.clone().into()
    }
}

#[derive(Debug)]
pub struct SOAResourceData {
    /// The name of the primary name server hosting the zone described by this SOA. Known as
    /// the MNAME in RFC 1035
    pub primary_name: Name,
    /// The mailbox of the person responsible for this zone. Known as RNAME in RFC 1025.
    pub responsible_name: Name,
    /// The version number of the original copy of the zone
    pub serial: u32,
    /// Time interval before the zone should be refreshed
    pub refresh: i32,
    /// Time interval before a failed refresh should be retried
    pub retry: i32,
    /// Time interval that specifies an upper limit for the zone remaining authoritative
    pub expire: i32,
    /// The minimum TTL for RRs in this zone
    pub minimum: u32,
}

impl ResourceData for SOAResourceData {
    fn serialise(&self) -> Vec<u8> {
        let mut result =
            Vec::with_capacity(self.primary_name.len() + self.responsible_name.len() + 20);
        result.append(&mut self.primary_name.clone().into());
        result.append(&mut self.responsible_name.clone().into());
        result.extend_from_slice(&self.serial.to_be_bytes());
        result.extend_from_slice(&self.refresh.to_be_bytes());
        result.extend_from_slice(&self.retry.to_be_bytes());
        result.extend_from_slice(&self.expire.to_be_bytes());
        result.extend_from_slice(&self.minimum.to_be_bytes());

        result
    }
}

#[derive(Debug)]
struct HInfoResourceData(String);

impl HInfoResourceData {
    fn read(source: &[u8]) -> Result<Self, RDNSError> {
        let name_str = String::from_utf8(source.to_owned())?;
        Ok(HInfoResourceData(name_str))
    }
}

impl ResourceData for HInfoResourceData {
    fn serialise(&self) -> Vec<u8> {
        self.0.clone().into_bytes()
    }
}

#[derive(Debug)]
struct MailExchangeResourceData {
    preference: u16,
    exchange: Name,
}

impl MailExchangeResourceData {
    fn read(priority: u16, exchange: &[u8]) -> Result<Self, RDNSError> {
        Ok(MailExchangeResourceData {
            preference: priority,
            exchange: Name::try_from(String::from_utf8(exchange.to_owned())?.as_str())?,
        })
    }
}

impl ResourceData for MailExchangeResourceData {
    fn serialise(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(2 + self.exchange.len());
        result.push((self.preference >> 8) as u8);
        result.push(self.preference as u8);
        result.extend(<Name as Into<Vec<u8>>>::into(self.exchange.clone()).as_slice());

        result
    }
}

#[cfg(test)]
mod tests {
    use crate::record::{
        CNameResourceData, HInfoResourceData, MailExchangeResourceData, RawResourceData,
        ResourceData,
    };
    use crate::test;

    #[test]
    fn round_trip_raw_record() {
        let input = "example.com".as_bytes();
        let raw = RawResourceData::read(input).unwrap();
        assert_eq!(input, raw.serialise());
    }

    #[test]
    fn round_trip_cname() {
        let input = "example.com";
        let cname = CNameResourceData::read(input.as_bytes()).unwrap();
        assert_eq!(test::dirty_to_bytes(input), cname.serialise());
    }

    #[test]
    fn round_trip_hinfo() {
        let input = "\"INTEL-386\" / \"WIN32\"".as_bytes();
        let hinfo = HInfoResourceData::read(input).unwrap();
        assert_eq!(input, hinfo.serialise());
    }

    #[test]
    fn round_trip_mail_exchange() {
        let input = "mail.example.com";
        let mx = MailExchangeResourceData::read(10, input.as_bytes()).unwrap();

        let mut expected = Vec::new();
        expected.push(0u8);
        expected.push(10);
        expected.extend(test::dirty_to_bytes(input));
        assert_eq!(expected, mx.serialise());
    }
}
