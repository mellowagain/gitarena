use std::net::IpAddr;

use maxminddb::{Reader, geoip2};
use once_cell::sync::Lazy;

static MMDB_BYTES: &[u8] = include_bytes!("../data/GeoLite2-City.mmdb");

static READER: Lazy<Reader<&'static [u8]>> = Lazy::new(|| Reader::from_source(MMDB_BYTES).expect("Failed to parse GeoLite2-City.mmdb"));

pub(crate) fn lookup(ip: IpAddr) -> (Option<String>, Option<String>) {
    let city_record: geoip2::City<'_> = match READER.lookup(ip) {
        Ok(r) => r,
        Err(_) => return (None, None),
    };

    let city = city_record.city.and_then(|c| c.names).and_then(|n| n.get("en").copied()).map(str::to_owned);

    let country = city_record.country.and_then(|c| c.names).and_then(|n| n.get("en").copied()).map(str::to_owned);

    (city, country)
}
