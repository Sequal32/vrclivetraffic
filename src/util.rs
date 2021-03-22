use std::collections::HashMap;

use lazy_static::lazy_static;
use regex::Regex;

use crate::error::Error;

lazy_static! {
    static ref AIRLINE_REGEX: Regex = Regex::new(r"[A-z]{3}\d+").unwrap();
}

pub fn convert_miles_to_lat(miles: f32) -> f32 {
    return miles / 69.0;
}

pub fn convert_miles_to_lon(miles: f32) -> f32 {
    return miles / 54.6;
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

pub trait AircraftData {
    fn squawk(&self) -> &str;
    fn callsign(&self) -> &str;
    fn is_on_ground(&self) -> bool;
    fn latitude(&self) -> f32;
    fn longitude(&self) -> f32;
    fn heading(&self) -> u32;
    fn ground_speed(&self) -> u32;
    fn timestamp(&self) -> u64;
    fn altitude(&self) -> i32;
    fn model(&self) -> &str;
    fn hex(&self) -> &str;
    fn origin(&self) -> &str {
        ""
    }
    fn destination(&self) -> &str {
        ""
    }
    fn is_airline(&self) -> bool {
        AIRLINE_REGEX.is_match(self.callsign())
    }
    fn get_airline(&self) -> Option<&str> {
        Some(AIRLINE_REGEX.captures(self.callsign())?.get(0)?.as_str())
    }
}

pub trait FromProvider {
    fn provider(&self) -> &str;
}

pub trait AircraftProvider {
    fn get_aircraft(&mut self) -> Result<AircraftMap, Error>;
    fn get_name(&self) -> &str;
}

pub trait ProvidedAircraftData: AircraftData + FromProvider {}

pub type BoxedData = Box<dyn ProvidedAircraftData>;
pub type AircraftMap = HashMap<String, BoxedData>;
