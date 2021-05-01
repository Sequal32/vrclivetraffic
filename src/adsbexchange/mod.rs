mod bincraft;
mod util;
pub use bincraft::*;
use log::warn;
use rand::{distributions::Alphanumeric, Rng};

use crate::error::Error;
use crate::util::{AircraftData, AircraftMap, AircraftProvider, Bounds};
use attohttpc::{body::Empty, PreparedRequest, Session};
use cookie::Cookie;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::iter;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::time::SystemTime;

const GLOBE_INDEX_GRID: f32 = 3.0;
const ENDPOINT: &str = "https://globe.adsbexchange.com";

fn get_global_index(lat: f32, lon: f32) -> u16 {
    let lat = GLOBE_INDEX_GRID * ((lat + 90.0) / GLOBE_INDEX_GRID).floor() - 90.0;
    let lon = GLOBE_INDEX_GRID * ((lon + 180.0) / GLOBE_INDEX_GRID).floor() - 180.0;

    let i = ((lat + 90.0) / GLOBE_INDEX_GRID).floor();
    let j = ((lon + 180.0) / GLOBE_INDEX_GRID).floor();

    let lat_mutliplier = (360.0 / GLOBE_INDEX_GRID + 1.0).floor();

    return (i * lat_mutliplier + j + 1000.0) as u16;
}

#[derive(Serialize, Deserialize)]
struct GlobeRates {
    simload: usize,
}

pub struct AdsbExchange {
    global_indexes: HashSet<u16>,
    session: Session,
    last_fetched_index: AtomicUsize,
    requests_per_interval: usize,
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
        session.header("Referer", "https://globe.adsbexchange.com");
        session.header("Host", "globe.adsbexchange.com");

        let mut adsb = Self {
            global_indexes,
            session,
            requests_per_interval: 4,
            last_fetched_index: AtomicUsize::new(0),
        };

        adsb.fetch_cookie().ok();

        adsb
    }

    fn fetch_cookie(&mut self) -> Result<(), Error> {
        let response = self.session.head(ENDPOINT).send()?;
        // We expect to get a Set-Cookie from this which will allow us to make more requests
        let mut cookies: Vec<String> = response
            .headers()
            .get_all("Set-Cookie")
            .into_iter()
            .map(|x| {
                let cookie = Cookie::parse(x.to_str().unwrap()).unwrap();
                return format!("{}={}", cookie.name(), cookie.value());
            })
            .collect();

        // generate random adsbx_sid
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            + 2 * 86400 * 1000;

        let random_chars = iter::repeat(())
            .map(|_| rand::thread_rng().sample(Alphanumeric))
            .take(11)
            .map(|x| x as char)
            .collect::<String>();

        let sid = format!("adsbx_sid={}_{}", time, random_chars);
        cookies.push(sid);

        self.session.header("Cookie", cookies.join("; "));

        // Validate SID by sending request to globeRates.json
        if let Ok(globe_rates) = self
            .session
            .get(format!("{}/globeRates.json", ENDPOINT))
            .send()
            .and_then(|x| x.json::<GlobeRates>())
        {
            self.requests_per_interval = globe_rates.simload;
        }

        Ok(())
    }

    fn get_url_for_index(&self, index: &u16) -> String {
        format!("{}/data/globe_{}.binCraft", ENDPOINT, index)
    }

    fn get_request(&self, index: &u16) -> PreparedRequest<Empty> {
        self.session.get(self.get_url_for_index(index)).prepare()
    }
}

impl AircraftProvider for AdsbExchange {
    fn get_aircraft(&mut self) -> Result<AircraftMap, Error> {
        let mut return_data = HashMap::new();

        // No tiles to fetch
        if self.global_indexes.len() == 0 {
            return Ok(return_data);
        }

        let last_fetched = self.last_fetched_index.load(SeqCst);
        let mut fetched = 0;

        for index in self
            .global_indexes
            .iter()
            .skip(last_fetched)
            .take(self.requests_per_interval)
        {
            let response = match self.get_request(index).send()?.error_for_status() {
                Ok(r) => r,
                Err(e) => {
                    warn!("Error fetching index {} from ADSBExchange: {}", index, e);
                    break;
                }
            };

            let bytes = response.bytes()?;

            let parsed_data = BinCraftData::from_bytes(&bytes);

            for aircraft in parsed_data.aircraft {
                let ident = aircraft.hex.clone();
                return_data.insert(ident, aircraft.into());
            }

            fetched += 1;
        }

        self.last_fetched_index
            .store((last_fetched + fetched) % self.global_indexes.len(), SeqCst);

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
        }
    }
}
