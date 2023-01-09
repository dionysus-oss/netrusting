use crate::error::RDNSError;
use crate::name::Name;

trait ResourceData: Sized {
    fn serialise(self) -> Vec<u8>;
}

struct RawResourceData<'a>(&'a [u8]);

impl<'a> RawResourceData<'a> {
    fn read(source: &'a [u8]) -> Result<Self, RDNSError> {
        Ok(RawResourceData(source))
    }
}

impl<'a> ResourceData for RawResourceData<'a> {
    fn serialise(self) -> Vec<u8> {
        self.0.to_owned()
    }
}

struct CNameResourceData(Name);

impl CNameResourceData {
    fn read(source: &[u8]) -> Result<Self, RDNSError> {
        let name_str = String::from_utf8(source.to_owned())?;
        let name = Name::try_from(name_str.as_ref())?;
        Ok(CNameResourceData(name))
    }
}

impl ResourceData for CNameResourceData {
    fn serialise(self) -> Vec<u8> {
        self.0.into()
    }
}

struct HInfoResourceData(String);

impl HInfoResourceData {
    fn read(source: &[u8]) -> Result<Self, RDNSError> {
        let name_str = String::from_utf8(source.to_owned())?;
        Ok(HInfoResourceData(name_str))
    }
}

impl ResourceData for HInfoResourceData {
    fn serialise(self) -> Vec<u8> {
        self.0.into_bytes()
    }
}

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
    fn serialise(self) -> Vec<u8> {
        let mut result = Vec::with_capacity(2 + self.exchange.len());
        result.push((self.preference >> 8) as u8);
        result.push(self.preference as u8);
        result.extend(<Name as Into<Vec<u8>>>::into(self.exchange).as_slice());

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
