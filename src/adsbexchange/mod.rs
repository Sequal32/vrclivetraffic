mod bincraft;
mod util;
pub use bincraft::*;
use log::warn;

use crate::error::Error;
use crate::util::{AircraftData, AircraftMap, AircraftProvider, Bounds};
use attohttpc::{body::Empty, PreparedRequest, Session};
use std::collections::{HashMap, HashSet};

const GLOBE_INDEX_GRID: f32 = 3.0;
const ENDPOINT: &str = "https://globe.adsbexchange.com/data";

fn get_global_index(lat: f32, lon: f32) -> u16 {
    let lat = GLOBE_INDEX_GRID * ((lat + 90.0) / GLOBE_INDEX_GRID).floor() - 90.0;
    let lon = GLOBE_INDEX_GRID * ((lon + 180.0) / GLOBE_INDEX_GRID).floor() - 180.0;

    let i = ((lat + 90.0) / GLOBE_INDEX_GRID).floor();
    let j = ((lon + 180.0) / GLOBE_INDEX_GRID).floor();

    let lat_mutliplier = (360.0 / GLOBE_INDEX_GRID + 1.0).floor();

    return (i * lat_mutliplier + j + 1000.0) as u16;
}

pub struct AdsbExchange {
    global_indexes: HashSet<u16>,
    session: Session,
}

impl AdsbExchange {
    pub fn new(radar_loc: &Bounds) -> Self {
        let mut global_indexes = HashSet::new();

        // https://github.com/wiedehopf/tar1090/blob/968e6578f24800eb3d92c90f71182a322b234121/html/script.js#L4225

        let mut lon = radar_loc.lon1;

        let x1 = radar_loc.lon1;
        let x2 = radar_loc.lon2;
        let _y1 = radar_loc.lat2;
        let y2 = radar_loc.lat1;

        let mut x3 = if x1 < x2 { x2 } else { 199.0 };

        while lon < x3 + GLOBE_INDEX_GRID {
            if x1 > x2 && lon > 180.0 {
                lon -= 360.0;
                x3 = x2;
            }

            if lon > x3 {
                lon = x3 + 0.01;
            }
            //
            let mut lat = radar_loc.lat2;

            while lat < y2 + GLOBE_INDEX_GRID {
                if lat > y2 {
                    lat = y2 + 0.01;
                }

                if lat > 90.0 {
                    break;
                }

                global_indexes.insert(get_global_index(lat, lon));

                lat += GLOBE_INDEX_GRID;
            }

            lon += GLOBE_INDEX_GRID;
        }

        // Setup HTTP session
        let mut session = attohttpc::Session::new();
        session.header("Accept", "*/*");
        session.header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:84.0) Gecko/20100101 Firefox/84.0",
        );
        session.header("Referer", "https://globe.adsbexchange.com");
        session.header("Host", "globe.adsbexchange.com");

        let mut adsb = Self {
            global_indexes,
            session,
        };

        adsb.fetch_cookie().ok();
        adsb.check_and_remove_bad_tiles().ok();

        adsb
    }

    fn check_and_remove_bad_tiles(&mut self) -> Result<(), Error> {
        let mut bad_tiles = Vec::new();

        for index in self.global_indexes.iter() {
            let response = self.session.head(self.get_url_for_index(index)).send()?;

            if response.status() != 200 {
                bad_tiles.push(*index);
            }
        }

        for bad_tile in bad_tiles {
            self.global_indexes.remove(&bad_tile);
        }

        Ok(())
    }

    fn fetch_cookie(&mut self) -> Result<(), Error> {
        let response = self.session.head(ENDPOINT).send()?;
        // We expect to get a Set-Cookie from this which will allow us to make more requests
        if let Some(cookie) = response.headers().get("Set-Cookie") {
            self.session.header("Cookie", cookie);
        }

        Ok(())
    }

    fn get_url_for_index(&self, index: &u16) -> String {
        format!("{}/globe_{}.binCraft", ENDPOINT, index)
    }

    fn get_request(&self, index: &u16) -> PreparedRequest<Empty> {
        self.session.get(self.get_url_for_index(index)).prepare()
    }
}

impl AircraftProvider for AdsbExchange {
    fn get_aircraft(&self) -> Result<AircraftMap, Error> {
        let mut return_data = HashMap::new();

        for index in self.global_indexes.iter() {
            let response = match self.get_request(index).send()?.error_for_status() {
                Ok(r) => r,
                Err(e) => {
                    warn!("Error fetching index {} from ADSBExchange: {}", index, e);
                    continue;
                }
            };

            let bytes = response.bytes()?;

            let parsed_data = BinCraftData::from_bytes(&bytes);

            for aircraft in parsed_data.aircraft {
                let ident = aircraft.hex.clone();
                return_data.insert(ident, aircraft.into());
            }
        }

        Ok(return_data)
    }

    fn get_name(&self) -> &str {
        "ADSBExchange"
    }
}

impl Into<AircraftData> for ADSBExData {
    fn into(self) -> AircraftData {
        AircraftData {
            squawk: self.squawk.unwrap_or_default(),
            callsign: self.flight.unwrap_or_default(),
            is_on_ground: self.airground == 1,
            latitude: self.lat.map(|x| x as f32).unwrap_or_default(),
            longitude: self.lon.map(|x| x as f32).unwrap_or_default(),
            heading: self.track.map(|x| x as u32).unwrap_or_default(),
            ground_speed: self.ias.map(|x| x as u32).unwrap_or_default(),
            timestamp: self.time as u64,
            altitude: self.alt_baro.map(|x| x as i32).unwrap_or_default(),
            model: self.aircraft_type,
            hex: self.hex,
            origin: String::new(),
            destination: String::new(),
            provider: "ADSBExchange",
        }
    }
}
