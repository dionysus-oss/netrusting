use crate::name::Name;
use bytes::Bytes;

pub mod error;
pub mod name;
mod record;

#[cfg(test)]
mod test;

// A resource record (RR)
pub struct ResourceRecord {
    /// The owner name of this resource record
    pub name: Name,
    // The TYPE code
    pub rr_type: RRType<u16>,
    // The CLASS code
    pub class: RRClass<u16>,
    /// The time interval that the resource may be cached for. A zero value means the record should
    /// not be cached.
    pub ttl: i32,
    /// The length of the RDATA field
    pub rd_length: u16,
    // The value of the resource record
    pub rdata: Bytes,
}

/// Resource record TYPE
#[derive(Debug, PartialEq)]
pub enum RRType<T> {
    /// Alias, a host address
    A,
    /// Name Server, an authoritative name server
    NS,
    /// Mail Destination, OBSOLETE use MX
    MD,
    /// Mail Forwarder, OBSOLETE use MX
    MF,
    /// Canonical Name, the canonical name for an alias
    CNAME,
    /// Start Of Authority, marks the start of a zone of authority
    SOA,
    /// Mailbox, a mailbox domain name _EXPERIMENTAL_
    MB,
    /// Mail Group, a mail group member _EXPERIMENTAL_
    MG,
    /// Mail Rename, a mail rename domain name _EXPERIMENTAL_
    MR,
    /// Null, a null resource record _EXPERIMENTAL_
    NULL,
    /// Well Known Service, a well known service description
    WKS,
    // Pointer, a domain name pointer
    PTR,
    /// Host Information
    HINFO,
    /// Mailbox Information or Mail list Information
    MINFO,
    /// Mail Exchange
    MX,
    /// Text, text strings
    TXT,
    /// A TYPE which is not known by this implementation
    UNKNOWN(T),
}

impl RRType<u16> {
    /// The two octet resource record TYPE code
    pub fn value(&self) -> u16 {
        match self {
            RRType::A => 1,
            RRType::NS => 2,
            RRType::MD => 3,
            RRType::MF => 4,
            RRType::CNAME => 5,
            RRType::SOA => 6,
            RRType::MB => 7,
            RRType::MG => 8,
            RRType::MR => 9,
            RRType::NULL => 10,
            RRType::WKS => 11,
            RRType::PTR => 12,
            RRType::HINFO => 13,
            RRType::MINFO => 14,
            RRType::MX => 15,
            RRType::TXT => 16,
            RRType::UNKNOWN(v) => *v,
        }
    }

    /// The enum value for the provided two octet TYPE code
    pub fn from_value(value: u16) -> Self {
        match value {
            1 => RRType::A,
            2 => RRType::NS,
            3 => RRType::MD,
            4 => RRType::MF,
            5 => RRType::CNAME,
            6 => RRType::SOA,
            7 => RRType::MB,
            8 => RRType::MG,
            9 => RRType::MR,
            10 => RRType::NULL,
            11 => RRType::WKS,
            12 => RRType::PTR,
            13 => RRType::HINFO,
            14 => RRType::MINFO,
            15 => RRType::MX,
            16 => RRType::TXT,
            v => RRType::UNKNOWN(v),
        }
    }
}

/// Resource record CLASS
#[derive(Debug, PartialEq)]
pub enum RRClass<T> {
    /// Internet, the internet
    IN,
    /// CSNET, OBSOLETE
    CS,
    /// CHAOS, the CHAOS class
    CH,
    /// Hesiod, the Hesiod name service
    HS,
    /// A CLASS which is not known by this implementation
    UNKNOWN(T),
}

impl RRClass<u16> {
    /// The two octet resource record CLASS code
    pub fn value(&self) -> u16 {
        match self {
            RRClass::IN => 1,
            RRClass::CS => 2,
            RRClass::CH => 3,
            RRClass::HS => 4,
            RRClass::UNKNOWN(v) => *v,
        }
    }

    pub fn from_value(value: u16) -> Self {
        match value {
            1 => RRClass::IN,
            2 => RRClass::CS,
            3 => RRClass::CH,
            4 => RRClass::HS,
            v => RRClass::UNKNOWN(v),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{RRClass, RRType};

    #[test]
    fn rr_type_round_trip() {
        assert_round_trip_for_rr_type(RRType::A);
        assert_round_trip_for_rr_type(RRType::NS);
        assert_round_trip_for_rr_type(RRType::MD);
        assert_round_trip_for_rr_type(RRType::MF);
        assert_round_trip_for_rr_type(RRType::CNAME);
        assert_round_trip_for_rr_type(RRType::SOA);
        assert_round_trip_for_rr_type(RRType::MB);
        assert_round_trip_for_rr_type(RRType::MG);
        assert_round_trip_for_rr_type(RRType::MR);
        assert_round_trip_for_rr_type(RRType::NULL);
        assert_round_trip_for_rr_type(RRType::WKS);
        assert_round_trip_for_rr_type(RRType::PTR);
        assert_round_trip_for_rr_type(RRType::HINFO);
        assert_round_trip_for_rr_type(RRType::MINFO);
        assert_round_trip_for_rr_type(RRType::MX);
        assert_round_trip_for_rr_type(RRType::TXT);
        assert_round_trip_for_rr_type(RRType::UNKNOWN(100));
    }

    fn assert_round_trip_for_rr_type(rr_type: RRType<u16>) {
        assert_eq!(rr_type, RRType::from_value(rr_type.value()));
    }

    #[test]
    fn rr_class_round_trip() {
        assert_round_trip_for_rr_class(RRClass::IN);
        assert_round_trip_for_rr_class(RRClass::CS);
        assert_round_trip_for_rr_class(RRClass::CH);
        assert_round_trip_for_rr_class(RRClass::HS);
        assert_round_trip_for_rr_class(RRClass::UNKNOWN(100));
    }

    fn assert_round_trip_for_rr_class(rr_class: RRClass<u16>) {
        assert_eq!(rr_class, RRClass::from_value(rr_class.value()));
    }
}
