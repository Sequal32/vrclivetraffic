use crate::error::Error;
use crate::util::{AircraftData, AircraftMap, AircraftProvider, Bounds};

use attohttpc;
use serde::Deserialize;
use serde_json::{self, Value};
use std::collections::HashMap;

const ENDPOINT: &str = "https://data-live.flightradar24.com/zones/fcgi/feed.js?faa=1&mlat=1&flarm=1&adsb=1&gnd=1&air=1&vehicles=1&estimated=1&gliders=1&stats=0&maxage=14400";

pub struct FlightRadar {
    base_url: String,
}

impl FlightRadar {
    pub fn new(radar_loc: &Bounds) -> Self {
        Self {
            base_url: format!(
                "{}&bounds={:.2},{:.2},{:.2},{:.2}",
                ENDPOINT, radar_loc.lat1, radar_loc.lat2, radar_loc.lon1, radar_loc.lon2
            ),
        }
    }
}

impl AircraftProvider for FlightRadar {
    fn get_aircraft(&self) -> Result<AircraftMap, Error> {
        let response = attohttpc::get(&self.base_url).send()?.error_for_status()?;

        let mut return_data = HashMap::new();

        let data: Value = serde_json::from_str(&response.text()?)?;

        // Iterate through aircraft
        for (_, value) in data.as_object().unwrap() {
            // Skip over stats data like numbers and objects
            if !value.is_array() {
                continue;
            }

            let data: FRData = serde_json::from_value(value.clone()).unwrap();

            return_data.insert(data.mode_s_code.clone(), data.into());
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
}

impl Into<AircraftData> for FRData {
    fn into(self) -> AircraftData {
        AircraftData {
            squawk: self.squawk_code,
            callsign: self.callsign,
            is_on_ground: self.is_on_ground == 1,
            latitude: self.latitude,
            longitude: self.longitude,
            heading: self.bearing,
            ground_speed: self.speed,
            timestamp: self.timestamp,
            altitude: self.altitude,
            model: self.model,
            hex: self.mode_s_code,
            origin: self.origin,
            destination: self.destination,
        }
    }
}
