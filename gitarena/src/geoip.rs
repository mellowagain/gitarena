use anyhow::Result;
use maxminddb::{Reader, geoip2};
use once_cell::sync::Lazy;
use std::net::IpAddr;

static MMDB_BYTES: &[u8] = include_bytes!("../data/GeoLite2-City.mmdb");

static READER: Lazy<Reader<&'static [u8]>> = Lazy::new(|| Reader::from_source(MMDB_BYTES).expect("Failed to parse GeoLite2-City.mmdb"));

pub(crate) fn lookup(ip: IpAddr) -> Result<(Option<String>, Option<String>)> {
    let result = READER.lookup(ip)?;

    let Some(record) = result.decode::<geoip2::City>()? else {
        return Ok((None, None));
    };

    let city = record.city.names.english.map(ToString::to_string);
    let country = record.country.names.english.map(ToString::to_string);

    Ok((city, country))
}
