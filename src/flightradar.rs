use std::collections::HashMap;

use reqwest::blocking;
use serde_json::{self, Value};
use serde::Deserialize;

const ENDPOINT: &str = "https://data-live.flightradar24.com/zones/fcgi/feed.js?faa=1&mlat=1&flarm=1&adsb=1&gnd=1&air=1&vehicles=1&estimated=1&gliders=1&stats=1&maxage=14400";

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
    pub is_on_ground: u8,
    pub rate_of_climb: i32,
    pub callsign: String,
    pub is_glider: u8,
    pub airline: String
}

pub struct FlightRadar {
    client: blocking::Client,
    radar_loc: Bounds
}

impl FlightRadar {
    pub fn new(radar_loc: Bounds) -> Self {
        Self {
            client: blocking::Client::new(),
            radar_loc
        }
    }

    pub fn get_aircraft(&self) -> Result<HashMap<String, AircraftData>, ()> {
        let response = self.client.get(
            format!("{}&bounds={:.2},{:.2},{:.2},{:.2}", 
                ENDPOINT, 
                self.radar_loc.lat1, 
                self.radar_loc.lat2, 
                self.radar_loc.lon1, 
                self.radar_loc.lon2
            ).as_str()
        ).send();

        let mut return_data = HashMap::new();
        
        if !response.is_ok() {return Err(())}

        if let Ok(text) = response.unwrap().text() {
            // Parse as JSON
            let data: Value = serde_json::from_str(text.as_str()).unwrap();
            // Iterate through aircraft
            for (index, value) in data.as_object().unwrap() {
                // Skip over stats data like numbers and objects
                if !value.is_array() {continue}
                let data: AircraftData = serde_json::from_value(value.clone()).unwrap();
                return_data.insert(index.clone(), data);
            }
            return Ok(return_data);
        }

        return Err(())
    }
}

pub struct Bounds {
    pub lat1: f32,
    pub lon1: f32,
    pub lat2: f32,
    pub lon2: f32,
}