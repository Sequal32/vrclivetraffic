use crate::airports::Airports;
use crate::util::{AircraftData, AircraftMap, AircraftProvider, Bounds, ProvidedAircraftData};
use crate::{error::Error, util::FromProvider};

use attohttpc;
use serde::Deserialize;
use serde_json::{self, Value};
use std::{collections::HashMap, rc::Rc};

const ENDPOINT: &str = "https://data-live.flightradar24.com/zones/fcgi/feed.js?faa=1&mlat=1&flarm=1&adsb=1&gnd=1&air=1&vehicles=1&estimated=1&gliders=1&stats=0&maxage=14400";

pub struct FlightRadar {
    base_url: String,
    airports: Rc<Airports>,
}

impl FlightRadar {
    pub fn new(radar_loc: &Bounds, airports: Rc<Airports>) -> Self {
        Self {
            base_url: format!(
                "{}&bounds={:.2},{:.2},{:.2},{:.2}",
                ENDPOINT, radar_loc.lat1, radar_loc.lat2, radar_loc.lon1, radar_loc.lon2
            ),
            airports,
        }
    }
}

impl AircraftProvider for FlightRadar {
    fn get_aircraft(&mut self) -> Result<AircraftMap, Error> {
        let response = attohttpc::get(&self.base_url).send()?.error_for_status()?;

        let mut return_data = HashMap::new();

        let data: Value = serde_json::from_str(&response.text()?)?;

        // Iterate through aircraft
        for (_, value) in data.as_object().unwrap() {
            // Skip over stats data like numbers and objects
            if !value.is_array() {
                continue;
            }

            let mut data: FRData = serde_json::from_value(value.clone()).unwrap();

            data.origin = self
                .airports
                .get_icao_from_iata(&data.origin)
                .map_or(String::new(), |x| x.clone());
            data.destination = self
                .airports
                .get_icao_from_iata(&data.destination)
                .map_or(String::new(), |x| x.clone());

            return_data.insert(
                data.mode_s_code.clone(),
                Box::new(data) as Box<dyn ProvidedAircraftData>,
            );
        }

        Ok(return_data)
    }

    fn get_name(&self) -> &str {
        "FlightRadar24"
    }
}

#[derive(Deserialize, Debug, Default, Clone)]
struct FRData {
    mode_s_code: String,
    latitude: f32,
    longitude: f32,
    bearing: u32,
    altitude: i32,
    speed: u32,
    squawk_code: String,
    radar: String,
    model: String,
    registration: String,
    timestamp: u64,
    origin: String,
    destination: String,
    flight: String,
    is_on_ground: u8,
    rate_of_climb: i32,
    callsign: String,
    is_glider: u8,
    airline: String,
    #[serde(default)]
    provider: String,
}

impl AircraftData for FRData {
    fn squawk(&self) -> &str {
        &self.squawk_code
    }

    fn callsign(&self) -> &str {
        &self.callsign
    }

    fn is_on_ground(&self) -> bool {
        self.is_on_ground == 0
    }

    fn latitude(&self) -> f32 {
        self.latitude
    }

    fn longitude(&self) -> f32 {
        self.longitude
    }

    fn heading(&self) -> u32 {
        self.bearing
    }

    fn ground_speed(&self) -> u32 {
        self.speed
    }

    fn timestamp(&self) -> u64 {
        self.timestamp
    }

    fn altitude(&self) -> i32 {
        self.altitude
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn hex(&self) -> &str {
        &self.mode_s_code
    }

    fn origin(&self) -> &str {
        &self.origin
    }

    fn destination(&self) -> &str {
        &self.destination
    }
}

impl FromProvider for FRData {
    fn provider(&self) -> &str {
        "FlightRadar24"
    }
}

impl ProvidedAircraftData for FRData {}
