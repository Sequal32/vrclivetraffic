use std::collections::HashMap;

use lazy_static::lazy_static;
use regex::Regex;

use crate::error::Error;

lazy_static! {
    static ref AIRLINE_REGEX: Regex = Regex::new(r"[A-z]{3}\d+").unwrap();
    static ref CALLSIGN_REGEX: Regex = Regex::new(r"[A-Z]{3}[A-Z0-9]{1,}").unwrap();
    static ref REGISTRATION_REGEX: Regex =
        Regex::new(r"[A-Z]-[A-Z]{4}|[A-Z]{2}-[A-Z]{3}|N[0-9]{1,5}[A-Z]{0,2}").unwrap();
}

pub fn convert_miles_to_lat(miles: f32) -> f32 {
    return miles / 69.0;
}

pub fn convert_miles_to_lon(miles: f32) -> f32 {
    return miles / 54.6;
}

pub fn is_valid_callsign(callsign: &str) -> bool {
    CALLSIGN_REGEX.is_match(callsign) || REGISTRATION_REGEX.is_match(callsign)
}

#[derive(Debug, Default, Clone)]
pub struct LatLon {
    pub lat: f32,
    pub lon: f32,
}

#[derive(Debug, Default, Clone)]
pub struct Vector2D {
    pub x: f32,
    pub y: f32,
}

impl Vector2D {
    pub fn from_heading_and_speed(heading: f32, speed: f32) -> Self {
        // Split speed into componenets
        // let angle = heading + 180 % 360;

        Self {
            x: heading.to_radians().cos() * speed,
            y: heading.to_radians().sin() * speed,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Bounds {
    pub lat1: f32,
    pub lon1: f32,
    pub lat2: f32,
    pub lon2: f32,
}

pub struct AircraftData {
    pub squawk: String,
    pub callsign: String,
    pub is_on_ground: bool,
    pub latitude: f32,
    pub longitude: f32,
    pub heading: u32,
    pub ground_speed: u32,
    pub timestamp: u64,
    pub altitude: i32,
    pub model: String,
    pub hex: String,
    pub origin: String,
    pub destination: String,
    pub provider: &'static str,
}

impl AircraftData {
    pub fn is_airline(&self) -> bool {
        AIRLINE_REGEX.is_match(&self.callsign)
    }

    pub fn get_airline(&self) -> Option<&str> {
        Some(AIRLINE_REGEX.captures(&self.callsign)?.get(0)?.as_str())
    }

    pub fn combine_with(self, rhs: Self) -> Self {
        let update_space = rhs.timestamp > self.timestamp;

        macro_rules! replace_if {
            ($condition: expr, $field: ident) => {
                if $condition {
                    self.$field
                } else {
                    rhs.$field
                }
            };
        }

        Self {
            squawk: self.squawk,
            callsign: replace_if!(is_valid_callsign(&self.callsign), callsign),
            is_on_ground: self.is_on_ground,
            latitude: replace_if!(self.latitude != 0.0 && !update_space, latitude),
            longitude: replace_if!(self.longitude != 0.0 && !update_space, longitude),
            heading: replace_if!(!update_space, heading),
            ground_speed: self.ground_speed,
            timestamp: self.timestamp,
            altitude: replace_if!(self.altitude != 0 && !update_space, altitude),
            model: replace_if!(self.model == "", model),
            hex: self.hex,
            origin: replace_if!(self.origin != "" && !update_space, origin),
            destination: replace_if!(self.destination != "" && !update_space, destination),
            provider: "Combined",
        }
    }
}

pub trait AircraftProvider {
    fn get_aircraft(&self) -> Result<AircraftMap, Error>;
    fn get_name(&self) -> &str;
}

pub type AircraftMap = HashMap<String, AircraftData>;
