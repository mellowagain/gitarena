use std::net::IpAddr;

use maxminddb::{Reader, geoip2};
use once_cell::sync::Lazy;

static MMDB_BYTES: &[u8] = include_bytes!("../data/GeoLite2-City.mmdb");

static READER: Lazy<Reader<&'static [u8]>> = Lazy::new(|| Reader::from_source(MMDB_BYTES).expect("Failed to parse GeoLite2-City.mmdb"));

pub(crate) fn lookup(ip: IpAddr) -> (Option<String>, Option<String>) {
    let Ok(lookup_result) = READER.lookup(ip) else {
        return (None, None);
    };

    let Ok(Some(city_record)) = lookup_result.decode::<geoip2::City<'_>>() else {
        return (None, None);
    };

    let city = city_record.city.names.english.map(str::to_owned);
    let country = city_record.country.names.english.map(str::to_owned);

    (city, country)
}
