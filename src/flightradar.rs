use crate::{error::Error, util::{AircraftProvider, MinimalAircraftData}};
use crate::util::Bounds;
use attohttpc;
use serde_json::{self, Value};
use serde::Deserialize;
use std::collections::HashMap;

const ENDPOINT: &str = "https://data-live.flightradar24.com/zones/fcgi/feed.js?faa=1&mlat=1&flarm=1&adsb=1&gnd=1&air=1&vehicles=1&estimated=1&gliders=1&stats=0&maxage=14400";

#[derive(Deserialize, Debug, Default, Clone)]
pub struct AircraftData {
    pub mode_s_code: String,
    pub latitude: f32,
    pub longitude: f32,
    pub bearing: u32,
    pub altitude: i32,
    pub speed: u32,
    pub squawk_code: String,
    pub radar: String,
    pub model: String,
    pub registration: String,
    pub timestamp: u64,
    pub origin: String,
    pub destination: String,
    pub flight: String,
    is_on_ground: u8,
    pub rate_of_climb: i32,
    pub callsign: String,
    is_glider: u8,
    pub airline: String
}

pub struct FlightRadar {
    base_url: String
}

impl FlightRadar {
    pub fn new(radar_loc: &Bounds) -> Self {
        Self {
            base_url: format!("{}&bounds={:.2},{:.2},{:.2},{:.2}", 
                ENDPOINT, 
                radar_loc.lat1, 
                radar_loc.lat2, 
                radar_loc.lon1, 
                radar_loc.lon2
            )
        }
    }
}

impl AircraftProvider for FlightRadar {
    fn get_aircraft(&mut self) -> Result<HashMap<String, MinimalAircraftData>, Error> {
        let response = attohttpc::get(&self.base_url)
            .send()?
            .error_for_status()?;

        let mut return_data = HashMap::new();

        let data: Value = serde_json::from_str(&response.text()?)?;

        // Iterate through aircraft
        for (index, value) in data.as_object().unwrap() {
            // Skip over stats data like numbers and objects
            if !value.is_array() {continue}

            let data: AircraftData = serde_json::from_value(value.clone()).unwrap();

            return_data.insert(index.clone(), MinimalAircraftData {
                squawk: data.squawk_code,
                callsign: data.callsign,
                is_on_ground: data.is_on_ground == 1,
                latitude: data.latitude,
                longitude: data.longitude,
                heading: data.bearing,
                ground_speed: data.speed,
                timestamp: data.timestamp,
                altitude: data.altitude,
            });
        }

        Ok(return_data)
    }

    fn get_name(&self) -> &str {
        "FlightRadar24"
    }
}